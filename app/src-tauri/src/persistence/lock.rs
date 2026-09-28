//! An OS-held advisory lock is authoritative; `lock.json` is recoverable metadata.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::ProjectId;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockInfo {
    pub project_id: ProjectId,
    pub pid: u32,
    pub host: String,
    pub session_token: Uuid,
    pub acquired_at: DateTime<Utc>,
    pub heartbeat_at: DateTime<Utc>,
}
impl LockInfo {
    /// Visible to integration tests so they can plant orphaned metadata;
    /// production callers always go through `acquire`.
    #[doc(hidden)]
    pub fn new(project_id: ProjectId) -> Self {
        let now = Utc::now();
        Self {
            project_id,
            pid: std::process::id(),
            host: hostname::get()
                .ok()
                .and_then(|v| v.into_string().ok())
                .unwrap_or_else(|| "unknown-host".into()),
            session_token: Uuid::new_v4(),
            acquired_at: now,
            heartbeat_at: now,
        }
    }
}
#[derive(Debug, Error)]
pub enum LockError {
    #[error("Project is already open (pid {pid} on {host}, held since {acquired_at})")]
    Held {
        pid: u32,
        host: String,
        acquired_at: DateTime<Utc>,
    },
    #[error("a previous session ended without releasing the Project (for example after a crash or power loss); the OS lock is free, so you can explicitly recover the Project")]
    RecoveryRequired,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("the leftover lock metadata is unreadable or corrupt ({0}); it was left untouched")]
    Corrupt(String),
}

pub struct LockGuard {
    metadata_path: PathBuf,
    guard_file: File,
    token: Uuid,
    stop: Option<Sender<()>>,
    heartbeat: Option<JoinHandle<()>>,
}
impl std::fmt::Debug for LockGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LockGuard").finish()
    }
}
impl LockGuard {
    pub fn release(mut self) {
        self.stop();
        self.remove_owned_metadata();
        let _ = self.guard_file.unlock();
    }
    fn stop(&mut self) {
        if let Some(tx) = self.stop.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.heartbeat.take() {
            let _ = handle.join();
        }
    }
    fn remove_owned_metadata(&self) {
        if matches!(read_metadata(&self.metadata_path), Ok(info) if info.session_token == self.token)
        {
            let _ = fs::remove_file(&self.metadata_path);
        }
    }
}
impl Drop for LockGuard {
    fn drop(&mut self) {
        self.stop();
        self.remove_owned_metadata();
        let _ = self.guard_file.unlock();
    }
}

