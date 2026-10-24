//! A slide player: the transition between slides.
//!
//! Both the audience output (the compositor worker) and the NDI sender need
//! the same behaviour: show a slide, run its transition from the previous
//! one, and keep sending frames. Sharing one player type is what guarantees
//! that the operator's preview and the broadcast show the same motion —
//! a preview that cuts while the broadcast fades is worse than no preview.
//!
//! The player does not own the pipeline: several screens can share one, which
//! keeps a single wgpu device alive for the whole app.

use crate::domain::{Slide, Transition};
use crate::media::decoder::RgbaFrame;
use crate::render::{Frame, Overlay, RenderPipeline, StyleOverride, transition_frame_at};

/// Plays slides with their transitions.
pub struct SlidePlayer {
    /// The frame currently on screen.
    current: Option<Frame>,
    /// The transition that is running, if any.
    running: Option<RunningTransition>,
    /// The overlay composited on top of every slide.
    overlay: Overlay,
    /// The style the slides are drawn in.
    style: StyleOverride,
    /// The frame rate the transition durations are measured against.
    fps: f32,
    /// Whether motion is reduced: the slide then changes without a
    /// transition.
    reduced_motion: bool,
    /// The transition used when a slide does not name one of its own.
    default_transition: Transition,
}

/// A transition that is running.
///
/// The two pictures are held and the frame is blended when it is asked for:
/// a transition that worked out all of its frames up front would hold the
/// whole motion in memory — a 1080p fade is fifteen pictures of eight
/// megabytes each — and the slide would change a moment after the operator
/// pressed the key rather than when they did.
struct RunningTransition {
    /// The picture the motion starts from.
    from: Frame,
    /// The picture the motion ends on.
    to: Frame,
    /// The motion being run.
    transition: Transition,
    /// The frame rate the duration is measured against.
    fps: f32,
    /// The step the next frame is.
    step: u32,
    /// How many steps the motion runs for.
    steps: u32,
}

impl SlidePlayer {
    /// Create a player.
    pub fn new(fps: f32) -> Self {
        Self {
            current: None,
            running: None,
            overlay: Overlay::default(),
            style: StyleOverride::default(),
            fps: fps.max(1.0),
            reduced_motion: false,
            default_transition: Transition::Cut,
        }
    }

    /// Reduce the motion: slides then change without a transition.
    pub fn set_reduced_motion(&mut self, reduced: bool) {
        self.reduced_motion = reduced;
    }

    /// The transition used when a slide does not name one of its own.
    ///
    /// A slide that asks for a cut has not chosen the motion, it has left it
    /// to the operator's setting.
    pub fn set_default_transition(&mut self, transition: Transition) {
        self.default_transition = transition;
    }

    /// The overlay composited on top of every slide.
    pub fn set_overlay(&mut self, overlay: Overlay) {
        self.overlay = overlay;
    }

    /// The style the slides are drawn in.
    pub fn set_style(&mut self, style: StyleOverride) {
        self.style = style;
    }

    /// Show a slide, running its transition from the slide on screen.
    pub fn show_slide(
        &mut self,
        pipeline: &mut RenderPipeline,
        slide: &Slide,
        video_frame: Option<&RgbaFrame>,
    ) -> anyhow::Result<()> {
        let next = pipeline.render_slide_styled(slide, video_frame, &self.overlay, &self.style)?;

        self.running = None;

        // The screen's theme can carry its own transition. A slide that asks
        // for a cut leaves the motion to the operator's setting. With reduced
        // motion the slide simply changes.
        let transition = if self.reduced_motion {
            Transition::Cut
        } else {
            match self.style.transition {
                Some(theme_transition) => theme_transition,
                None => match slide.transition {
                    Transition::Cut => self.default_transition,
                    other => other,
                },
            }
        };

        match self.current.take() {
            Some(previous) => {
                let steps = crate::render::transition_length(transition, self.fps);
                if steps == 0 {
                    // A cut: the new slide is on screen straight away.
                    self.current = Some(next);
                } else {
                    // The motion runs from here: the frames are blended as
                    // they are asked for, so the change starts at once.
                    self.running = Some(RunningTransition {
                        from: previous,
                        to: next,
                        transition,
                        fps: self.fps,
                        step: 0,
                        steps,
                    });
                }
            }
            None => self.current = Some(next),
        }
        Ok(())
    }

