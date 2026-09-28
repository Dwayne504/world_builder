//! Shared Windows-safe, recoverable "publish a whole file" protocol used by
//! every small JSON file this crate replaces in place (currently
//! `manifest.json` and the application preferences file).
//!
//! Direct replacement with `std::fs::rename` is attempted first. Where
//! replacement fails (including Windows sharing/filesystem constraints), a
//! successor is written and fsynced to a sibling `.next` file, validated by
//! reading it back, and only then is any existing destination moved aside
//! to a sibling `.previous` file immediately before the successor is
//! published. A crash at any point during publication leaves one of three
//! recoverable states on disk: the original file untouched, a valid
//! `.previous` plus a valid `.next`, or a valid `.previous` with the
//! destination already replaced -- never a half-written destination.
//! [`recover`] repairs missing destinations the next time the file is opened.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// The sibling path holding a not-yet-published successor.
pub fn next_path(path: &Path) -> PathBuf {
    sibling_with_suffix(path, "next")
}

/// The sibling path holding the most recently published prior version,
/// kept only long enough to recover from an interrupted publish.
pub fn previous_path(path: &Path) -> PathBuf {
    sibling_with_suffix(path, "previous")
}

fn sibling_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => path.with_extension(format!("{ext}.{suffix}")),
        None => path.with_extension(suffix),
    }
}

/// Publishes `bytes` at `path`.
///
/// 1. Writes `bytes` to a sibling `.next` file and `fsync`s it.
/// 2. Reads that file back and calls `validate` on the bytes actually on
///    disk; a validation failure removes the unpublished `.next` and
///    leaves `path` completely untouched.
/// 3. Only after successful validation does it attempt to publish: a
///    direct rename first, falling back where needed to moving the
///    existing `path` aside as `.previous` immediately before renaming
///    `.next` into place.
/// 4. Best-effort `fsync`s the parent directory (ignored where the
///    platform does not support it, e.g. Windows).
/// 5. Removes the now-unneeded `.previous` only after successful
///    publication; a failure at any earlier step preserves whatever was
///    previously at `path` (directly, or recoverably via `.previous`).
pub fn publish(path: &Path, bytes: &[u8], validate: impl Fn(&[u8]) -> bool) -> io::Result<()> {
    let next = next_path(path);
    let previous = previous_path(path);
    {
        let mut file = File::create(&next)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    let written = fs::read(&next)?;
    if written != bytes || !validate(&written) {
        let _ = fs::remove_file(&next);
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "successor file failed post-write validation; publication was refused",
        ));
    }
    if let Err(error) = fs::rename(&next, path) {
        if path.exists() {
            // Fallback: an existing destination must be moved aside before
            // the successor can take its place. The prior file remains
            // fully recoverable as `.previous` throughout this step.
            let _ = fs::remove_file(&previous);
            fs::rename(path, &previous)?;
            fs::rename(&next, path)?;
        } else {
            let _ = fs::remove_file(&next);
            return Err(error);
        }
    }
    if let Some(parent) = path.parent() {
        if let Ok(dir) = File::open(parent) {
            let _ = dir.sync_all();
        }
    }
    // Publication is complete; `.previous` is no longer a recovery source.
    // A failure to remove it here is not itself a publish failure.
    let _ = fs::remove_file(&previous);
    Ok(())
}

/// Repairs an interrupted publication: if `path` is missing but a `.next`
/// or `.previous` sibling exists and passes `validate`, promotes it back
/// to `path` (`.next` first, since it is the newer intended content).
/// Never touches an already-present `path`, and never promotes content
/// that fails `validate`.
pub fn recover(path: &Path, validate: impl Fn(&[u8]) -> bool) -> io::Result<()> {
    if path.is_file() {
        return Ok(());
    }
    for candidate in [next_path(path), previous_path(path)] {
        if candidate.is_file() {
            if let Ok(bytes) = fs::read(&candidate) {
                if validate(&bytes) {
                    fs::rename(&candidate, path)?;
                    return Ok(());
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn always_valid(_: &[u8]) -> bool {
        true
    }

    #[test]
    fn first_publish_creates_the_destination_and_no_leftovers() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.json");
        publish(&path, b"first", always_valid).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"first");
        assert!(!next_path(&path).exists());
        assert!(!previous_path(&path).exists());
    }

    #[test]
    fn a_later_publish_replaces_the_destination_with_no_leftovers() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.json");
        publish(&path, b"first", always_valid).unwrap();
        publish(&path, b"second", always_valid).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"second");
        assert!(!next_path(&path).exists());
        assert!(!previous_path(&path).exists());
    }

    #[test]
    fn recovery_promotes_a_valid_next_file_when_the_destination_is_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.json");
        fs::write(next_path(&path), b"recovered from next").unwrap();
        recover(&path, always_valid).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"recovered from next");
        assert!(!next_path(&path).exists());
    }

    #[test]
    fn recovery_promotes_a_valid_previous_file_when_next_is_absent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.json");
        fs::write(previous_path(&path), b"recovered from previous").unwrap();
        recover(&path, always_valid).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"recovered from previous");
        assert!(!previous_path(&path).exists());
    }

    #[test]
    fn recovery_prefers_next_over_previous_when_both_are_present() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.json");
        fs::write(previous_path(&path), b"older").unwrap();
        fs::write(next_path(&path), b"newer").unwrap();
        recover(&path, always_valid).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"newer");
    }

    #[test]
    fn recovery_never_touches_an_already_present_destination() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.json");
        fs::write(&path, b"current").unwrap();
        fs::write(next_path(&path), b"stale leftover").unwrap();
        recover(&path, always_valid).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"current");
    }

    #[test]
    fn recovery_skips_a_candidate_that_fails_validation() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.json");
        fs::write(next_path(&path), b"corrupt").unwrap();
        fs::write(previous_path(&path), b"valid").unwrap();
        recover(&path, |bytes| bytes == b"valid").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"valid");
    }

    #[test]
    fn failed_validation_never_publishes_and_leaves_the_prior_file_intact() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.json");
        publish(&path, b"good", always_valid).unwrap();
        let err = publish(&path, b"bad", |bytes| bytes != b"bad");
        assert!(err.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"good");
        assert!(!next_path(&path).exists());
    }

    #[test]
    fn failed_successor_creation_preserves_the_published_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.json");
        publish(&path, b"prior", always_valid).unwrap();
        fs::create_dir(next_path(&path)).unwrap();
        assert!(publish(&path, b"replacement", always_valid).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"prior");
        assert!(next_path(&path).is_dir());
    }
}
