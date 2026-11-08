//! Off-thread slide compositor.
//!
//! Everything the audience can see — the dock preview, the stage display and
//! the audience output window — must show the *same* composition the NDI
//! sender and the recorder produce. They therefore all share the compositor
//! in [`crate::render`] instead of re-implementing the drawing rules.
//!
//! Compositing is expensive: the rasteriser walks every pixel of the output
//! resolution and each text layer costs a GPU rasterisation plus a readback.
//! Doing that on the GPUI main thread would stall the interface, so the
//! pipeline lives on a dedicated worker thread. The UI posts a request and
//! receives a finished frame back over a channel; a fingerprint of the source
//! content makes sure an unchanged slide is never re-composited.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use gpui_kit::RenderImage;

use crate::domain::Slide;
use crate::media::decoder::RgbaFrame;
use crate::render::RenderPipeline;

/// A request to composite one slide at a given size.
pub(crate) struct CompositorRequest {
    /// Monotonic id. A response older than the newest request is stale and is
    /// dropped by the UI.
    pub(crate) generation: u64,
    /// The screen this composition is for. Each screen gets its own
    /// composition, so two projectors can show different props.
    pub(crate) screen_id: String,
    /// The slide to draw.
    pub(crate) slide: Slide,
    /// Latest decoded video frame, when the slide shows one.
    pub(crate) video: Option<RgbaFrame>,
    /// The props, mask and live message composited on top of the slide.
    pub(crate) overlay: Overlay,
    /// The test pattern the screen shows over everything else.
    ///
    /// A test pattern belongs to the screen, not to the slide: it says the
    /// projector is alive, whatever the operator has queued.
    pub(crate) test_pattern: Option<crate::render::test_pattern::Pattern>,
    /// The style the slide is drawn in, for a screen with its own theme.
    pub(crate) style: StyleOverride,
    /// The transition used when a slide does not name one of its own.
    pub(crate) default_transition: crate::domain::Transition,
    /// Whether the motion is reduced: the slide then changes without a
    /// transition.
    pub(crate) reduced_motion: bool,
    /// The content fingerprint of the slide. The worker uses it to tell a
    /// new picture from the next frame of a running transition.
    pub(crate) fingerprint: u64,
    /// The fingerprint of the slide on its own. The worker runs a
    /// transition only when this changes, so a prop that is toggled or a
    /// video frame that arrives redraws the picture without replaying the
    /// motion.
    pub(crate) slide_fingerprint: u64,
    /// The resolution the slides were authored against.
    ///
    /// The text is sized in points against it, so the worker scales the text
    /// for an output at another resolution: the same slide has to look the
    /// same on a 720p screen as on a 1080p one.
    pub(crate) reference_width: u32,
    /// Target width in pixels.
    pub(crate) width: u32,
    /// Target height in pixels.
    pub(crate) height: u32,
}

impl CompositorRequest {
    /// Build a request; the generation is assigned when it is sent.
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_arguments)]
    fn new(
        screen_id: String,
        slide: Slide,
        video: Option<RgbaFrame>,
        overlay: Overlay,
        style: StyleOverride,
        test_pattern: Option<crate::render::test_pattern::Pattern>,
        reduced_motion: bool,
        default_transition: crate::domain::Transition,
        fingerprint: u64,
        reference_width: u32,
        width: u32,
        height: u32,
    ) -> Self {
        let slide_fingerprint = slide_fingerprint(&slide);
        Self {
            generation: 0,
            screen_id,
            slide,
            video,
            overlay,
            style,
            test_pattern,
            reduced_motion,
            default_transition,
            fingerprint,
            slide_fingerprint,
            reference_width,
            width,
            height,
        }
    }
}

/// A finished composition, ready to be displayed.
///
/// [`RenderImage`] is neither `Clone` nor `Debug`, so both are implemented
/// here by hand: the image is shared through its `Arc`, and the debug output
/// deliberately omits the pixels.
pub struct CompositorResponse {
    /// The generation of the request this answers.
    pub(crate) generation: u64,
    /// The screen the composition is for.
    pub(crate) screen_id: String,
    /// The composited frame.
    pub(crate) image: Arc<RenderImage>,
    /// How many transition frames are still to come.
    pub(crate) transition_remaining: usize,
}

impl Clone for CompositorResponse {
    fn clone(&self) -> Self {
        Self {
            generation: self.generation,
            screen_id: self.screen_id.clone(),
            image: Arc::clone(&self.image),
            transition_remaining: self.transition_remaining,
        }
    }
}

impl std::fmt::Debug for CompositorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositorResponse")
            .field("generation", &self.generation)
            .field("screen_id", &self.screen_id)
            .field("transition_remaining", &self.transition_remaining)
            .finish_non_exhaustive()
    }
}

/// The UI-side handle to the compositor worker.
///
/// Requests are fire-and-forget: the worker answers on the response channel
/// that the UI bridge reads.
pub(crate) struct CompositorHandle {
    requests: Sender<CompositorRequest>,
    /// Generation of the most recent request, so stale responses can be
    /// recognised.
    pub(crate) generation: u64,
}