    /// Re-render the slide without changing the transition.
    ///
    /// The overlay or the video frame changed, but the slide did not: the
    /// picture is replaced, the motion is left alone.
    pub fn rerender(
        &mut self,
        pipeline: &mut RenderPipeline,
        slide: &Slide,
        video_frame: Option<&RgbaFrame>,
    ) -> anyhow::Result<()> {
        let frame = pipeline.render_slide_styled(slide, video_frame, &self.overlay, &self.style)?;
        if let Some(running) = self.running.as_mut() {
            // A transition is running: the frame it is heading for is the
            // one that has to be redrawn, so the picture that ends up on
            // screen is the current one. The transition itself keeps
            // playing.
            running.to = frame;
        } else {
            self.current = Some(frame);
        }
        Ok(())
    }

    /// The next frame to send.
    ///
    /// While a transition runs this is the next transition frame; otherwise
    /// it is the slide on screen. A broadcast needs a frame on every tick,
    /// so the current frame is returned again once the transition is over.
    pub fn next_frame(&mut self) -> Option<Frame> {
        let Some(running) = self.running.as_mut() else {
            return self.current.clone();
        };
        // The frame is blended now, not in advance: the motion starts at
        // once and the player holds two pictures rather than a whole
        // transition's worth of them.
        let frame = transition_frame_at(
            &running.from,
            &running.to,
            running.transition,
            running.fps,
            running.step,
        );
        running.step += 1;
        if running.step >= running.steps {
            // The transition finished: the target slide becomes current.
            let running = self.running.take().expect("the transition is running");
            self.current = Some(running.to);
        }
        frame
    }

    /// Whether a transition is still running.
    pub fn is_transitioning(&self) -> bool {
        self.running.is_some()
    }

    /// Black the output.
    pub fn black(&mut self, pipeline: &RenderPipeline) {
        self.current = Some(pipeline.render_black());
        self.running = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Transition;
    use crate::domain::{Background, Color, Object, ObjectContent, ShapeType, SlideContent};

    /// A slide filled with one colour.
    fn slide(color: [u8; 3], transition: Transition) -> Slide {
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
            transition,
            group: None,
            notes: None,
            layers: vec![shape],
            cues: Vec::new(),
        }
    }

    #[test]
    fn a_cut_shows_the_new_slide_straight_away() {
        let mut pipeline = RenderPipeline::new_software(16, 16);
        let mut player = SlidePlayer::new(30.0);
        player
            .show_slide(&mut pipeline, &slide([255, 0, 0], Transition::Cut), None)
            .expect("the slide renders");
        assert!(!player.is_transitioning());

        let frame = player.next_frame().expect("a frame is on screen");
        assert_eq!(frame.pixel(8, 8).expect("a pixel").0, 255);
    }

    #[test]
    fn a_slide_without_its_own_transition_uses_the_default() {
        let mut pipeline = RenderPipeline::new_software(16, 16);
        let mut player = SlidePlayer::new(30.0);
        player.set_default_transition(Transition::Fade { duration_ms: 300 });
        player
            .show_slide(&mut pipeline, &slide([255, 0, 0], Transition::Cut), None)
            .expect("the slide renders");

        // The slide asked for a cut, so the operator's default decides.
        player
            .show_slide(&mut pipeline, &slide([0, 0, 255], Transition::Cut), None)
            .expect("the slide renders");
        assert!(player.is_transitioning(), "the default transition runs");

        // A slide that names its own transition keeps it.
        let mut player = SlidePlayer::new(30.0);
        player.set_default_transition(Transition::Fade { duration_ms: 300 });
        player
            .show_slide(&mut pipeline, &slide([255, 0, 0], Transition::Cut), None)
            .expect("the slide renders");
        player
            .show_slide(
                &mut pipeline,
                &slide([0, 0, 255], Transition::Slide { duration_ms: 300 }),
                None,
            )
            .expect("the slide renders");
        assert!(player.is_transitioning(), "the slide's own transition runs");
    }

