use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{FrameRate, NdiSender};
use crate::Result;
use crate::domain::Slide;
use crate::media::decoder::RgbaFrame;
use crate::render::{FrameSlot, Overlay, RenderPipeline, SlidePlayer, StyleOverride};

#[derive(Clone)]
pub enum OutputCommand {
    ShowSlide(Box<Slide>),
    BlackScreen,
    Clear,
}

/// The state the sender needs for one tick.
///
/// It is copied out of the shared state so the picture can be drawn without
/// holding the lock.
struct Snapshot {
    command: OutputCommand,
    overlay: Overlay,
    style: StyleOverride,
    default_transition: crate::domain::Transition,
    reduced_motion: bool,
    revision: u64,
    slide_serial: u64,
    shown_serial: Option<u64>,
    video_frame: Option<RgbaFrame>,
}

struct OutputState {
    command: OutputCommand,
    video_frame_source: Option<Arc<Mutex<Option<RgbaFrame>>>>,
    /// The props, mask and live message composited on top of the slide.
    overlay: Overlay,
    /// The style the slide is drawn in, for a screen with its own theme.
    style: StyleOverride,
    /// The transition used when a slide does not name one of its own.
    default_transition: crate::domain::Transition,
    /// Whether the motion is reduced: the slide then changes without a
    /// transition.
    reduced_motion: bool,
    /// Bumped whenever the overlay or the style changes, so the sender knows
    /// the picture has to be drawn again.
    revision: u64,
    /// Counts the slides pushed into the output. The player compares it with
    /// the one it is showing to decide whether a new slide (and therefore a
    /// new transition) starts.
    slide_serial: u64,
    /// The serial of the slide the player is currently showing.
    shown_serial: Option<u64>,
}

pub struct NdiOutputLoop {
    state: Arc<Mutex<OutputState>>,
    /// The signal that stops the sender.
    ///
    /// It is held in an `Option` so the signal can be sent from a shared
    /// reference: the sender has to be stoppable when the loop is dropped.
    stop_tx: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    /// The resolution the sender is broadcasting at.
    width: u32,
    height: u32,
}

impl NdiOutputLoop {
    /// Start the sender.
    ///
    /// `publish` names the screen the broadcast belongs to and the slot its
    /// frames are published into, so the operator's preview can show exactly
    /// what leaves the machine.
    pub fn start(
        source_name: &str,
        width: u32,
        height: u32,
        frame_rate: FrameRate,
        publish: Option<(String, Arc<FrameSlot>)>,
    ) -> Result<Self> {
        // The frame rate comes from the settings, so it can be nonsense: a
        // zero would make the interval infinite, and the sender would spin
        // without a pause.
        let fps = (frame_rate.numerator as f32 / frame_rate.denominator.max(1) as f32).max(1.0);
        let frame_interval = Duration::from_secs_f32(1.0 / fps);

        let sender = NdiSender::new(source_name.to_string(), width, height, frame_rate)?;

        let state = Arc::new(Mutex::new(OutputState {
            command: OutputCommand::BlackScreen,
            video_frame_source: None,
            overlay: Overlay::default(),
            style: StyleOverride::default(),
            default_transition: crate::domain::Transition::Cut,
            reduced_motion: false,
            revision: 0,
            slide_serial: 0,
            shown_serial: None,
        }));

        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
        let stop_tx = Mutex::new(Some(stop_tx));

        let state_clone = Arc::clone(&state);
        tokio::spawn(async move {
            output_task(sender, state_clone, frame_interval, fps, publish, stop_rx).await;
        });

        Ok(Self {
            state,
            stop_tx,
            width,
            height,
        })
    }

    pub fn set_slide(&self, slide: Slide) {
        let mut s = self
            .state
            .lock()
            .expect("NDI state mutex poisoned — unrecoverable");
        s.slide_serial += 1;
        s.command = OutputCommand::ShowSlide(Box::new(slide));
    }

