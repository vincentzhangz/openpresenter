use anyhow::{Result, anyhow};
use ffmpeg_next as ffmpeg;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc::{Receiver, Sender, channel},
};
use std::thread;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct RgbaFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub pts_secs: f64,
}

/// Repack a decoded frame into a tightly packed buffer.
///
/// The decoder aligns every row to a 32-byte boundary, so the rows of a
/// frame whose width is not a multiple of eight pixels do not follow one
/// another: the padding of one row is the start of the next. Everything
/// downstream reads the frame as a tight buffer, so the rows are copied out
/// here — a picture that is skewed on the projector is the symptom of
/// skipping this.
pub fn pack_rows(data: &[u8], stride: usize, width: u32, height: u32) -> Vec<u8> {
    let row_bytes = width as usize * 4;
    let mut packed = Vec::with_capacity(row_bytes * height as usize);
    for row in 0..height as usize {
        let start = row * stride;
        let available = data.len().saturating_sub(start).min(row_bytes);
        packed.extend_from_slice(&data[start..start + available]);
        // A frame that is shorter than it claims is padded rather than
        // truncated: a short row would shift every row after it.
        packed.resize(packed.len() + (row_bytes - available), 0);
    }
    packed
}

enum DecoderCmd {
    Play,
    Pause,
    Stop,
    SetLoop(bool),
    Seek(f64),
    SetSpeed(f64),
}

pub struct VideoDecoder {
    current_frame: Arc<Mutex<Option<RgbaFrame>>>,
    position_secs: Arc<Mutex<f64>>,
    cmd_tx: Sender<DecoderCmd>,
    is_playing: Arc<AtomicBool>,
    /// Whether the clip ran to its end.
    ///
    /// The decode thread stops at the end of the file, so the transport
    /// cannot simply start it again: the file has to be opened once more.
    has_ended: Arc<AtomicBool>,
    /// Fatal errors reported by the decode thread, drained by the UI on tick.
    error: Arc<Mutex<Option<String>>>,
    pub width: u32,
    pub height: u32,
    pub duration_secs: f64,
    pub speed: f64,
}

impl VideoDecoder {
    pub fn open(path: &str) -> Result<Self> {
        ffmpeg::init().map_err(|e| anyhow!("ffmpeg init: {e}"))?;

        let ictx = ffmpeg::format::input(path)?;
        let stream = ictx
            .streams()
            .best(ffmpeg::media::Type::Video)
            .ok_or_else(|| anyhow!("no video stream in '{path}'"))?;

        let codec_ctx = ffmpeg::codec::context::Context::from_parameters(stream.parameters())?;
        let probe_decoder = codec_ctx.decoder().video()?;
        let width = probe_decoder.width();
        let height = probe_decoder.height();

        let time_base = stream.time_base();
        let duration_secs = {
            let d = stream.duration();
            if d > 0 {
                d as f64 * f64::from(time_base)
            } else {
                0.0
            }
        };
        drop(ictx);

        let current_frame: Arc<Mutex<Option<RgbaFrame>>> = Arc::new(Mutex::new(None));
        let position_secs: Arc<Mutex<f64>> = Arc::new(Mutex::new(0.0));
        let is_playing = Arc::new(AtomicBool::new(false));
        let has_ended = Arc::new(AtomicBool::new(false));
        let error = Arc::new(Mutex::new(None));
        let (cmd_tx, cmd_rx) = channel::<DecoderCmd>();

        let frame_arc = current_frame.clone();
        let pos_arc = position_secs.clone();
        let playing_arc = is_playing.clone();
        let ended_arc = has_ended.clone();
        let err_arc = error.clone();
        let path_owned = path.to_string();

        thread::Builder::new()
            .name("video-decoder".into())
            .spawn(move || {
                // A decode failure must not be silent: the frame would otherwise
                // freeze on the output with no indication of what went wrong.
                if let Err(e) = decode_loop(
                    path_owned,
                    frame_arc,
                    pos_arc,
                    playing_arc,
                    ended_arc,
                    cmd_rx,
                ) && let Ok(mut slot) = err_arc.lock()
                {
                    *slot = Some(e.to_string());
                }
            })?;

        Ok(Self {
            current_frame,
            position_secs,
            cmd_tx,
            is_playing,
            has_ended,
            error,
            width,
            height,
            duration_secs,
            speed: 1.0,
        })
    }

    pub fn play(&self) {
        self.is_playing.store(true, Ordering::Release);
        let _ = self.cmd_tx.send(DecoderCmd::Play);
    }

    pub fn pause(&self) {
        self.is_playing.store(false, Ordering::Release);
        let _ = self.cmd_tx.send(DecoderCmd::Pause);
    }