    #[test]
    fn reduced_motion_changes_the_slide_without_a_transition() {
        let mut pipeline = RenderPipeline::new_software(16, 16);
        let mut player = SlidePlayer::new(30.0);
        player.set_reduced_motion(true);
        player
            .show_slide(&mut pipeline, &slide([255, 0, 0], Transition::Cut), None)
            .expect("the slide renders");

        // The next slide fades in, but with reduced motion it simply changes.
        player
            .show_slide(
                &mut pipeline,
                &slide([0, 0, 255], Transition::Fade { duration_ms: 300 }),
                None,
            )
            .expect("the slide renders");
        assert!(!player.is_transitioning());

        let frame = player.next_frame().expect("the new slide is on screen");
        assert_eq!(frame.pixel(8, 8).expect("a pixel").2, 255);
    }

    /// A transition is drawn a frame at a time.
    ///
    /// The motion is blended when the frame is asked for, so the player
    /// holds two pictures rather than a whole transition's worth of them: a
    /// 1080p fade would otherwise be a hundred megabytes in memory before
    /// the first frame was on screen, and the slide would change a moment
    /// after the operator pressed the key rather than when they did.
    #[test]
    fn a_transition_is_drawn_a_frame_at_a_time() {
        let mut pipeline = RenderPipeline::new_software(16, 16);
        let mut player = SlidePlayer::new(30.0);
        player
            .show_slide(&mut pipeline, &slide([255, 0, 0], Transition::Cut), None)
            .expect("the slide renders");
        player
            .show_slide(
                &mut pipeline,
                &slide([0, 0, 255], Transition::Fade { duration_ms: 5000 }),
                None,
            )
            .expect("the slide renders");

        // The motion is running, and it is held as two pictures and a step.
        assert!(player.is_transitioning());
        let running = player.running.as_ref().expect("the motion is running");
        assert_eq!(running.steps, 150, "the motion runs for 150 frames");
        assert_eq!(running.step, 0, "and it has drawn none of them yet");

        // The first frame is blended when it is asked for.
        let frame = player.next_frame().expect("a frame is on screen");
        assert!(frame.pixel(8, 8).is_some(), "the frame has a picture in it");
        assert_eq!(
            player.running.as_ref().expect("the motion runs").step,
            1,
            "the motion has moved one step"
        );
    }

    #[test]
    fn a_fade_steps_through_the_transition() {
        let mut pipeline = RenderPipeline::new_software(16, 16);
        let mut player = SlidePlayer::new(30.0);
        player
            .show_slide(&mut pipeline, &slide([255, 0, 0], Transition::Cut), None)
            .expect("the slide renders");

        // 300 ms at 30 fps is nine transition frames.
        player
            .show_slide(
                &mut pipeline,
                &slide([0, 0, 255], Transition::Fade { duration_ms: 300 }),
                None,
            )
            .expect("the slide renders");
        assert!(player.is_transitioning());

        let mut steps = 0;
        let mut previous_blue = 0u8;
        while player.is_transitioning() {
            let frame = player.next_frame().expect("a transition frame");
            steps += 1;
            // The fade moves steadily towards the new (blue) slide.
            let pixel = frame.pixel(8, 8).expect("a pixel");
            assert!(
                pixel.2 > previous_blue,
                "the fade moves towards the new slide"
            );
            previous_blue = pixel.2;
        }
        assert_eq!(steps, 9);

        // The transition ends on the new slide.
        let frame = player.next_frame().expect("the final frame");
        assert_eq!(frame.pixel(8, 8).expect("a pixel").2, 255);
    }
}