impl CompositorHandle {
    /// Ask the worker to composite `slide` at the given size.
    ///
    /// Returns the generation of the request. Only one request is worth having
    /// in flight: the UI skips a new request while the previous one is still
    /// being composited, because a newer frame always supersedes an older
    /// one.
    pub(crate) fn request(&mut self, mut request: CompositorRequest) -> u64 {
        self.generation += 1;
        request.generation = self.generation;
        if self.requests.send(request).is_err() {
            // The worker thread is gone (shutdown); drop the request quietly.
            log::warn!("compositor worker is not accepting requests");
        }
        self.generation
    }
}

/// Start the compositor worker thread.
///
/// In tests the worker is inert: the GPUI test scheduler rejects any
/// activity from a non-test thread as non-determinism ("Detected activity
/// on thread openpresenter-compositor ... Your test is not deterministic"),
/// and editor tests exercise layout and state, never compositor output.
/// The production binary always runs the real worker.
pub(crate) fn spawn_for_window() -> (
    CompositorHandle,
    tokio::sync::mpsc::UnboundedReceiver<CompositorResponse>,
) {
    #[cfg(test)]
    {
        let (request_tx, _request_rx) = channel::<CompositorRequest>();
        let (response_tx, response_rx) =
            tokio::sync::mpsc::unbounded_channel::<CompositorResponse>();
        std::mem::forget(response_tx);
        (
            CompositorHandle {
                requests: request_tx,
                generation: 0,
            },
            response_rx,
        )
    }
    #[cfg(not(test))]
    {
        spawn()
    }
}

/// Start the compositor worker thread.
///
/// Returns the UI-side request handle and the response receiver that the UI
/// bridge awaits on.
pub(crate) fn spawn() -> (
    CompositorHandle,
    tokio::sync::mpsc::UnboundedReceiver<CompositorResponse>,
) {
    let (request_tx, request_rx) = channel::<CompositorRequest>();
    let (response_tx, response_rx) = tokio::sync::mpsc::unbounded_channel::<CompositorResponse>();

    let spawned = std::thread::Builder::new()
        .name("openpresenter-compositor".to_string())
        .spawn(move || worker_loop(request_rx, response_tx));
    if spawned.is_err() {
        log::error!("failed to spawn the compositor worker thread");
    }

    (
        CompositorHandle {
            requests: request_tx,
            generation: 0,
        },
        response_rx,
    )
}

/// The worker loop: composite every request that arrives.
fn worker_loop(
    requests: Receiver<CompositorRequest>,
    responses: tokio::sync::mpsc::UnboundedSender<CompositorResponse>,
) {
    // One pipeline per output resolution. The screens that run at the same
    // resolution share a pipeline, and the pipelines share the GPU text
    // renderer, so a projector at 720p and a projector at 1080p still cost
    // one wgpu device between them.
    let mut pipelines: HashMap<(u32, u32), RenderPipeline> = HashMap::new();
    let mut shared_gpu_text: Option<crate::render::SharedGpuText> = None;
    // One player per screen, so each screen runs its own transition and shows
    // its own composition.
    let mut screens: HashMap<String, ScreenPlayer> = HashMap::new();

    while let Ok(request) = requests.recv() {
        if responses.is_closed() {
            // The UI is gone; stop working.
            return;
        }
        // A panic in the worker would kill the output: the screen would
        // freeze on the last frame and nobody would know why. The panic is
        // caught, so the worker carries on with the next request.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            handle_request(
                &mut pipelines,
                &mut screens,
                &mut shared_gpu_text,
                request,
                &responses,
            );
        }));
        if outcome.is_err() {
            log::error!("compositor: the worker caught a panic; the frame was skipped");
        }
    }
}