    pub fn stop(&self) {
        self.is_playing.store(false, Ordering::Release);
        let _ = self.cmd_tx.send(DecoderCmd::Stop);
    }

    pub fn set_loop(&self, looping: bool) {
        let _ = self.cmd_tx.send(DecoderCmd::SetLoop(looping));
    }

    pub fn set_speed(&mut self, speed: f64) {
        self.speed = speed.max(0.1);
        let _ = self.cmd_tx.send(DecoderCmd::SetSpeed(self.speed));
    }

    pub fn seek(&self, secs: f64) {
        let _ = self.cmd_tx.send(DecoderCmd::Seek(secs));
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::Acquire)
    }

    /// Whether the clip ran to its end.
    ///
    /// The decode thread stops at the end of the file, so a finished clip has
    /// to be opened again before it can play a second time.
    pub fn has_ended(&self) -> bool {
        self.has_ended.load(Ordering::Acquire)
    }

    pub fn position_secs(&self) -> f64 {
        self.position_secs.lock().map(|g| *g).unwrap_or(0.0)
    }

    pub fn current_frame(&self) -> Option<RgbaFrame> {
        self.current_frame.lock().ok()?.clone()
    }

    /// The identity of the current frame, without copying its pixels.
    ///
    /// The decoded frame is a multi-megabyte buffer, so the compositor asks
    /// for the identity first and only copies the pixels when the frame
    /// really changed. A paused video keeps its last frame, and this is how
    /// the compositor knows that the frame on the output is still the right
    /// one.
    pub fn current_frame_identity(&self) -> Option<u64> {
        let guard = self.current_frame.lock().ok()?;
        guard.as_ref().map(|frame| frame.pts_secs.to_bits())
    }

    pub fn shared_frame_arc(&self) -> Arc<Mutex<Option<RgbaFrame>>> {
        Arc::clone(&self.current_frame)
    }

    /// Take a pending fatal decode error, if the decode thread reported one.
    ///
    /// Returns `None` when decoding is healthy, and clears the stored error so
    /// each failure is surfaced to the user exactly once.
    pub fn take_error(&self) -> Option<String> {
        self.error.lock().ok()?.take()
    }
}

pub fn extract_thumbnail(video_path: &str, seek_secs: f64) -> Option<String> {
    ffmpeg::init().ok()?;

    let mut ictx = ffmpeg::format::input(video_path).ok()?;

    let video_idx = ictx.streams().best(ffmpeg::media::Type::Video)?.index();

    let time_base = ictx
        .stream(video_idx)
        .map(|s| s.time_base())
        .unwrap_or(ffmpeg::Rational::new(1, 25));

    let seek_ts = (seek_secs / f64::from(time_base)) as i64;
    let _ = ictx.seek(seek_ts, ..seek_ts);

    let mut decoder = {
        let stream = ictx.stream(video_idx)?;
        let ctx = ffmpeg::codec::context::Context::from_parameters(stream.parameters()).ok()?;
        ctx.decoder().video().ok()?
    };

    let mut scaler = ffmpeg::software::scaling::Context::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        ffmpeg::format::Pixel::RGBA,
        decoder.width(),
        decoder.height(),
        ffmpeg::software::scaling::Flags::BILINEAR,
    )
    .ok()?;

    let mut video_frame = ffmpeg::frame::Video::empty();
    let mut rgba_frame = ffmpeg::frame::Video::empty();

    for (stream, packet) in ictx.packets() {
        if stream.index() != video_idx {
            continue;
        }
        if decoder.send_packet(&packet).is_err() {
            continue;
        }
        if decoder.receive_frame(&mut video_frame).is_ok()
            && scaler.run(&video_frame, &mut rgba_frame).is_ok()
        {
            let w = rgba_frame.width();
            let h = rgba_frame.height();
            let data = pack_rows(rgba_frame.data(0), rgba_frame.stride(0), w, h);

            let img = image::RgbaImage::from_raw(w, h, data)?;

            let thumb_path = {
                let src = std::path::Path::new(video_path);
                let stem = src.file_stem()?.to_string_lossy();
                let dir = src.parent().unwrap_or_else(|| std::path::Path::new("."));
                dir.join(format!("{stem}_thumb.png"))
                    .to_string_lossy()
                    .into_owned()
            };

            img.save(&thumb_path).ok()?;
            return Some(thumb_path);
        }
    }
    None
}

impl Drop for VideoDecoder {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(DecoderCmd::Stop);
    }
}

#[derive(PartialEq)]
enum PauseResult {
    Resumed,
    Stop,
    /// A seek arrived while the video was paused: the frame at the new
    /// position is decoded and the video stays paused.
    Seeked,
}

