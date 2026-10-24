use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::anyhow;
use ffmpeg_next as ffmpeg;
use ffmpeg_next::format::Pixel;
use ffmpeg_next::software::scaling;

/// A frame as the compositor produced it: tightly packed BGRA.
pub struct RawFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

enum EncoderMsg {
    Frame(RawFrame),
    Finish,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RecordingState {
    #[default]
    Idle,
    Recording,
    Finishing,
}

/// A recording that is running.
pub struct RecordingSession {
    sender: mpsc::SyncSender<EncoderMsg>,
    handle: thread::JoinHandle<anyhow::Result<()>>,
    /// The file this session writes to.
    pub output_path: PathBuf,
    pub started_at: Instant,
    pub frames_submitted: u64,
    pub frames_dropped: u64,
    /// Whether the encoder thread has died.
    ///
    /// The encoder runs on a thread of its own: if it dies, the frames are
    /// dropped and the recording is over, so the operator has to be told.
    pub failed: bool,
}

impl RecordingSession {
    /// Start a recording session.
    ///
    /// The encoder runs on its own thread; the frames are handed to it
    /// through a bounded queue.
    pub fn start(
        output_path: impl AsRef<Path>,
        width: u32,
        height: u32,
        fps: u32,
        codec: crate::config::CaptureCodec,
    ) -> anyhow::Result<Self> {
        let path = output_path.as_ref().to_path_buf();
        let (tx, rx) = mpsc::sync_channel::<EncoderMsg>(64);

        let path_clone = path.clone();
        let handle = thread::spawn(move || {
            // A panic in the encoder would leave the file without its
            // trailer. The panic is caught so it is in the log — a panic
            // message only reaches the terminal, and the terminal is gone
            // when the operator reports the problem.
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                encoder_thread_body(path_clone, width, height, fps, codec, rx)
            })) {
                Ok(result) => result,
                Err(_) => Err(anyhow!("the encoder panicked")),
            }
        });

        Ok(Self {
            sender: tx,
            handle,
            output_path: path,
            started_at: Instant::now(),
            frames_submitted: 0,
            frames_dropped: 0,
            failed: false,
        })
    }

    pub fn submit_frame(&mut self, data: Vec<u8>, width: u32, height: u32) -> bool {
        let frame = RawFrame {
            data,
            width,
            height,
        };
        let outcome = self.sender.try_send(EncoderMsg::Frame(frame));
        if let Err(mpsc::TrySendError::Disconnected(_)) = &outcome {
            self.failed = true;
        }
        match outcome {
            Ok(()) => {
                self.frames_submitted += 1;
                true
            }
            Err(mpsc::TrySendError::Full(_)) => {
                self.frames_dropped += 1;
                // Log the first drop, then only every 30th: this fires per frame
                // and would otherwise flood the log on a long recording. The
                // running total is surfaced in the recording panel regardless.
                if self.frames_dropped == 1 || self.frames_dropped.is_multiple_of(30) {
                    log::warn!(
                        "[recording] encoder queue full — dropped frame #{} (total dropped: {})",
                        self.frames_submitted + self.frames_dropped,
                        self.frames_dropped
                    );
                }
                false
            }
            Err(mpsc::TrySendError::Disconnected(_)) => false,
        }
    }

    /// Whether the encoder thread has died.
    pub fn failed(&self) -> bool {
        self.failed
    }

    /// Write the file out on a background thread.
    ///
    /// The encoder is told to finish and the trailer of the file is written
    /// after it — the finish signal blocks while the encoder queue is full,
    /// and the write waits for the encoder to drain. Neither belongs on the
    /// UI thread, so both happen here.
    fn finalize_in_background(self) -> Finalizer {
        let finalizer = Finalizer::new();
        let done = Arc::clone(&finalizer.done);
        let result = Arc::clone(&finalizer.result);
        let (sender, handle) = (self.sender, self.handle);
        thread::spawn(move || {
            let _ = sender.send(EncoderMsg::Finish);
            let outcome = handle
                .join()
                .unwrap_or_else(|_| Err(anyhow!("the encoder thread panicked")));
            if let Ok(mut slot) = result.lock() {
                *slot = Some(outcome);
            }
            done.store(true, Ordering::SeqCst);
        });
        finalizer
    }

    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }
}