    pub fn black_screen(&self) {
        let mut s = self
            .state
            .lock()
            .expect("NDI state mutex poisoned — unrecoverable");
        s.command = OutputCommand::BlackScreen;
    }

    pub fn clear(&self) {
        let mut s = self
            .state
            .lock()
            .expect("NDI state mutex poisoned — unrecoverable");
        s.command = OutputCommand::Clear;
    }

    /// The resolution the sender is broadcasting at.
    ///
    /// The pipeline is built with it, so a changed resolution needs a new
    /// sender.
    pub fn resolution(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Stop the sender.
    pub fn stop(&self) {
        if let Ok(mut guard) = self.stop_tx.lock()
            && let Some(tx) = guard.take()
        {
            let _ = tx.send(());
        }
    }

    pub fn is_running(&self) -> bool {
        Arc::strong_count(&self.state) > 1
    }

    /// Set the overlay (props, mask, live message) that is composited on top
    /// of the slide.
    ///
    /// Setting the same overlay again is free: the revision only moves when
    /// the picture would really change, so the sender can be told the
    /// current overlay as often as the operator likes.
    pub fn set_overlay(&self, overlay: Overlay) {
        let mut state = self
            .state
            .lock()
            .expect("NDI state mutex poisoned — unrecoverable");
        if state.overlay == overlay {
            return;
        }
        state.overlay = overlay;
        state.revision += 1;
    }

    /// Set the style the slide is drawn in, for a screen with its own theme.
    pub fn set_style(&self, style: StyleOverride) {
        let mut state = self
            .state
            .lock()
            .expect("NDI state mutex poisoned — unrecoverable");
        if state.style == style {
            return;
        }
        state.style = style;
        state.revision += 1;
    }

    /// Set the transition used when a slide does not name one of its own.
    pub fn set_default_transition(&self, transition: crate::domain::Transition) {
        self.state
            .lock()
            .expect("NDI state mutex poisoned — unrecoverable")
            .default_transition = transition;
    }

    /// Reduce the motion: the slide then changes without a transition.
    pub fn set_reduced_motion(&self, reduced: bool) {
        self.state
            .lock()
            .expect("NDI state mutex poisoned — unrecoverable")
            .reduced_motion = reduced;
    }

    pub fn set_video_frame_source(&self, src: Option<Arc<Mutex<Option<RgbaFrame>>>>) {
        self.state
            .lock()
            .expect("NDI state mutex poisoned — unrecoverable")
            .video_frame_source = src;
    }
}

impl Drop for NdiOutputLoop {
    /// Stop the sender when the loop is dropped.
    ///
    /// Without this the task keeps publishing into the slot and sending
    /// video until the process dies.
    fn drop(&mut self) {
        self.stop();
    }
}

async fn output_task(
    mut sender: NdiSender,
    state: Arc<Mutex<OutputState>>,
    frame_interval: Duration,
    fps: f32,
    publish: Option<(String, Arc<FrameSlot>)>,
    mut stop_rx: tokio::sync::oneshot::Receiver<()>,
) {
    let width = sender.width();
    let height = sender.height();
    // The player owns the pipeline and the transition between slides, so the
    // broadcast shows the same motion as the operator's preview.
    let mut pipeline = RenderPipeline::new(width, height);
    let mut player = SlidePlayer::new(fps);
    // The generation of the published frames, so a surface can tell a new
    // frame from the one it already painted.
    let mut published: u64 = 0;
    // The overlay revision the picture was last drawn with, and the video
    // frame it was drawn from.
    let mut applied_revision: u64 = 0;
    let mut applied_video_pts: Option<u64> = None;

    // Whether a black picture has been drawn since the last slide: a
    // broadcast that has never shown a slide still has to send a frame, or
    // the receiver shows a lost picture instead of black.
    let mut black_drawn = false;

    loop {
        // The sender is stopped when the signal arrives *or* when the
        // sender is dropped: a dropped sender would otherwise keep
        // publishing until the process dies.
        match stop_rx.try_recv() {
            Ok(()) => break,
            Err(tokio::sync::oneshot::error::TryRecvError::Closed) => break,
            Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {}
        }

        // The state is copied out under the lock and the picture is drawn
        // outside it. A render takes a whole frame, and the operator's
        // window pushes the overlay on every tick: it would stall on the
        // lock for as long as the render takes.
        let snapshot = {
            let s = state
                .lock()
                .expect("NDI state mutex poisoned — unrecoverable");
            Snapshot {
                command: s.command.clone(),
                overlay: s.overlay.clone(),
                style: s.style.clone(),
                default_transition: s.default_transition,
                reduced_motion: s.reduced_motion,
                revision: s.revision,
                slide_serial: s.slide_serial,
                shown_serial: s.shown_serial,
                video_frame: s
                    .video_frame_source
                    .as_ref()
                    .and_then(|arc| arc.lock().ok()?.clone()),
            }
        };

        // A panic in the draw would kill the broadcast: the receiver would
        // see a lost source and the operator would have no way of knowing
        // why. The panic is caught, so the sender carries on.
        let drawn = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            match &snapshot.command {
                OutputCommand::ShowSlide(slide) => {
                    if snapshot.shown_serial != Some(snapshot.slide_serial) {
                        player.set_overlay(snapshot.overlay.clone());
                        player.set_style(snapshot.style.clone());
                        player.set_default_transition(snapshot.default_transition);
                        player.set_reduced_motion(snapshot.reduced_motion);
                        applied_revision = snapshot.revision;
                        applied_video_pts =
                            snapshot.video_frame.as_ref().map(|f| f.pts_secs.to_bits());
                        if let Err(e) =
                            player.show_slide(&mut pipeline, slide, snapshot.video_frame.as_ref())
                        {
                            log::error!("NDI render failed: {e}");
                            player.black(&pipeline);
                        }
                        // The serial is written back so the next tick knows the
                        // picture is up to date.
                        state
                            .lock()
                            .expect("NDI state mutex poisoned — unrecoverable")
                            .shown_serial = Some(snapshot.slide_serial);
                        black_drawn = false;
                    } else {
                        // The overlay, the style or the video frame changed:
                        // the picture is drawn again, but the motion is left
                        // alone.
                        let video_changed =
                            snapshot.video_frame.as_ref().map(|f| f.pts_secs.to_bits())
                                != applied_video_pts;
                        if snapshot.revision != applied_revision || video_changed {
                            player.set_overlay(snapshot.overlay.clone());
                            player.set_style(snapshot.style.clone());
                            applied_revision = snapshot.revision;
                            applied_video_pts =
                                snapshot.video_frame.as_ref().map(|f| f.pts_secs.to_bits());
                            if let Err(e) =
                                player.rerender(&mut pipeline, slide, snapshot.video_frame.as_ref())
                            {
                                log::error!("NDI render failed: {e}");
                            }
                        }
                    }
                    player.next_frame()
                }
                OutputCommand::BlackScreen | OutputCommand::Clear => {
                    if snapshot.shown_serial.is_some() || !black_drawn {
                        player.black(&pipeline);
                        black_drawn = true;
                    }
                    state
                        .lock()
                        .expect("NDI state mutex poisoned — unrecoverable")
                        .shown_serial = None;
                    player.next_frame()
                }
            }
        }));
        if drawn.is_err() {
            log::error!("NDI: the sender caught a panic; the frame was skipped");
            continue;
        }
        let frame = drawn.unwrap_or_default();

        if let Some(frame) = &frame {
            if let Err(e) = sender.send_frame(frame) {
                log::error!("NDI send error: {e}");
            }
            // The broadcast publishes its own frame, so the operator can
            // preview what the stream actually shows.
            if let Some((screen_id, slot)) = &publish
                && let Some(image) = crate::render::frame_to_image(frame)
            {
                published += 1;
                slot.publish(screen_id, published, image);
            }
        }

        tokio::time::sleep(frame_interval).await;
    }
}