/// Compose one screen's frame and publish it.
///
/// The work happens outside the worker's loop so a panic in it can be
/// caught: the output has to survive one bad frame.
fn handle_request(
    pipelines: &mut HashMap<(u32, u32), RenderPipeline>,
    screens: &mut HashMap<String, ScreenPlayer>,
    shared_gpu_text: &mut Option<crate::render::SharedGpuText>,
    request: CompositorRequest,
    responses: &tokio::sync::mpsc::UnboundedSender<CompositorResponse>,
) {
    {
        let target = (request.width.max(1), request.height.max(1));
        if !pipelines.contains_key(&target) {
            if shared_gpu_text.is_none() {
                *shared_gpu_text = match crate::render::create_gpu_text_renderer() {
                    Ok(renderer) => Some(crate::render::SharedGpuText::new(std::sync::Mutex::new(
                        renderer,
                    ))),
                    Err(e) => {
                        log::warn!("GPU text renderer unavailable, using pixel font: {e}");
                        None
                    }
                };
            }
            let pipeline =
                RenderPipeline::with_gpu_text(shared_gpu_text.clone(), target.0, target.1);
            pipelines.entry(target).or_insert(pipeline);
        }

        let Some(pipeline) = pipelines.get_mut(&target) else {
            return;
        };
        // The resolution the slides were authored against follows the
        // request: a pipeline that kept the old one would scale the text by
        // a stale factor on a screen at another resolution.
        pipeline.set_reference_width(request.reference_width);

        // A screen whose resolution changed cannot keep the frames it holds:
        // they are the old size. The player is dropped, so the screen starts
        // again with the new size.
        if let Some(screen) = screens.get(&request.screen_id)
            && screen.size != target
        {
            screens.remove(&request.screen_id);
        }

        let screen = screens
            .entry(request.screen_id.clone())
            .or_insert_with(|| ScreenPlayer::new(TRANSITION_FPS, target));

        // The overlay and the style always follow the request. Whether the
        // picture is drawn with a transition depends on whether the slide
        // itself changed: a prop that is toggled or a video frame that
        // arrives redraws the picture but leaves the motion alone, so the
        // projector and the broadcast show the same thing.
        screen.player.set_overlay(request.overlay.clone());
        screen.player.set_style(request.style.clone());
        screen.player.set_reduced_motion(request.reduced_motion);
        screen
            .player
            .set_default_transition(request.default_transition);

        let new_slide = request.slide_fingerprint != screen.slide_fingerprint;
        if new_slide {
            // A new slide: render it and start its transition.
            if let Err(e) =
                screen
                    .player
                    .show_slide(pipeline, &request.slide, request.video.as_ref())
            {
                log::error!("compositor: failed to render the slide: {e}");
                screen.player.black(pipeline);
            }
        } else if request.fingerprint != screen.fingerprint {
            // The same slide, a different picture: the overlay, the style or
            // the video frame changed. The picture is replaced, the motion
            // is left alone.
            if let Err(e) = screen
                .player
                .rerender(pipeline, &request.slide, request.video.as_ref())
            {
                log::error!("compositor: failed to redraw the slide: {e}");
            }
        }
        screen.fingerprint = request.fingerprint;
        screen.slide_fingerprint = request.slide_fingerprint;

        let Some(mut frame) = screen.player.next_frame() else {
            return;
        };
        // The test pattern goes over everything: it is the screen's own
        // picture, so it is drawn last and it covers the slide.
        if let Some(pattern) = request.test_pattern {
            crate::render::test_pattern::draw(pattern, &mut frame);
        }
        let transition_remaining = if screen.player.is_transitioning() {
            // More transition frames are still to come.
            1
        } else {
            0
        };

        let response = match crate::render::frame_to_image(&frame) {
            Some(image) => CompositorResponse {
                generation: request.generation,
                screen_id: request.screen_id.clone(),
                image,
                transition_remaining,
            },
            None => {
                log::error!("compositor: could not convert the frame to an image");
                return;
            }
        };
        if responses.send(response).is_err() {
            // The UI is gone; stop working.
            log::debug!("compositor: the window is gone; the worker stops");
        }
    }
}

/// One screen's player and the slide it is showing.
struct ScreenPlayer {
    player: SlidePlayer,
    /// The resolution the player's frames are drawn at.
    size: (u32, u32),
    /// The fingerprint of the picture the player is showing. A request with
    /// the same fingerprint is the next frame of a running transition, not a
    /// new picture.
    fingerprint: u64,
    /// The fingerprint of the **slide** the player is showing.
    ///
    /// The slide and the picture are two different things: a prop that is
    /// toggled or a video frame that arrives changes the picture without
    /// changing the slide. Only a new slide runs a transition; a new
    /// picture is drawn in place, the way the broadcast draws it.
    slide_fingerprint: u64,
}

impl ScreenPlayer {
    fn new(fps: f32, size: (u32, u32)) -> Self {
        Self {
            player: SlidePlayer::new(fps),
            size,
            fingerprint: 0,
            slide_fingerprint: 0,
        }
    }
}

/// The frame rate the transition durations are measured against.
const TRANSITION_FPS: f32 = 30.0;

