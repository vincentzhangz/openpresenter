//! The log the app leaves behind.
//!
//! The app writes to the terminal as well, but a terminal is gone when the
//! app is closed — and the operator who reports "it stopped working" has
//! nothing to send. The log file is what is read then.

use std::io::Write;
use std::path::PathBuf;

/// The largest the log is allowed to grow before it is rolled over.
const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;

/// The directory the log is written to.
///
/// Each platform has its own place for logs, and the operator looks there.
#[cfg(target_os = "macos")]
fn log_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("Library")
        .join("Logs")
        .join("openpresenter")
}

#[cfg(target_os = "windows")]
fn log_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("openpresenter")
        .join("logs")
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn log_dir() -> PathBuf {
    dirs::state_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("openpresenter")
        .join("logs")
}

/// The file the log is written to.
pub fn log_file_path() -> PathBuf {
    log_dir().join("openpresenter.log")
}

/// Open the log file, rolling the old one over when it has grown too big.
///
/// A log that grows without limit fills the disk of a machine that has been
/// running the app for months.
fn open_log_file(path: &std::path::Path) -> Option<std::fs::File> {
    if let Some(parent) = path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        eprintln!("cannot create the log directory: {e}");
        return None;
    }
    // The old log is rolled over rather than deleted: the failure before the
    // current one is often the interesting one.
    if let Ok(meta) = std::fs::metadata(path)
        && meta.len() > MAX_LOG_BYTES
    {
        let old = path.with_extension("log.1");
        let _ = std::fs::remove_file(&old);
        let _ = std::fs::rename(path, old);
    }
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        Ok(file) => Some(file),
        Err(e) => {
            eprintln!("cannot open the log file: {e}");
            None
        }
    }
}

/// A writer that writes to the terminal and to the log file.
struct Tee {
    terminal: std::io::Stderr,
    file: std::fs::File,
}

impl Write for Tee {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // The terminal is best effort: a full disk must not stop the log.
        let _ = self.terminal.write_all(buf);
        self.file.write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let _ = self.terminal.flush();
        self.file.flush()
    }
}

/// Start the logger: the terminal and the log file.
pub fn init_logger() {
    let path = log_file_path();
    let file = open_log_file(&path);
    let mut builder =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
    match file {
        Some(file) => {
            builder.target(env_logger::Target::Pipe(Box::new(Tee {
                terminal: std::io::stderr(),
                file,
            })));
        }
        None => {
            builder.target(env_logger::Target::Stderr);
        }
    }
    builder.init();
    log::info!("the log is at {}", path.display());
}