fn wait_while_paused(
    playing: &Arc<AtomicBool>,
    cmd_rx: &Receiver<DecoderCmd>,
    looping: &mut bool,
    speed: &mut f64,
    seek_to: &mut Option<f64>,
) -> PauseResult {
    playing.store(false, Ordering::Release);
    loop {
        match cmd_rx.recv() {
            Ok(DecoderCmd::Play) => {
                playing.store(true, Ordering::Release);
                return PauseResult::Resumed;
            }
            Ok(DecoderCmd::Stop) | Err(_) => return PauseResult::Stop,
            Ok(DecoderCmd::SetLoop(l)) => *looping = l,
            Ok(DecoderCmd::SetSpeed(s)) => *speed = s.max(0.1),
            Ok(DecoderCmd::Seek(secs)) => {
                *seek_to = Some(secs);
                // The picture follows the sound: a seek that moves the
                // soundtrack but leaves the frame where it was is a
                // transport that lies about where it is.
                return PauseResult::Seeked;
            }
            Ok(DecoderCmd::Pause) => {}
        }
    }
}

fn decode_loop(
    path: String,
    frame_arc: Arc<Mutex<Option<RgbaFrame>>>,
    position_arc: Arc<Mutex<f64>>,
    playing: Arc<AtomicBool>,
    ended: Arc<AtomicBool>,
    cmd_rx: Receiver<DecoderCmd>,
) -> Result<()> {
    let mut looping = false;
    let mut seek_to: Option<f64> = None;
    let mut speed: f64 = 1.0;

    loop {
        match cmd_rx.recv() {
            Ok(DecoderCmd::Play) => break,
            Ok(DecoderCmd::Stop) | Err(_) => return Ok(()),
            Ok(DecoderCmd::SetLoop(l)) => looping = l,
            Ok(DecoderCmd::Seek(s)) => seek_to = Some(s),
            Ok(DecoderCmd::SetSpeed(s)) => speed = s.max(0.1),
            Ok(DecoderCmd::Pause) => {}
        }
    }

    'outer: loop {
        let mut ictx = ffmpeg::format::input(&path)?;

        let video_stream_index = ictx
            .streams()
            .best(ffmpeg::media::Type::Video)
            .ok_or_else(|| anyhow!("no video stream"))?
            .index();

        let time_base = ictx
            .stream(video_stream_index)
            .map(|s| s.time_base())
            .unwrap_or(ffmpeg::Rational::new(1, 25));
        let time_base_f64 = f64::from(time_base);

        if let Some(secs) = seek_to.take() {
            let ts = (secs / time_base_f64) as i64;
            let _ = ictx.seek(ts, ..ts);
        }

        let mut decoder = {
            let stream = ictx
                .stream(video_stream_index)
                .ok_or_else(|| anyhow!("stream not found"))?;
            let ctx = ffmpeg::codec::context::Context::from_parameters(stream.parameters())?;
            ctx.decoder().video()?
        };

        let mut scaler = ffmpeg::software::scaling::Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            ffmpeg::format::Pixel::RGBA,
            decoder.width(),
            decoder.height(),
            ffmpeg::software::scaling::Flags::BILINEAR,
        )?;

        let mut video_frame = ffmpeg::frame::Video::empty();
        let mut rgba_frame = ffmpeg::frame::Video::empty();

        let base_frame_delay_ms = {
            let fps = f64::from(
                ictx.stream(video_stream_index)
                    .map(|s| s.avg_frame_rate())
                    .unwrap_or(ffmpeg::Rational::new(25, 1)),
            );
            if fps > 0.0 {
                (1000.0 / fps).round() as u64
            } else {
                33
            }
        };

        'packet: for (stream, packet) in ictx.packets() {
            while let Ok(cmd) = cmd_rx.try_recv() {
                match cmd {
                    DecoderCmd::Stop => break 'outer,
                    DecoderCmd::SetLoop(l) => looping = l,
                    DecoderCmd::SetSpeed(s) => speed = s.max(0.1),
                    DecoderCmd::Play => {
                        playing.store(true, Ordering::Release);
                    }
                    DecoderCmd::Seek(secs) => {
                        seek_to = Some(secs);
                        break 'packet;
                    }
                    DecoderCmd::Pause => {
                        if wait_while_paused(
                            &playing,
                            &cmd_rx,
                            &mut looping,
                            &mut speed,
                            &mut seek_to,
                        ) == PauseResult::Stop
                        {
                            break 'outer;
                        }
                        if seek_to.is_some() {
                            break 'packet;
                        }
                    }
                }
            }

            if stream.index() != video_stream_index {
                continue;
            }

            if decoder.send_packet(&packet).is_err() {
                continue;
            }

            while decoder.receive_frame(&mut video_frame).is_ok() {
                if scaler.run(&video_frame, &mut rgba_frame).is_err() {
                    continue;
                }

                let pts_secs = video_frame
                    .pts()
                    .map(|pts| pts as f64 * time_base_f64)
                    .unwrap_or(0.0);

                let data = pack_rows(
                    rgba_frame.data(0),
                    rgba_frame.stride(0),
                    rgba_frame.width(),
                    rgba_frame.height(),
                );
                let width = rgba_frame.width();
                let height = rgba_frame.height();

                if let Ok(mut guard) = frame_arc.lock() {
                    *guard = Some(RgbaFrame {
                        data,
                        width,
                        height,
                        pts_secs,
                    });
                }
                if let Ok(mut pos) = position_arc.lock() {
                    *pos = pts_secs;
                }

                let delay = ((base_frame_delay_ms as f64) / speed).round() as u64;
                thread::sleep(Duration::from_millis(delay.max(1)));

                let mut seek_in_frame = false;
                while let Ok(cmd) = cmd_rx.try_recv() {
                    match cmd {
                        DecoderCmd::Stop => break 'outer,
                        DecoderCmd::SetLoop(l) => looping = l,
                        DecoderCmd::SetSpeed(s) => speed = s.max(0.1),
                        DecoderCmd::Play => {
                            playing.store(true, Ordering::Release);
                        }
                        DecoderCmd::Seek(secs) => {
                            seek_to = Some(secs);
                            seek_in_frame = true;
                        }
                        DecoderCmd::Pause => {
                            if wait_while_paused(
                                &playing,
                                &cmd_rx,
                                &mut looping,
                                &mut speed,
                                &mut seek_to,
                            ) == PauseResult::Stop
                            {
                                break 'outer;
                            }
                            if seek_to.is_some() {
                                seek_in_frame = true;
                            }
                        }
                    }
                }
                if seek_in_frame {
                    break 'packet;
                }
            }

            if seek_to.is_some() {
                break 'packet;
            }
        }

        if looping || seek_to.is_some() {
            continue 'outer;
        } else {
            // The clip ran to its end: the transport has to open the file
            // again to play it a second time.
            playing.store(false, Ordering::Release);
            ended.store(true, Ordering::Release);
            break 'outer;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The error slot must start empty and must report each stored failure
    /// exactly once, so a decoder fault surfaces to the user a single time.
    #[test]
    fn take_error_drains_once() {
        let slot = Arc::new(Mutex::new(None));
        assert_eq!(slot.lock().unwrap().take(), None);

        *slot.lock().unwrap() = Some("stream not found".to_string());
        assert_eq!(
            slot.lock().unwrap().take(),
            Some("stream not found".to_string())
        );
        assert_eq!(slot.lock().unwrap().take(), None);
    }

    /// The rows of a decoded frame are copied out of the decoder's padding.
    ///
    /// The decoder aligns every row to a 32-byte boundary, so the rows of a
    /// frame whose width is not a multiple of eight pixels do not follow one
    /// another. Reading the frame as a tight buffer would draw it skewed.
    #[test]
    fn the_rows_are_packed() {
        // A three-pixel-wide frame: three pixels are twelve bytes, and the
        // decoder pads the row to sixteen.
        const WIDTH: u32 = 3;
        const HEIGHT: u32 = 2;
        const STRIDE: usize = 16;
        let mut padded = Vec::with_capacity(STRIDE * HEIGHT as usize);
        for row in 0..HEIGHT as usize {
            for byte in 0..12u8 {
                padded.push((row as u8) * 10 + byte);
            }
            // The padding of this row.
            padded.extend_from_slice(&[0xFF; STRIDE - 12]);
        }

        let packed = pack_rows(&padded, STRIDE, WIDTH, HEIGHT);
        assert_eq!(packed.len(), 12 * HEIGHT as usize, "the frame is tight");
        assert_eq!(
            &packed[0..12],
            &padded[0..12],
            "the first row keeps its pixels"
        );
        assert_eq!(
            &packed[12..24],
            &padded[STRIDE..STRIDE + 12],
            "the second row starts where its own pixels start"
        );
        assert!(
            !packed[12..24].contains(&0xFF),
            "the padding of the first row is not mistaken for pixels"
        );
    }

    /// A frame that is shorter than it claims is padded, not truncated: a
    /// short row would shift every row after it.
    #[test]
    fn a_short_frame_is_padded() {
        let packed = pack_rows(&[1, 2, 3, 4], 4, 2, 2);
        assert_eq!(packed.len(), 16, "the frame is the size it claims");
        assert_eq!(&packed[0..4], &[1, 2, 3, 4]);
        assert_eq!(&packed[4..8], &[0, 0, 0, 0], "the missing row is padded");
    }
}