fn guard_path(metadata_path: &Path) -> PathBuf {
    metadata_path.with_extension("guard")
}
fn read_metadata(path: &Path) -> Result<LockInfo, LockError> {
    serde_json::from_str(&fs::read_to_string(path)?).map_err(|e| LockError::Corrupt(e.to_string()))
}
fn write_metadata(path: &Path, info: &LockInfo) -> Result<(), LockError> {
    let temp = path.with_extension(format!("json.{}.tmp", Uuid::new_v4()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    file.write_all(
        serde_json::to_string_pretty(info)
            .map_err(|e| LockError::Corrupt(e.to_string()))?
            .as_bytes(),
    )?;
    file.sync_all()?;
    // Metadata is never authoritative; preserving an old valid copy is safer
    // than truncating it. Windows replacement is handled by manifest protocol.
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    fs::rename(temp, path)?;
    Ok(())
}
fn heartbeat(path: &Path, token: Uuid) {
    if let Ok(mut info) = read_metadata(path) {
        if info.session_token == token {
            info.heartbeat_at = Utc::now();
            let _ = write_metadata(path, &info);
        }
    }
}

pub fn acquire(
    lock_path: &Path,
    project_id: ProjectId,
    recover_stale: bool,
) -> Result<LockGuard, LockError> {
    // Decision order matters:
    // 1. The OS advisory guard is authoritative -- first prove it is not
    //    actively held by another instance. An active lock is never stolen
    //    or bypassed, not even with `recover_stale`, and never offered for
    //    recovery.
    // 2. Only once the guard is ours, read and validate the heartbeat
    //    metadata (never authoritative, only evidence about a previous
    //    session).
    // 3. Any readable leftover metadata, regardless of its age, means a
    //    previous session ended without releasing the Project: explicit
    //    recovery is immediately available. There is no waiting period --
    //    recovery is never automatic, and a normal (non-recovering) open
    //    attempt never alters or refreshes the leftover metadata.
    let guard = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(guard_path(lock_path))?;
    if let Err(error) = guard.try_lock() {
        return Err(if matches!(error, std::fs::TryLockError::WouldBlock) {
            // The OS lock is actively held: never steal it. Ownership
            // details come from the heartbeat metadata when readable, but a
            // missing/corrupt file must not downgrade the report -- the
            // authoritative fact is the actively held OS lock itself.
            match read_metadata(lock_path) {
                Ok(info) => LockError::Held {
                    pid: info.pid,
                    host: info.host,
                    acquired_at: info.acquired_at,
                },
                Err(_) => LockError::Held {
                    pid: 0,
                    host: "unknown-host".to_string(),
                    acquired_at: DateTime::<Utc>::UNIX_EPOCH,
                },
            }
        } else {
            LockError::Io(error.into())
        });
    }
    if lock_path.exists() {
        // Validate the metadata is at least readable evidence of a
        // previous session; its content (including age) never gates
        // whether recovery is offered.
        if let Err(error) = read_metadata(lock_path) {
            // Corrupt/unreadable metadata is never deleted: failing
            // safely preserves the evidence for manual inspection.
            let _ = guard.unlock();
            return Err(error);
        }
        if !recover_stale {
            // The leftover file stays exactly where it is until the caller
            // explicitly re-runs with recovery enabled.
            let _ = guard.unlock();
            return Err(LockError::RecoveryRequired);
        }
    }
    let info = LockInfo::new(project_id);
    write_metadata(lock_path, &info)?;
    let (tx, rx) = mpsc::channel();
    let path = lock_path.to_owned();
    let token = info.session_token;
    let thread = thread::spawn(move || {
        while rx.recv_timeout(HEARTBEAT_INTERVAL).is_err() {
            heartbeat(&path, token);
        }
    });
    Ok(LockGuard {
        metadata_path: lock_path.to_owned(),
        guard_file: guard,
        token,
        stop: Some(tx),
        heartbeat: Some(thread),
    })
}

pub fn inspect(lock_path: &Path) -> Result<Option<LockInfo>, LockError> {
    if lock_path.exists() {
        read_metadata(lock_path).map(Some)
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use tempfile::tempdir;

    #[test]
    fn competing_acquisition_has_one_owner_for_the_guard_lifetime() {
        let dir = tempdir().unwrap();
        let path = Arc::new(dir.path().join("lock.json"));
        let barrier = Arc::new(Barrier::new(2));
        let contender_path = path.clone();
        let contender_barrier = barrier.clone();
        let contender = thread::spawn(move || {
            contender_barrier.wait();
            acquire(&contender_path, ProjectId::new(), false)
        });
        let owner = acquire(&path, ProjectId::new(), false).unwrap();
        barrier.wait();
        assert!(matches!(
            contender.join().unwrap(),
            Err(LockError::Held { .. })
        ));
        owner.release();
        assert!(acquire(&path, ProjectId::new(), false).is_ok());
    }

    #[test]
    fn leftover_metadata_cannot_recover_while_an_os_owner_is_active() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("lock.json");
        let owner = acquire(&path, ProjectId::new(), false).unwrap();
        assert!(matches!(
            acquire(&path, ProjectId::new(), true),
            Err(LockError::Held { .. })
        ));
        owner.release();
    }

    #[test]
    fn fresh_leftover_metadata_without_an_os_owner_offers_immediate_recovery() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("lock.json");
        // Orphaned metadata from a crashed session with a heartbeat only
        // moments old. Recovery must not require any waiting period.
        let fresh = LockInfo::new(ProjectId::new());
        write_metadata(&path, &fresh).unwrap();

        assert!(matches!(
            acquire(&path, ProjectId::new(), false),
            Err(LockError::RecoveryRequired)
        ));
        // A normal (non-recovering) open attempt must not alter or refresh
        // the leftover metadata.
        let after = read_metadata(&path).unwrap();
        assert_eq!(after.session_token, fresh.session_token);

        let project_id = ProjectId::new();
        let guard = acquire(&path, project_id, true).unwrap();
        let recovered = read_metadata(&path).unwrap();
        assert_eq!(recovered.project_id, project_id);
        assert_ne!(recovered.session_token, fresh.session_token);
        guard.release();
        assert!(!path.exists());
    }

    #[test]
    fn old_leftover_metadata_without_an_os_owner_requires_explicit_recovery() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("lock.json");
        let mut orphan = LockInfo::new(ProjectId::new());
        orphan.heartbeat_at = Utc::now() - chrono::Duration::hours(2);
        write_metadata(&path, &orphan).unwrap();

        // Normal open reports that explicit recovery is available...
        assert!(matches!(
            acquire(&path, ProjectId::new(), false),
            Err(LockError::RecoveryRequired)
        ));
        // ...and must not have removed or rewritten the stale file.
        let after = read_metadata(&path).unwrap();
        assert_eq!(after.session_token, orphan.session_token);

        // The explicit recovery acquires the lock and replaces metadata.
        let project_id = ProjectId::new();
        let guard = acquire(&path, project_id, true).unwrap();
        let recovered = read_metadata(&path).unwrap();
        assert_eq!(recovered.project_id, project_id);
        assert_ne!(recovered.session_token, orphan.session_token);
        guard.release();
        // A clean release removes the metadata it owned.
        assert!(!path.exists());
    }

    #[test]
    fn corrupt_metadata_without_an_os_owner_fails_safely_and_is_preserved() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("lock.json");
        fs::write(&path, b"{ this is not json").unwrap();

        for recover in [false, true] {
            let result = acquire(&path, ProjectId::new(), recover);
            assert!(
                matches!(result, Err(LockError::Corrupt(_))),
                "expected Corrupt for recover={recover}, got {result:?}"
            );
        }
        // The unreadable file is evidence, not debris: never deleted.
        assert_eq!(fs::read(&path).unwrap(), b"{ this is not json");
        // Our failed attempts did not leave the OS guard held.
        let guard = acquire_guard_only(&path);
        drop(guard);
    }

    #[test]
    fn corrupt_metadata_with_an_active_os_owner_reports_held_not_corrupt() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("lock.json");
        let owner = acquire(&path, ProjectId::new(), false).unwrap();
        // The owner crashed hard enough to corrupt its own metadata, yet
        // still holds the OS advisory lock (e.g. another instance did the
        // damage). Ownership must never be inferred from metadata alone.
        fs::write(&path, b"garbage").unwrap();

        for recover in [false, true] {
            assert!(matches!(
                acquire(&path, ProjectId::new(), recover),
                Err(LockError::Held { .. })
            ));
        }
        assert_eq!(fs::read(&path).unwrap(), b"garbage");
        owner.release();
    }

    /// Acquires only the OS advisory guard file, bypassing metadata
    /// handling, to prove a previous failure did not leave it held.
    fn acquire_guard_only(lock_path: &Path) -> File {
        let guard = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(guard_path(lock_path))
            .unwrap();
        guard.try_lock().unwrap();
        guard
    }
}