/// The write-out of a recording that has been stopped.
///
/// The encoder writes the trailer of the file after it is told to finish, so
/// the last part of a recording happens after the operator pressed stop.
struct Finalizer {
    /// Set once the encoder thread is done.
    done: Arc<AtomicBool>,
    /// The result of the encoder thread, once it is done.
    result: Arc<Mutex<Option<anyhow::Result<()>>>>,
}

impl Finalizer {
    fn new() -> Self {
        Self {
            done: Arc::new(AtomicBool::new(false)),
            result: Arc::new(Mutex::new(None)),
        }
    }

    /// Whether the file has been written.
    fn is_done(&self) -> bool {
        self.done.load(Ordering::SeqCst)
    }

    /// The result of the encoder thread, once it is done.
    fn take_result(&self) -> Option<anyhow::Result<()>> {
        self.result.lock().ok().and_then(|mut slot| slot.take())
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
    codec: crate::config::CaptureCodec,
    rx: mpsc::Receiver<EncoderMsg>,
) -> anyhow::Result<()> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    ffmpeg::init().map_err(|e| anyhow!("ffmpeg init: {e}"))?;

    let mut octx = ffmpeg::format::output(&output_path)
        .map_err(|e| anyhow!("open output '{}': {e}", output_path.display()))?;

    // The codec is the operator's choice: H.264 is what everything plays,
    // and H.265 is a smaller file for the same picture.
    let codec_id = match codec {
        crate::config::CaptureCodec::H264 => ffmpeg::codec::Id::H264,
        crate::config::CaptureCodec::H265 => ffmpeg::codec::Id::H265,
    };
    let codec = ffmpeg::encoder::find(codec_id).ok_or_else(|| {
        anyhow!("the {codec_id} encoder was not found — ffmpeg was built without it")
    })?;

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

    let mut last_pts: i64 = -1;
    let mut frames_encoded: u64 = 0;
    let started = Instant::now();

