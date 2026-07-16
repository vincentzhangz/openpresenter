use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{FrameRate, NdiSender};
use crate::Result;
use crate::domain::{Slide, Transition};
use crate::media::decoder::RgbaFrame;
use crate::render::{Frame, RenderPipeline, transition_frames};

#[derive(Clone)]
pub enum OutputCommand {
    ShowSlide(Box<Slide>),
    BlackScreen,
    Clear,
}

struct OutputState {
    command: OutputCommand,
    prev_frame: Option<Frame>,
    transition: Transition,
    transition_remaining: Vec<Frame>,
    video_frame_source: Option<Arc<Mutex<Option<RgbaFrame>>>>,
    transition_pipeline: RenderPipeline,
}

pub struct NdiOutputLoop {
    state: Arc<Mutex<OutputState>>,
    stop_tx: tokio::sync::oneshot::Sender<()>,
}

impl NdiOutputLoop {
    pub fn start(
        source_name: &str,
        width: u32,
        height: u32,
        frame_rate: FrameRate,
    ) -> Result<Self> {
        let fps = frame_rate.numerator as f32 / frame_rate.denominator as f32;
        let frame_interval = Duration::from_secs_f32(1.0 / fps);

        let sender = NdiSender::new(source_name.to_string(), width, height, frame_rate)?;

        let state = Arc::new(Mutex::new(OutputState {
            command: OutputCommand::BlackScreen,
            prev_frame: None,
            transition: Transition::Cut,
            transition_remaining: vec![],
            video_frame_source: None,
            transition_pipeline: RenderPipeline::new_software(width, height),
        }));

        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();

        let state_clone = Arc::clone(&state);
        tokio::spawn(async move {
            output_task(sender, state_clone, frame_interval, stop_rx).await;
        });

        Ok(Self { state, stop_tx })
    }

    pub fn set_slide(&self, slide: Slide) {
        let mut s = self
            .state
            .lock()
            .expect("NDI state mutex poisoned — unrecoverable");
        if let Some(prev) = s.prev_frame.take() {
            let next_frame = s
                .transition_pipeline
                .render_slide(&slide)
                .unwrap_or_else(|_| s.transition_pipeline.render_black());
            let fps = 30.0_f32;
            let frames = transition_frames(&prev, &next_frame, s.transition, fps);
            s.transition_remaining = frames;
        }
        s.transition = slide.transition;
        s.command = OutputCommand::ShowSlide(Box::new(slide));
    }

    pub fn black_screen(&self) {
        let mut s = self
            .state
            .lock()
            .expect("NDI state mutex poisoned — unrecoverable");
        s.transition_remaining.clear();
        s.command = OutputCommand::BlackScreen;
    }

    pub fn clear(&self) {
        let mut s = self
            .state
            .lock()
            .expect("NDI state mutex poisoned — unrecoverable");
        s.transition_remaining.clear();
        s.command = OutputCommand::Clear;
    }

    pub fn stop(self) {
        let _ = self.stop_tx.send(());
    }

    pub fn is_running(&self) -> bool {
        Arc::strong_count(&self.state) > 1
    }

    pub fn set_video_frame_source(&self, src: Option<Arc<Mutex<Option<RgbaFrame>>>>) {
        self.state
            .lock()
            .expect("NDI state mutex poisoned — unrecoverable")
            .video_frame_source = src;
    }
}

async fn output_task(
    mut sender: NdiSender,
    state: Arc<Mutex<OutputState>>,
    frame_interval: Duration,
    mut stop_rx: tokio::sync::oneshot::Receiver<()>,
) {
    let width = sender.width();
    let height = sender.height();
    let mut pipeline = RenderPipeline::new(width, height);

    loop {
        if stop_rx.try_recv().is_ok() {
            break;
        }

        let frame = {
            let mut s = state
                .lock()
                .expect("NDI state mutex poisoned — unrecoverable");

            let video_frame: Option<RgbaFrame> = s
                .video_frame_source
                .as_ref()
                .and_then(|arc| arc.lock().ok()?.clone());

            if let Some(transition_frame) = s.transition_remaining.first().cloned() {
                s.transition_remaining.remove(0);
                transition_frame
            } else {
                let frame = match &s.command {
                    OutputCommand::ShowSlide(slide) => pipeline
                        .render_slide_with_video(slide, video_frame.as_ref())
                        .unwrap_or_else(|_| pipeline.render_black()),
                    OutputCommand::BlackScreen | OutputCommand::Clear => pipeline.render_black(),
                };
                s.prev_frame = Some(frame.clone());
                frame
            }
        };

        if let Err(e) = sender.send_frame(&frame) {
            eprintln!("NDI send error: {}", e);
        }

        tokio::time::sleep(frame_interval).await;
    }
}
