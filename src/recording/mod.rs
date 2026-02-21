use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::anyhow;
use ffmpeg_next as ffmpeg;
use ffmpeg_next::format::Pixel;
use ffmpeg_next::software::scaling;

pub struct RawFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

enum EncoderMsg {
    Frame(RawFrame),
    Finish,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum RecordingState {
    #[default]
    Idle,
    Recording,
    Finishing,
}


pub struct RecordingSession {
    sender: mpsc::SyncSender<EncoderMsg>,
    pub output_path: PathBuf,
    pub started_at: Instant,
    pub frames_submitted: u64,
    pub frames_dropped: u64,
}

impl RecordingSession {
    pub fn start(
        output_path: impl AsRef<Path>,
        width: u32,
        height: u32,
        fps: u32,
    ) -> anyhow::Result<(Self, thread::JoinHandle<anyhow::Result<()>>)> {
        let path = output_path.as_ref().to_path_buf();
        let (tx, rx) = mpsc::sync_channel::<EncoderMsg>(64);

        let path_clone = path.clone();
        let handle = thread::spawn(move || encoder_thread_body(path_clone, width, height, fps, rx));

        let session = Self {
            sender: tx,
            output_path: path,
            started_at: Instant::now(),
            frames_submitted: 0,
            frames_dropped: 0,
        };
        Ok((session, handle))
    }

    pub fn submit_frame(&mut self, data: Vec<u8>, width: u32, height: u32) -> bool {
        let frame = RawFrame {
            data,
            width,
            height,
        };
        match self.sender.try_send(EncoderMsg::Frame(frame)) {
            Ok(()) => {
                self.frames_submitted += 1;
                true
            }
            Err(mpsc::TrySendError::Full(_)) => {
                self.frames_dropped += 1;
                eprintln!(
                    "[recording] encoder queue full — dropped frame #{} (total dropped: {})",
                    self.frames_submitted + self.frames_dropped,
                    self.frames_dropped
                );
                false
            }
            Err(mpsc::TrySendError::Disconnected(_)) => false,
        }
    }

    pub fn finish(self) {
        let _ = self.sender.send(EncoderMsg::Finish);
    }

    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }
}

fn drain_encoder(
    encoder: &mut ffmpeg::codec::encoder::Video,
    octx: &mut ffmpeg::format::context::Output,
    stream_index: usize,
    codec_time_base: ffmpeg::Rational,
    stream_time_base: ffmpeg::Rational,
) -> anyhow::Result<()> {
    let mut packet = ffmpeg::codec::packet::Packet::empty();
    loop {
        match encoder.receive_packet(&mut packet) {
            Ok(()) => {
                packet.set_stream(stream_index);
                packet.rescale_ts(codec_time_base, stream_time_base);
                packet
                    .write_interleaved(octx)
                    .map_err(|e| anyhow!("write_interleaved: {e}"))?;
            }
            Err(ffmpeg::Error::Other {
                errno: ffmpeg_next::ffi::EAGAIN,
            }) => break,
            Err(ffmpeg::Error::Eof) => break,
            Err(e) => return Err(anyhow!("receive_packet: {e}")),
        }
    }
    Ok(())
}