    while let Ok(EncoderMsg::Frame(raw)) = rx.recv() {
        let mut bgra_frame = ffmpeg::frame::Video::new(Pixel::BGRA, width, height);
        {
            let stride = bgra_frame.stride(0);
            let dst = bgra_frame.data_mut(0);
            if let Err(e) = copy_bgra_rows(dst, stride, &raw.data, width, height) {
                // A frame that does not fit is left out: the recording keeps
                // going with the frames that do.
                log::warn!("[recording] skipped a frame: {e}");
                continue;
            }
        }

        let mut yuv_frame = ffmpeg::frame::Video::empty();
        scaler
            .run(&bgra_frame, &mut yuv_frame)
            .map_err(|e| anyhow!("scaler.run: {e}"))?;
        // The timestamp follows the wall clock, not the frame counter: if the
        // compositor cannot keep up and a frame is dropped, the video still
        // plays at the right speed instead of in slow motion.
        let raw_pts = (started.elapsed().as_secs_f64() * fps as f64).round() as i64;
        let pts = next_pts(raw_pts, last_pts);
        yuv_frame.set_pts(Some(pts));
        last_pts = pts;

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

    log::info!(
        "[recording] Done. {frames_encoded} frames → {}",
        output_path.display()
    );
    Ok(())
}

/// Copy a tightly packed BGRA picture into the encoder's frame buffer.
///
/// The buffer is allocated with a 32-byte alignment, so its rows do not
/// start `width * 4` bytes apart: every row starts `stride` bytes after the
/// one before it and the tail of a row is padding. A flat copy of the packed
/// source would shift every row by that padding, which shears the picture.
///
/// The source has to hold exactly `width * height * 4` bytes: a frame of a
/// different size is refused instead of half-copied.
fn copy_bgra_rows(
    dst: &mut [u8],
    stride: usize,
    src: &[u8],
    width: u32,
    height: u32,
) -> anyhow::Result<()> {
    let row_bytes = width as usize * 4;
    let expected = row_bytes * height as usize;
    if src.len() != expected {
        return Err(anyhow!(
            "a {width}×{height} frame needs {expected} bytes, got {}",
            src.len()
        ));
    }
    if stride < row_bytes {
        return Err(anyhow!(
            "the frame stride {stride} is smaller than a row of {row_bytes} bytes"
        ));
    }
    if dst.len() < stride * height as usize {
        return Err(anyhow!(
            "the frame buffer holds {} bytes, {width}×{height} at stride {stride} needs {}",
            dst.len(),
            stride * height as usize
        ));
    }
    for row in 0..height as usize {
        let from = row * row_bytes;
        let to = row * stride;
        dst[to..to + row_bytes].copy_from_slice(&src[from..from + row_bytes]);
    }
    Ok(())
}

/// The timestamp of the next frame.
///
/// The wall clock can hand out the same value twice, so the timestamp is
/// clamped to advance by one. The clamped value is the one that is kept —
/// storing the raw one would leave the clamp where it was, and the frames
/// would share a timestamp.
fn next_pts(raw_pts: i64, last_pts: i64) -> i64 {
    raw_pts.max(last_pts + 1)
}

/// The recording panel's state: the session that runs, and where it writes.
#[derive(Default)]
pub struct RecordingManager {
    /// What the operator sees: idle, recording, or writing the file out.
    pub state: RecordingState,
    session: Option<RecordingSession>,
    /// The file the operator named. An empty path means "pick a name".
    pub output_path: String,
    pub frame_width: u32,
    pub frame_height: u32,
    pub fps: u32,
    /// The output resolution changed while the recording was running.
    ///
    /// The encoder is started at a fixed size, so the frames of the new
    /// resolution cannot be encoded. They are left out rather than fed to
    /// the encoder with the wrong size, and the operator is told that the
    /// recording kept the size it started with.
    pub resolution_changed: bool,
    /// The file the running session — or the one that just ended — writes to.
    ///
    /// This is not the operator's setting: a generated name belongs to the
    /// session, so the next session picks a name of its own instead of
    /// writing over the file of the last one.
    pub session_path: Option<PathBuf>,
    /// The write-out of the session that was stopped.
    finalizer: Option<Finalizer>,
    /// The names this manager already used, so two takes never share a file.
    used_paths: Vec<PathBuf>,
    /// The preset the recordings are made with.
    ///
    /// A service is recorded the same way every week, so the folder, the
    /// name and the frame rate are chosen once and kept.
    pub preset: Option<crate::config::CapturePreset>,
    /// The directory a generated recording is written to.
    video_dir: PathBuf,
    /// The totals of the take that ended last, so they survive the stop.
    last_frames_captured: u64,
    last_frames_dropped: u64,
    last_elapsed: Option<Duration>,
}

impl RecordingManager {
    pub fn new() -> Self {
        Self {
            state: RecordingState::Idle,
            session: None,
            output_path: String::new(),
            frame_width: 1920,
            frame_height: 1080,
            fps: 60,
            resolution_changed: false,
            session_path: None,
            finalizer: None,
            used_paths: Vec::new(),
            preset: None,
            video_dir: dirs::video_dir().unwrap_or_else(|| PathBuf::from(".")),
            last_frames_captured: 0,
            last_frames_dropped: 0,
            last_elapsed: None,
        }
    }

