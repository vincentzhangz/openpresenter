//! Crash-safe file writing.
//!
//! The settings and the prop library are small files the app rewrites
//! whenever the operator changes something. Writing them in place means an
//! interrupted write — a crash, a power loss, a kill — can leave a truncated
//! file behind, and the next start would silently fall back to the defaults.
//! Writing to a temporary file and renaming it over the target avoids that:
//! the rename is atomic, so the file is either the old one or the new one.

use std::path::Path;

/// Write a file atomically.
///
/// The content is written to a temporary file beside the target and then
/// renamed over it. A failure leaves the original file untouched.
pub(crate) fn write_atomic(path: &Path, contents: &[u8]) -> anyhow::Result<()> {
    let mut cursor = std::io::Cursor::new(contents);
    write_atomic_from(path, &mut cursor)
}

/// Write a file atomically from a reader.
///
/// The content is copied to a temporary file beside the target and then
/// renamed over it, so a failure leaves the original file untouched. The
/// content is streamed rather than held in memory, so a large file does not
/// have to fit in RAM.
pub(crate) fn write_atomic_from(
    path: &Path,
    source: &mut impl std::io::Read,
) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "config".to_string());
    let temp = path.with_file_name(format!("{file_name}.tmp"));

    let result = (|| -> anyhow::Result<()> {
        let mut file = std::fs::File::create(&temp)?;
        std::io::copy(source, &mut file)?;
        file.sync_all()?;
        Ok(())
    })();
    if let Err(e) = result {
        // The write failed: do not leave the temporary file behind.
        let _ = std::fs::remove_file(&temp);
        return Err(e);
    }

    match std::fs::rename(&temp, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            // The rename failed: do not leave the temporary file behind.
            let _ = std::fs::remove_file(&temp);
            Err(e.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_write_leaves_no_temporary_file_behind() {
        let dir = std::env::temp_dir().join("openpresenter-atomic-test");
        std::fs::create_dir_all(&dir).expect("the temp directory exists");
        let path = dir.join("settings.toml");

        write_atomic(&path, b"first").expect("the file is written");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "first");

        // A second write replaces the content, and the temporary file is gone.
        write_atomic(&path, b"second").expect("the file is rewritten");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");
        assert!(!dir.join("settings.toml.tmp").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