fn encoder_thread_body(
    output_path: PathBuf,
    width: u32,
    height: u32,
    fps: u32,
    rx: mpsc::Receiver<EncoderMsg>,
) -> anyhow::Result<()> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    ffmpeg::init().map_err(|e| anyhow!("ffmpeg init: {e}"))?;

    let mut octx = ffmpeg::format::output(&output_path)
        .map_err(|e| anyhow!("open output '{}': {e}", output_path.display()))?;

    let codec = ffmpeg::encoder::find(ffmpeg::codec::Id::H264)
        .ok_or_else(|| anyhow!("libx264 encoder not found — ffmpeg built without H264 support"))?;

    let mut ost = octx
        .add_stream(codec)
        .map_err(|e| anyhow!("add_stream: {e}"))?;
    let ost_index = ost.index();

    let codec_time_base = ffmpeg::Rational::new(1, fps as i32);
    let enc_ctx = ffmpeg::codec::context::Context::new_with_codec(codec);
    let mut video_enc = enc_ctx
        .encoder()
        .video()
        .map_err(|e| anyhow!("encoder().video(): {e}"))?;

    video_enc.set_width(width);
    video_enc.set_height(height);
    video_enc.set_format(Pixel::YUV420P);
    video_enc.set_time_base(codec_time_base);
    video_enc.set_frame_rate(Some(ffmpeg::Rational::new(fps as i32, 1)));

    let mut x264_opts = ffmpeg::Dictionary::new();
    x264_opts.set("preset", "fast");
    x264_opts.set("crf", "23");
    x264_opts.set("tune", "zerolatency");

    let mut encoder = video_enc
        .open_as_with(codec, x264_opts)
        .map_err(|e| anyhow!("open encoder: {e}"))?;

    ost.set_parameters(&encoder);
    ost.set_time_base(codec_time_base);
    let stream_time_base = ost.time_base();

    let mut scaler = scaling::Context::get(
        Pixel::BGRA,
        width,
        height,
        Pixel::YUV420P,
        width,
        height,
        scaling::Flags::BILINEAR,
    )
    .map_err(|e| anyhow!("scaling context: {e}"))?;

    octx.write_header()
        .map_err(|e| anyhow!("write_header: {e}"))?;

    let stream_time_base = octx
        .stream(ost_index)
        .map(|s| s.time_base())
        .unwrap_or(stream_time_base);

    let mut pts: i64 = 0;
    let mut frames_encoded: u64 = 0;

    loop {
        match rx.recv() {
            Ok(EncoderMsg::Frame(raw)) => {
                let mut bgra_frame = ffmpeg::frame::Video::new(Pixel::BGRA, width, height);
                {
                    let dst = bgra_frame.data_mut(0);
                    let src = &raw.data;
                    let copy_len = dst.len().min(src.len());
                    dst[..copy_len].copy_from_slice(&src[..copy_len]);
                }

                let mut yuv_frame = ffmpeg::frame::Video::empty();
                scaler
                    .run(&bgra_frame, &mut yuv_frame)
                    .map_err(|e| anyhow!("scaler.run: {e}"))?;
                yuv_frame.set_pts(Some(pts));
                pts += 1;

                encoder
                    .send_frame(&yuv_frame)
                    .map_err(|e| anyhow!("send_frame: {e}"))?;
                drain_encoder(
                    &mut encoder,
                    &mut octx,
                    ost_index,
                    codec_time_base,
                    stream_time_base,
                )?;

                frames_encoded += 1;
            }
            Ok(EncoderMsg::Finish) | Err(_) => break,
        }
    }

    encoder.send_eof().map_err(|e| anyhow!("send_eof: {e}"))?;
    drain_encoder(
        &mut encoder,
        &mut octx,
        ost_index,
        codec_time_base,
        stream_time_base,
    )?;

    octx.write_trailer()
        .map_err(|e| anyhow!("write_trailer: {e}"))?;

    eprintln!(
        "[recording] Done. {frames_encoded} frames → {}",
        output_path.display()
    );
    Ok(())
}

#[derive(Default)]
pub struct RecordingManager {
    pub state: RecordingState,
    session: Option<RecordingSession>,
    join_handle: Option<thread::JoinHandle<anyhow::Result<()>>>,
    pub output_path: String,
    pub frame_width: u32,
    pub frame_height: u32,
    pub fps: u32,
}

impl RecordingManager {
    pub fn new() -> Self {
        Self {
            state: RecordingState::Idle,
            session: None,
            join_handle: None,
            output_path: String::new(),
            frame_width: 1920,
            frame_height: 1080,
            fps: 60,
        }
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        if self.state != RecordingState::Idle {
            return Ok(());
        }
        let path = if self.output_path.is_empty() {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            format!(
                "{}/OpenPresenter_{ts}.mp4",
                dirs::video_dir()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_else(|| ".".to_string())
            )
        } else {
            self.output_path.clone()
        };

        let (session, handle) =
            RecordingSession::start(&path, self.frame_width, self.frame_height, self.fps)?;
        self.output_path = path;
        self.session = Some(session);
        self.join_handle = Some(handle);
        self.state = RecordingState::Recording;
        Ok(())
    }

    pub fn stop(&mut self) {
        if self.state != RecordingState::Recording {
            return;
        }
        self.state = RecordingState::Finishing;
        if let Some(session) = self.session.take() {
            session.finish();
        }
        if let Some(h) = self.join_handle.take() {
            let _ = h.join();
        }
        self.state = RecordingState::Idle;
    }

    pub fn submit_frame(&mut self, data: Vec<u8>, width: u32, height: u32) -> bool {
        if let Some(ref mut session) = self.session {
            session.submit_frame(data, width, height)
        } else {
            false
        }
    }

    pub fn frames_dropped(&self) -> u64 {
        self.session.as_ref().map(|s| s.frames_dropped).unwrap_or(0)
    }

    pub fn elapsed(&self) -> Option<Duration> {
        self.session.as_ref().map(|s| s.elapsed())
    }

    pub fn frames_captured(&self) -> u64 {
        self.session
            .as_ref()
            .map(|s| s.frames_submitted)
            .unwrap_or(0)
    }
}