    /// Start a recording session.
    ///
    /// The operator's path is used as it is. An empty path gets a generated
    /// name, and that name is unique: the encoder truncates the file it
    /// opens, so two takes that shared a name would destroy the first one.
    pub fn start(&mut self) -> anyhow::Result<()> {
        if self.state != RecordingState::Idle {
            return Ok(());
        }
        // The preset says how the take is made: a service recorded at a
        // different frame rate every week is a service the operator cannot
        // put on the website afterwards.
        if let Some(preset) = self.preset.as_ref() {
            self.fps = preset.fps.clamp(1, 60);
        }
        let path = self.resolve_output_path();
        // The codec is the operator's choice: H.264 is what everything
        // plays, and H.265 is a smaller file for the same picture.
        let codec = self
            .preset
            .as_ref()
            .map(|preset| preset.codec)
            .unwrap_or_default();
        let session =
            RecordingSession::start(&path, self.frame_width, self.frame_height, self.fps, codec)?;
        self.session_path = Some(path);
        self.session = Some(session);
        self.state = RecordingState::Recording;
        Ok(())
    }

    /// The file the next session writes to.
    ///
    /// A path the operator named is used as it is. An empty path gets a
    /// generated name, and the name is unique — the encoder truncates the
    /// file it opens, so a repeated name would destroy the previous take.
    fn resolve_output_path(&mut self) -> PathBuf {
        // A preset is the standing choice: it says where the take goes and
        // what it is called. The operator's own path is the one-off
        // override, and it is used when there is no preset.
        let stamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        if let Some(preset) = self.preset.as_ref() {
            let dir = if preset.destination.trim().is_empty() {
                if self.video_dir.as_os_str().is_empty() {
                    PathBuf::from(".")
                } else {
                    self.video_dir.clone()
                }
            } else {
                expand_home(&preset.destination)
            };
            // The preset names the take: every week of a service has its own
            // file, and none of them overwrites another.
            let name = preset.resolve_file_name((stamp_ms / 1000) as u64);
            let candidate = dir.join(format!("{name}.mp4"));
            if !self.used_paths.contains(&candidate) && !candidate.exists() {
                self.used_paths.push(candidate.clone());
                return candidate;
            }
            // The name is taken, so the take is given a number of its own:
            // the encoder truncates the file it opens, so a repeated name
            // would destroy the take before it.
            let mut attempt = 1u32;
            loop {
                let candidate = dir.join(format!("{name}-{attempt}.mp4"));
                if !self.used_paths.contains(&candidate) && !candidate.exists() {
                    self.used_paths.push(candidate.clone());
                    return candidate;
                }
                attempt += 1;
            }
        }
        if !self.output_path.trim().is_empty() {
            return expand_home(&self.output_path);
        }
        let dir = if self.video_dir.as_os_str().is_empty() {
            PathBuf::from(".")
        } else {
            self.video_dir.clone()
        };
        let mut attempt = 0u32;
        loop {
            let candidate = generated_output_path(&dir, stamp_ms, attempt);
            if !self.used_paths.contains(&candidate) && !candidate.exists() {
                self.used_paths.push(candidate.clone());
                return candidate;
            }
            attempt += 1;
        }
    }

    /// Use a preset for the recordings.
    ///
    /// The preset is the operator's standing choice of where the take goes
    /// and how it is named, so it is applied when it is chosen rather than
    /// when the recording starts.
    pub fn apply_preset(&mut self, preset: Option<crate::config::CapturePreset>) {
        if let Some(preset) = preset.as_ref() {
            self.fps = preset.fps.clamp(1, 60);
        }
        self.preset = preset;
    }

    /// Whether the operator is asked before the recording stops.
    pub fn confirm_before_stopping(&self) -> bool {
        self.preset
            .as_ref()
            .map(|preset| preset.confirm_before_stopping)
            .unwrap_or(false)
    }

    /// Stop the recording.
    ///
    /// The encoder is told to finish and the file is written out on a
    /// background thread: the finish signal can block while the encoder
    /// queue is full, and the trailer is written after it. The UI thread
    /// must not wait for either, so the state stays
    /// [`RecordingState::Finishing`] until the file is there.
    pub fn stop(&mut self) {
        if self.state != RecordingState::Recording {
            return;
        }
        self.state = RecordingState::Finishing;
        let Some(session) = self.session.take() else {
            self.state = RecordingState::Idle;
            return;
        };
        // The totals of the take that just ended stay available: the
        // operator wants to see them after the stop.
        self.remember_take(
            session.frames_submitted,
            session.frames_dropped,
            session.elapsed(),
        );
        self.finalizer = Some(session.finalize_in_background());
    }