/// Fingerprint the content a composition is built from.
///
/// The UI compares this against the current slide before asking the worker
/// for a new frame, so an unchanged slide is never re-composited. The video
/// fingerprint is folded in so a playing video keeps the preview moving.
pub(crate) fn content_fingerprint(
    slide: &Slide,
    video: Option<u64>,
    overlay: u64,
    reference_width: u32,
    width: u32,
    height: u32,
) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mut mix = |bytes: &[u8]| {
        for b in bytes {
            hash ^= u64::from(*b);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    if let Ok(json) = serde_json::to_vec(slide) {
        mix(&json);
    }
    mix(&video.unwrap_or_default().to_le_bytes());
    mix(&overlay.to_le_bytes());
    // The size is part of the content: a different resolution has to be
    // composed again.
    mix(&width.to_le_bytes());
    mix(&height.to_le_bytes());
    // The resolution the slides were authored against is part of it too:
    // the text is scaled by it, so a screen at 720p has to be composed again
    // when the authoring resolution changes.
    mix(&reference_width.to_le_bytes());
    hash
}

/// Fingerprint the slide's identity.
///
/// The slide decides whether a transition runs, so this has to answer "is
/// this the same slide?". The slide's id is that answer: the content
/// changes while the operator edits the slide, and that must redraw the
/// picture without replaying the motion. Folding the whole slide in here
/// would restart the transition on every keystroke.
pub(crate) fn slide_fingerprint(slide: &Slide) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for b in slide.id.as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

// ---------------------------------------------------------------------------
// UI side
// ---------------------------------------------------------------------------

use crate::render::{Overlay, SlidePlayer, StyleOverride};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::ViewMode;
use crate::ui::output::AUDIENCE_SCREEN_ID;
use crate::ui::stage::STAGE_SCREEN_ID;
use gpui_kit::{Context, img};

/// Ask the worker for a new composition when the on-screen content changed.
///
/// Called from the periodic compositor tick. Every screen that shows the live
/// slide gets its own composition, so two projectors can show different
/// props. Only one request per screen is ever in flight: a newer frame
/// always supersedes an older one, so queueing a second request while the
/// first is running would only waste GPU time.
pub(crate) fn request_if_changed(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    // The broadcast composites the slide itself, so it keeps its own copy of
    // the overlay. Telling it the current one on every tick keeps the
    // broadcast and the projector showing the same picture, whatever changed
    // the overlay — and a repeated push costs nothing, because the sender
    // only redraws when the overlay really differs.
    crate::ui::props::push_overlay(w);

    // The broadcast publishes its own frames; the operator's preview has to
    // repaint when a new one arrives.
    let broadcast = crate::ui::props::ndi_screen_id(w);
    if let Some((generation, _)) = w.compositor.slot.latest(&broadcast)
        && generation != w.compositor.broadcast_generation
    {
        w.compositor.broadcast_generation = generation;
        cx.notify();
    }

    // A black screen hides the slide entirely, so there is nothing to
    // compose; the surfaces paint their own black panel.
    if w.output.black_screen {
        // Remember that the screens are blacked out, so un-blacking replaces
        // the frame instead of leaving the output black.
        for state in w.compositor.screens.values_mut() {
            state.dirty = true;
        }
        if !w.compositor.slot.is_empty() {
            w.compositor.slot.clear_all();
            cx.notify();
        }
        return;
    }

    let Some(slide) = current_slide(w) else {
        return;
    };

    // While a canvas drag is in progress the slide changes on every pointer
    // move. The edit canvas already shows the change live, so re-composing
    // the full-resolution frame for each move would only burn the GPU; the
    // composition catches up when the drag ends.
    if w.canvas.drag.is_some() {
        return;
    }

    // The decoded frame is a multi-megabyte buffer, so its identity is read
    // first and the pixels are only copied when a screen really needs them.
    // A paused video keeps its last frame: the operator pauses to talk about
    // the picture, so the picture has to stay on the output.
    let player = w
        .video
        .player
        .as_ref()
        .filter(|_| slide_needs_video(&slide));
    let video_fingerprint = player.and_then(|p| p.current_frame_identity());
    let mut video: Option<RgbaFrame> = None;
    let mut video_copied = false;

    // While recording, the slide is re-composed on every tick even when it did
    // not change: a recording needs a steady frame rate, and the encoder
    // timestamps follow the wall clock.
    let recording = w.recording.manager.state == crate::recording::RecordingState::Recording;

    let reduced_motion = w.ui.reduce_motion;
    let default_transition = w.presenting.global_transition;
    // The resolution the slides were authored against: the text is sized
    // against it, so an output at another resolution scales its text.
    let reference_width = w.app_config.output.width;
    let screens = composite_screens(w);

    for screen_id in screens {
        // One request at a time per screen. If the worker never answers, the
        // request is retried after a second.
        let outstanding = w.compositor.screens.get(&screen_id);
        if outstanding.is_some_and(|state| state.in_flight) {
            let stuck = outstanding
                .and_then(|state| state.requested_at)
                .is_some_and(|at| at.elapsed() > std::time::Duration::from_secs(1));
            if !stuck {
                continue;
            }
        }

        // Every screen is composed at its own resolution, so a projector can
        // run at 1080p while a second monitor runs at 720p.
        let (width, height) = screen_resolution(w, &screen_id);

        // Each screen has its own configuration: the projector can show the
        // lower thirds while another screen shows only the slide.
        let overlay = crate::ui::props::active_overlay_for_screen(w, &screen_id);
        let style = crate::ui::props::style_for_screen(w, &screen_id);
        let fingerprint = content_fingerprint(
            &slide,
            video_fingerprint,
            crate::ui::props::overlay_fingerprint(w, &screen_id),
            reference_width,
            width,
            height,
        );

        // The pattern is read before the screens are borrowed: the screens
        // are held mutably while the request is built.
        let test_pattern = crate::ui::test_pattern::pattern_for_screen(w, &screen_id);
        let state = w.compositor.screens.entry(screen_id.clone()).or_default();

        // A running transition needs a frame per tick as well.
        let transitioning = state.transition_remaining > 0;
        if fingerprint == state.fingerprint && !recording && !transitioning && !state.dirty {
            continue;
        }
        state.fingerprint = fingerprint;
        state.dirty = false;

        let Some(handle) = w.compositor.handle.as_mut() else {
            continue;
        };
        // The pixels are copied now, for the first screen that needs them.
        if !video_copied {
            video = player.and_then(|p| p.current_frame());
            video_copied = true;
        }
        let generation = handle.request(CompositorRequest::new(
            screen_id.clone(),
            slide.clone(),
            video.clone(),
            overlay,
            style,
            test_pattern,
            reduced_motion,
            default_transition,
            fingerprint,
            reference_width,
            width,
            height,
        )); // The generation is per screen: the screens are served in turn, so a
        // global counter would drop every frame but the last one.
        state.generation = generation;
        state.in_flight = true;
        state.requested_at = Some(std::time::Instant::now());
    }
}

/// Whether a screen is one of the stage screens.
///
/// The stage is the speaker's own screen, so a pattern the operator puts on
/// the stage is for the speaker and not for the audience.
pub(crate) fn is_stage_screen(screen_id: &str) -> bool {
    screen_id == STAGE_SCREEN_ID
}

/// Ask every screen for a new frame.
///
/// The test pattern is drawn over the picture, so a screen showing one has
/// to be drawn again: the picture it holds is the one from before the
/// pattern was turned on.
pub(crate) fn invalidate_all(w: &mut MainWindow) {
    for state in w.compositor.screens.values_mut() {
        state.dirty = true;
    }
    w.compositor.broadcast_generation = 0;
}

/// The resolution a screen is composed at.
///
/// Every output can have a resolution of its own, so a projector can run at
/// 1080p while a second monitor runs at 720p. The audience screen follows
/// the operator's global output resolution; the other outputs carry their
/// own.
pub(crate) fn screen_resolution(w: &MainWindow, screen_id: &str) -> (u32, u32) {
    let global = (w.app_config.output.width, w.app_config.output.height);
    // The audience screen is the projector, and the recorder and the
    // broadcast follow it, so it always renders at the operator's global
    // output resolution. The other outputs carry a resolution of their own.
    if screen_id == AUDIENCE_SCREEN_ID || screen_id == "main" {
        return global;
    }
    w.output
        .manager
        .get(screen_id)
        .map(|output| (output.width, output.height))
        .filter(|(width, height)| *width > 0 && *height > 0)
        .unwrap_or(global)
}

/// The screens that need their own composition of the live slide.
///
/// The audience screen always does: it is the projector. Any other active
/// window output that routes the live slide gets one too, so a second
/// projector can show a different look.
fn composite_screens(w: &MainWindow) -> Vec<String> {
    let mut screens = vec![AUDIENCE_SCREEN_ID.to_string()];
    // The stage monitor shows the live slide too, and it has its own
    // composition when it is open — or when the operator is previewing it
    // in the dock, which otherwise showed a black box.
    // A monitor of any kind needs the stage screen's composition: a screen
    // routed to the stage (or mirroring it) opens a monitor of its own, and
    // it would show an empty picture without it.
    if crate::ui::stage::stage_monitor_is_open(w)
        || w.presenting.preview_screen_target == STAGE_SCREEN_ID
    {
        screens.push(STAGE_SCREEN_ID.to_string());
    }
    let routes = w
        .output
        .manager
        .iter()
        .map(|output| (output.id.clone(), output.content.clone()))
        .collect::<HashMap<_, _>>();

    for output in w.output.manager.iter() {
        if output.id == AUDIENCE_SCREEN_ID || !output.active {
            continue;
        }
        if !matches!(output.output_type, crate::output::OutputType::Window) {
            continue;
        }
        // A mirror shows the picture of its source, so it needs no
        // composition of its own — the window reads the source's frame.
        if matches!(
            output.content,
            crate::output::OutputContentRoute::Mirror { .. }
        ) {
            continue;
        }
        if output.content.resolve(&routes) == crate::output::ScreenKind::Live {
            screens.push(output.id.clone());
        }
    }
    screens
}

/// Whether a composed frame belongs to the recording.
///
/// The recorder captures the audience screen. The stage monitor and a second
/// projector have compositions of their own, and they can carry a different
/// look; feeding those frames to the encoder would make the recording
/// flicker between the screens instead of showing the output.
fn feeds_recording(screen_id: &str) -> bool {
    screen_id == AUDIENCE_SCREEN_ID
}

/// Take a finished composition from the worker.
///
/// Responses older than the newest request are stale (a newer slide was
/// requested in the meantime) and are dropped.
pub(crate) fn apply_frame(
    w: &mut MainWindow,
    response: CompositorResponse,
    cx: &mut Context<MainWindow>,
) {
    let state = w
        .compositor
        .screens
        .entry(response.screen_id.clone())
        .or_default();
    // A response older than this screen's newest request is stale: a newer
    // frame superseded it.
    if response.generation < state.generation {
        return;
    }
    state.in_flight = false;
    state.requested_at = None;
    state.transition_remaining = response.transition_remaining;

    // A running recording consumes the same composed frame the audience
    // sees: the pipeline already writes BGRA, which is what the encoder
    // wants. Only the audience screen's frame is recorded — the stage
    // monitor and a second projector have compositions of their own, and
    // feeding those to the encoder would make the recording flicker
    // between the screens' different looks.
    if w.recording.manager.state == crate::recording::RecordingState::Recording
        && feeds_recording(&response.screen_id)
    {
        // The encoder was started at a fixed size. A resolution change while
        // recording would otherwise hand it frames of another size, and the
        // picture would be garbled, so such a frame is left out.
        let size = response.image.size(0);
        let fits = size.width.0 as u32 == w.recording.manager.frame_width
            && size.height.0 as u32 == w.recording.manager.frame_height;
        if fits && let Some(bytes) = response.image.as_bytes(0) {
            w.recording.manager.submit_frame(
                bytes.to_vec(),
                w.recording.manager.frame_width,
                w.recording.manager.frame_height,
            );
        } else if !w.recording.manager.resolution_changed {
            // Tell the operator once: the recording keeps the size it
            // started with.
            w.recording.manager.resolution_changed = true;
            log::warn!(
                "[recording] the output resolution changed while recording — the recording keeps {}×{}",
                w.recording.manager.frame_width,
                w.recording.manager.frame_height
            );
        }
    }

    w.compositor
        .slot
        .publish(&response.screen_id, response.generation, response.image);
    // The stage monitor shows the live frame, so it needs the new one.
    crate::ui::stage::publish_stage_if_open(w);
    cx.notify();
}

/// The slide that is currently on screen.
///
/// In Edit mode that is the slide being edited; in Show mode it is the live
/// slide of the open presentation.
fn current_slide(w: &MainWindow) -> Option<Slide> {
    match w.shell.current_mode {
        ViewMode::Edit => w.get_current_slide().cloned(),
        ViewMode::Show => {
            let index = w.presenting.slide_index;
            w.presenting
                .presentation
                .as_ref()
                .and_then(|p| p.slides.get(index))
                .cloned()
        }
    }
}

/// Whether the slide shows a video and therefore needs the decoded frame.
fn slide_needs_video(slide: &Slide) -> bool {
    slide.video_path().is_some()
}

/// The latest composited frame as an image element.
///
/// Returns `None` when nothing has been composed yet, so callers can fall
/// back to their own empty state. The caller owns the sizing and the object
/// fit.
pub(crate) fn frame_image(w: &MainWindow) -> Option<gpui_kit::Img> {
    frame_for_screen(w, AUDIENCE_SCREEN_ID)
}

/// The composited frame of a screen, as a GPUI image.
///
/// Each screen has its own composition, so the operator can look at the
/// projector, the broadcast stream or the stage monitor.
pub(crate) fn frame_for_screen(w: &MainWindow, screen_id: &str) -> Option<gpui_kit::Img> {
    let (_, image) = w.compositor.slot.latest(screen_id)?;
    Some(img(image))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::Frame;

    #[test]
    fn frame_to_image_keeps_the_pixel_bytes() {
        let mut frame = Frame::new(2, 1);
        frame.set_pixel(0, 0, 10, 20, 30, 255);
        frame.set_pixel(1, 0, 40, 50, 60, 255);

        let image = crate::render::frame_to_image(&frame).expect("a 2x1 frame converts");
        let bytes = image.as_bytes(0).expect("the image carries one frame");
        // GPUI stores BGRA; the render frame already writes [b, g, r, a].
        assert_eq!(&bytes[0..4], &[30, 20, 10, 255]);
        assert_eq!(&bytes[4..8], &[60, 50, 40, 255]);
    }

    #[test]
    fn fingerprint_changes_with_the_slide() {
        let a = Slide::new_text("One".to_string());
        let b = Slide::new_text("Two".to_string());
        assert_ne!(
            content_fingerprint(&a, None, 0, 1920, 1920, 1080),
            content_fingerprint(&b, None, 0, 1920, 1920, 1080)
        );
        assert_eq!(
            content_fingerprint(&a, None, 0, 1920, 1920, 1080),
            content_fingerprint(&a, None, 0, 1920, 1920, 1080)
        );
    }

    #[test]
    fn fingerprint_includes_the_resolution() {
        let slide = Slide::new_text("Same".to_string());
        assert_ne!(
            content_fingerprint(&slide, None, 0, 1920, 1920, 1080),
            content_fingerprint(&slide, None, 0, 1920, 1280, 720)
        );
    }

    #[test]
    fn fingerprint_includes_the_reference_width() {
        // The text is scaled by the resolution the slides were authored
        // against, so a screen has to be composed again when it changes.
        let slide = Slide::new_text("Same".to_string());
        assert_ne!(
            content_fingerprint(&slide, None, 0, 1920, 1920, 1080),
            content_fingerprint(&slide, None, 0, 1280, 1920, 1080)
        );
    }

    #[test]
    fn fingerprint_includes_the_overlay() {
        let slide = Slide::new_text("Same".to_string());
        assert_ne!(
            content_fingerprint(&slide, None, 0, 1920, 1920, 1080),
            content_fingerprint(&slide, None, 1, 1920, 1920, 1080)
        );
    }

    #[test]
    fn fingerprint_includes_the_video_frame() {
        let slide = Slide::new_text("Same".to_string());
        assert_ne!(
            content_fingerprint(&slide, None, 0, 1920, 1920, 1080),
            content_fingerprint(&slide, Some(1), 0, 1920, 1920, 1080)
        );
    }

    /// The recorder has to capture the audience screen only. The stage
    /// monitor and a second projector compose their own frames, and those
    /// would make the recording flicker between the screens' looks.
    #[test]
    fn only_the_audience_screen_feeds_the_recording() {
        assert!(feeds_recording(AUDIENCE_SCREEN_ID));
        assert!(!feeds_recording(STAGE_SCREEN_ID));
        assert!(!feeds_recording("projector_b"));
    }

    #[test]
    fn two_screens_get_their_own_composition() {
        use crate::domain::{Background, Color, Object, ObjectContent, ShapeType, SlideContent};

        /// A slide filled with one colour.
        fn slide(color: [u8; 3]) -> Slide {
            let mut shape = Object::new_shape(ShapeType::Rectangle);
            if let ObjectContent::Shape { fill, .. } = &mut shape.content {
                *fill = Color {
                    r: color[0],
                    g: color[1],
                    b: color[2],
                    a: 255,
                };
            }
            shape.width = 1.0;
            shape.height = 1.0;

            Slide {
                id: "slide".to_string(),
                content: SlideContent::Text {
                    text: "Slide".to_string(),
                    style: Default::default(),
                },
                background: Background::Solid(Color {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255,
                }),
                transition: Default::default(),
                group: None,
                notes: None,
                layers: vec![shape],
                cues: Vec::new(),
            }
        }

        let (mut handle, mut responses) = spawn();

        // The projector shows the slide.
        handle.request(CompositorRequest::new(
            String::from("projector"),
            slide([255, 0, 0]),
            None,
            Overlay::default(),
            StyleOverride::default(),
            None,
            false,
            crate::domain::Transition::Cut,
            1,
            32,
            32,
            32,
        ));
        // The second screen shows the same slide with the slide layer off.
        handle.request(CompositorRequest::new(
            String::from("second"),
            slide([255, 0, 0]),
            None,
            Overlay {
                show_slide: false,
                ..Overlay::default()
            },
            StyleOverride::default(),
            None,
            false,
            crate::domain::Transition::Cut,
            1,
            32,
            32,
            32,
        ));

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        // The screen id and the colour of the slide's middle pixel.
        let mut seen: Vec<(String, (u8, u8, u8))> = Vec::new();
        while seen.len() < 2 {
            let response = loop {
                match responses.try_recv() {
                    Ok(response) => break response,
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                        assert!(
                            std::time::Instant::now() < deadline,
                            "the compositor worker did not answer in time"
                        );
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(e) => panic!("the response channel failed: {e}"),
                }
            };
            let bytes = response.image.as_bytes(0).expect("the frame has pixels");
            let middle = &bytes[(16 * 32 + 16) * 4..(16 * 32 + 16) * 4 + 4];
            seen.push((
                response.screen_id.clone(),
                (middle[2], middle[1], middle[0]),
            ));
        }

        // The projector shows the red slide; the second screen is black,
        // because its slide layer is switched off.
        let projector = seen
            .iter()
            .find(|(id, _)| id == "projector")
            .expect("the projector's frame");
        assert_eq!(projector.1.0, 255);

        let second = seen
            .iter()
            .find(|(id, _)| id == "second")
            .expect("the second screen's frame");
        assert_eq!(second.1.0, 0, "the slide layer is off");
    }

    /// Two screens can run at different resolutions: a projector at 1080p
    /// and a second monitor at 720p each get a frame of their own size.
    #[test]
    fn two_screens_can_run_at_different_resolutions() {
        let (mut handle, mut responses) = spawn();

        handle.request(CompositorRequest::new(
            String::from("big"),
            slide([255, 0, 0], crate::domain::Transition::Cut),
            None,
            Overlay::default(),
            StyleOverride::default(),
            None,
            false,
            crate::domain::Transition::Cut,
            1,
            32,
            32,
            16,
        ));
        handle.request(CompositorRequest::new(
            String::from("small"),
            slide([255, 0, 0], crate::domain::Transition::Cut),
            None,
            Overlay::default(),
            StyleOverride::default(),
            None,
            false,
            crate::domain::Transition::Cut,
            1,
            16,
            16,
            8,
        ));

        let mut sizes: Vec<(String, usize)> = Vec::new();
        while sizes.len() < 2 {
            let response = next_response(&mut responses);
            let bytes = response.image.as_bytes(0).expect("the frame has pixels");
            sizes.push((response.screen_id.clone(), bytes.len()));
        }

        let big = sizes
            .iter()
            .find(|(id, _)| id == "big")
            .expect("the big screen's frame");
        assert_eq!(
            big.1,
            32 * 16 * 4,
            "the big screen is composed at its own size"
        );

        let small = sizes
            .iter()
            .find(|(id, _)| id == "small")
            .expect("the small screen's frame");
        assert_eq!(
            small.1,
            16 * 8 * 4,
            "the small screen is composed at its own size"
        );
    }

    /// A slide filled with one colour and the given transition.
    ///
    /// The id names the slide, so two slides with different colours are two
    /// different slides — the way the real ones are.
    fn slide(color: [u8; 3], transition: crate::domain::Transition) -> Slide {
        use crate::domain::{Background, Color, Object, ObjectContent, ShapeType, SlideContent};

        let mut shape = Object::new_shape(ShapeType::Rectangle);
        if let ObjectContent::Shape { fill, .. } = &mut shape.content {
            *fill = Color {
                r: color[0],
                g: color[1],
                b: color[2],
                a: 255,
            };
        }
        shape.width = 1.0;
        shape.height = 1.0;

        Slide {
            id: format!("slide-{}-{}", color[0], color[1]),
            content: SlideContent::Text {
                text: "Slide".to_string(),
                style: Default::default(),
            },
            background: Background::Solid(Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            }),
            transition,
            group: None,
            notes: None,
            layers: vec![shape],
            cues: Vec::new(),
        }
    }

    /// Wait for the worker's next response.
    fn next_response(
        responses: &mut tokio::sync::mpsc::UnboundedReceiver<CompositorResponse>,
    ) -> CompositorResponse {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            match responses.try_recv() {
                Ok(response) => return response,
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "the compositor worker did not answer in time"
                    );
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(e) => panic!("the response channel failed: {e}"),
            }
        }
    }

    #[test]
    fn the_worker_serves_a_transition_frame_by_frame() {
        use crate::domain::Transition;

        let (mut handle, mut responses) = spawn();

        // The first slide is shown without a transition.
        handle.request(CompositorRequest::new(
            String::from("audience"),
            slide([255, 0, 0], Transition::Cut),
            None,
            Overlay::default(),
            StyleOverride::default(),
            None,
            false,
            crate::domain::Transition::Cut,
            1,
            32,
            32,
            32,
        ));
        let first = next_response(&mut responses);
        assert_eq!(first.transition_remaining, 0);
        let bytes = first.image.as_bytes(0).expect("the frame has pixels");
        assert_eq!(
            &bytes[0..4],
            &[0, 0, 255, 255],
            "the red slide is on screen"
        );

        // The next slide fades in: the worker serves one frame per request
        // until the transition is over.
        handle.request(CompositorRequest::new(
            String::from("audience"),
            slide([0, 0, 255], Transition::Fade { duration_ms: 300 }),
            None,
            Overlay::default(),
            StyleOverride::default(),
            None,
            false,
            crate::domain::Transition::Cut,
            2,
            32,
            32,
            32,
        ));
        let mut frames = 1;
        let mut response = next_response(&mut responses);
        while response.transition_remaining > 0 {
            // The same slide again asks for the next transition frame.
            handle.request(CompositorRequest::new(
                String::from("audience"),
                slide([0, 0, 255], Transition::Fade { duration_ms: 300 }),
                None,
                Overlay::default(),
                StyleOverride::default(),
                None,
                false,
                crate::domain::Transition::Cut,
                2,
                32,
                32,
                32,
            ));
            response = next_response(&mut responses);
            frames += 1;
        }
        // 300 ms at 30 fps is nine transition frames.
        assert_eq!(frames, 9);
    }

    /// Toggling a prop changes the picture but not the slide. The picture
    /// has to be redrawn in place: a transition here would fade the slide
    /// onto itself, and the broadcast — which redraws in place — would cut
    /// instead, so the two surfaces would disagree.
    #[test]
    fn an_overlay_change_redraws_the_slide_without_a_transition() {
        use crate::domain::Prop;

        let (mut handle, mut responses) = spawn();
        let fade = crate::domain::Transition::Fade { duration_ms: 300 };

        // The slide goes on screen.
        handle.request(CompositorRequest::new(
            String::from("audience"),
            slide([255, 0, 0], fade),
            None,
            Overlay::default(),
            StyleOverride::default(),
            None,
            false,
            crate::domain::Transition::Cut,
            1,
            32,
            32,
            32,
        ));
        let first = next_response(&mut responses);
        assert_eq!(
            first.transition_remaining, 0,
            "the first slide is not a transition"
        );

        // A prop is toggled: the same slide, a different picture.
        let prop = Prop {
            id: "prop".to_string(),
            name: "prop".to_string(),
            content: crate::domain::PropContent::Rectangle {
                color: [1.0, 1.0, 1.0, 1.0],
                corner_radius: 0.0,
            },
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            visible: true,
        };
        handle.request(CompositorRequest::new(
            String::from("audience"),
            slide([255, 0, 0], fade),
            None,
            Overlay {
                props: vec![prop],
                ..Overlay::default()
            },
            StyleOverride::default(),
            None,
            false,
            crate::domain::Transition::Cut,
            2,
            32,
            32,
            32,
        ));
        let second = next_response(&mut responses);
        assert_eq!(
            second.transition_remaining, 0,
            "an overlay change must not replay the slide's transition"
        );
    }

    #[test]
    fn the_worker_answers_a_request_with_a_composed_frame() {
        use crate::domain::{Background, Color, Object, ObjectContent, ShapeType, SlideContent};

        let mut shape = Object::new_shape(ShapeType::Rectangle);
        if let ObjectContent::Shape { fill, .. } = &mut shape.content {
            *fill = Color {
                r: 10,
                g: 20,
                b: 30,
                a: 255,
            };
        }
        shape.width = 1.0;
        shape.height = 1.0;

        let slide = Slide {
            id: "slide".to_string(),
            content: SlideContent::Text {
                text: "Hello".to_string(),
                style: Default::default(),
            },
            background: Background::Solid(Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            }),
            transition: Default::default(),
            group: None,
            notes: None,
            layers: vec![shape],
            cues: Vec::new(),
        };

        let (mut handle, mut responses) = spawn();
        let generation = handle.request(CompositorRequest::new(
            String::from("audience"),
            slide,
            None,
            Overlay::default(),
            StyleOverride::default(),
            None,
            false,
            crate::domain::Transition::Cut,
            1,
            32,
            32,
            32,
        ));
        assert_eq!(generation, 1);

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);

        let response = loop {
            match responses.try_recv() {
                Ok(response) => break response,
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "the compositor worker did not answer in time"
                    );
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(e) => panic!("the response channel failed: {e}"),
            }
        };
        assert_eq!(response.generation, 1);

        // The frame carries the rectangle that fills the slide.
        let bytes = response.image.as_bytes(0).expect("the frame has pixels");
        assert_eq!(bytes.len(), 32 * 32 * 4);
        // The render pipeline writes BGRA, so the blue fill lands in the
        // first channel.
        assert_eq!(&bytes[0..4], &[30, 20, 10, 255]);
    }
}