    /// Keep the totals of the take that just ended.
    fn remember_take(&mut self, captured: u64, dropped: u64, elapsed: Duration) {
        self.last_frames_captured = captured;
        self.last_frames_dropped = dropped;
        self.last_elapsed = Some(elapsed);
    }

    /// Whether the recording that was stopped has been written out.
    pub fn is_finished(&self) -> bool {
        self.finalizer.as_ref().is_some_and(|f| f.is_done())
    }

    /// Collect the result of the finalised recording.
    ///
    /// The state returns to [`RecordingState::Idle`] once the file is there.
    pub fn take_finish_result(&mut self) -> Option<anyhow::Result<()>> {
        if !self.finalizer.as_ref()?.is_done() {
            // The encoder is still writing the file.
            return None;
        }
        let finalizer = self.finalizer.take()?;
        self.state = RecordingState::Idle;
        Some(
            finalizer
                .take_result()
                .unwrap_or_else(|| Err(anyhow!("the encoder thread ended without a result"))),
        )
    }

    pub fn submit_frame(&mut self, data: Vec<u8>, width: u32, height: u32) -> bool {
        if let Some(ref mut session) = self.session {
            session.submit_frame(data, width, height)
        } else {
            false
        }
    }

    /// Whether the encoder thread has died.
    ///
    /// A dead encoder drops every frame, so the recording is over without the
    /// operator knowing.
    pub fn encoder_failed(&self) -> bool {
        self.session.as_ref().is_some_and(|s| s.failed())
    }

    /// The frames the encoder took.
    ///
    /// While a session runs this is the running total; after the stop it is
    /// the total of the take that just ended.
    pub fn frames_dropped(&self) -> u64 {
        self.session
            .as_ref()
            .map(|s| s.frames_dropped)
            .unwrap_or(self.last_frames_dropped)
    }

    /// How long the recording ran.
    ///
    /// While a session runs this is its live duration; after the stop it is
    /// the duration of the take that just ended.
    pub fn elapsed(&self) -> Option<Duration> {
        self.session
            .as_ref()
            .map(|s| s.elapsed())
            .or(self.last_elapsed)
    }

    /// The frames the encoder took.
    ///
    /// While a session runs this is the running total; after the stop it is
    /// the total of the take that just ended.
    pub fn frames_captured(&self) -> u64 {
        self.session
            .as_ref()
            .map(|s| s.frames_submitted)
            .unwrap_or(self.last_frames_captured)
    }
}

/// The name of a recording the operator did not name.
///
/// The name carries the wall clock down to the millisecond, and the attempt
/// counter keeps it unique when two takes start within the same millisecond.
/// A path the operator typed, with `~` standing for their home directory.
///
/// The panel's own placeholder shows `~/Videos/output.mp4`, so that is what
/// the operator types: a path that is used literally would land the take in a
/// folder called "~".
fn expand_home(path: &str) -> PathBuf {
    let trimmed = path.trim();
    if trimmed == "~" {
        return std::env::home_dir().unwrap_or_else(|| PathBuf::from("~"));
    }
    if let Some(rest) = trimmed.strip_prefix("~/")
        && let Some(home) = std::env::home_dir()
    {
        return home.join(rest);
    }
    PathBuf::from(trimmed)
}

fn generated_output_path(dir: &Path, stamp_ms: u128, attempt: u32) -> PathBuf {
    let name = if attempt == 0 {
        format!("OpenPresenter_{stamp_ms}.mp4")
    } else {
        format!("OpenPresenter_{stamp_ms}_{attempt}.mp4")
    };
    dir.join(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The stride the encoder's buffer has for a given width: the rows are
    /// padded to a multiple of 32 bytes.
    fn padded_stride(width: u32) -> usize {
        let row_bytes = width as usize * 4;
        row_bytes.div_ceil(32) * 32
    }

    #[test]
    fn the_rows_land_at_the_stride_of_the_buffer() {
        // 1366 px is not a multiple of 8 px, so the rows of the buffer are
        // padded and a flat copy would shear the picture.
        let width = 1366u32;
        let height = 3u32;
        let row_bytes = width as usize * 4;
        let stride = padded_stride(width);
        assert!(stride > row_bytes, "the test needs a padded stride");

        let mut src = vec![0u8; row_bytes * height as usize];
        for row in 0..height as usize {
            src[row * row_bytes] = (row + 1) as u8;
        }
        let mut dst = vec![0u8; stride * height as usize];

        copy_bgra_rows(&mut dst, stride, &src, width, height).unwrap();

        for row in 0..height as usize {
            assert_eq!(
                dst[row * stride],
                (row + 1) as u8,
                "row {row} landed at the wrong offset"
            );
            // The padding behind the row is left alone.
            assert!(
                dst[row * stride + row_bytes..(row + 1) * stride]
                    .iter()
                    .all(|byte| *byte == 0),
                "row {row} wrote into the padding"
            );
        }
    }

    #[test]
    fn a_frame_of_the_wrong_size_is_refused() {
        let mut dst = vec![0u8; padded_stride(4) * 4];
        let err = copy_bgra_rows(&mut dst, padded_stride(4), &[0u8; 10], 4, 4).unwrap_err();
        assert!(
            err.to_string().contains("needs"),
            "the mismatch is reported: {err}"
        );
    }

    #[test]
    fn the_timestamp_always_advances() {
        // The first frame starts at zero.
        assert_eq!(next_pts(0, -1), 0);
        // The wall clock handing out the same value twice still advances.
        assert_eq!(next_pts(5, 5), 6);
        // A clock that jumps backwards does not repeat a timestamp.
        assert_eq!(next_pts(3, 7), 8);
    }

    #[test]
    fn two_recordings_choose_different_files() {
        let dir = tempfile::tempdir().unwrap();
        let mut manager = RecordingManager::new();
        manager.video_dir = dir.path().to_path_buf();
        manager.output_path = String::new();

        let first = manager.resolve_output_path();
        let second = manager.resolve_output_path();

        assert_ne!(first, second, "the second take writes to its own file");
        assert!(
            manager.output_path.is_empty(),
            "the operator's path is not rewritten by a recording"
        );
    }

    #[test]
    fn an_operator_path_is_used_as_it_is() {
        let mut manager = RecordingManager::new();
        manager.output_path = "/tmp/openpresenter-take.mp4".to_string();
        assert_eq!(
            manager.resolve_output_path(),
            PathBuf::from("/tmp/openpresenter-take.mp4")
        );
    }

    /// A path the operator typed with `~` is their home directory.
    ///
    /// The panel's own placeholder shows `~/Videos/output.mp4`, so that is
    /// what the operator types: a path used literally would land the take in
    /// a folder called "~".
    #[test]
    fn a_tilde_path_is_the_home_directory() {
        let home = std::env::home_dir().expect("a home directory");
        let mut manager = RecordingManager::new();
        manager.output_path = "~/Videos/output.mp4".to_string();
        assert_eq!(
            manager.resolve_output_path(),
            home.join("Videos/output.mp4")
        );
    }

    /// A path the operator typed is used as it is.
    #[test]
    fn a_plain_path_is_used_as_it_is() {
        let mut manager = RecordingManager::new();
        manager.output_path = "/tmp/openpresenter-take.mp4".to_string();
        assert_eq!(
            manager.resolve_output_path(),
            PathBuf::from("/tmp/openpresenter-take.mp4")
        );
    }

    #[test]
    fn the_totals_of_the_last_take_survive_the_stop() {
        let mut manager = RecordingManager::new();
        manager.remember_take(120, 3, Duration::from_secs(7));

        assert_eq!(manager.frames_captured(), 120);
        assert_eq!(manager.frames_dropped(), 3);
        assert_eq!(manager.elapsed(), Some(Duration::from_secs(7)));
    }
}
