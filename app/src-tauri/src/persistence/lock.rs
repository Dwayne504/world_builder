//! Exclusive Project lock.
//!
//! `lock.json` lives at the package root (not inside `data/`) so it is
//! trivially excluded from portable backups: it is operational state about
//! *this machine/process*, never authored content.
//!
//! Cross-platform notes (engineering spike): this slice detects staleness
//! purely by lock age plus an explicit recovery step, rather than by
//! probing whether the owning process is still alive. Checking liveness of
//! a PID cross-platform (especially across restarts, containers, and
//! networked/cloud-synced filesystems) is unreliable, so age-based staleness
//! plus an explicit user-confirmed recovery action is the safer default.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::ProjectId;

/// A lock older than this without being refreshed is eligible for explicit
/// stale-lock recovery. Ordinary session lifetimes are far shorter.
pub const STALE_LOCK_THRESHOLD: Duration = Duration::from_secs(30 * 60);

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
    fn new_for(project_id: ProjectId) -> Self {
        let now = Utc::now();
        LockInfo {
            project_id,
            pid: std::process::id(),
            host: hostname::get()
                .ok()
                .and_then(|s| s.into_string().ok())
                .unwrap_or_else(|| "unknown-host".to_string()),
            session_token: Uuid::new_v4(),
            acquired_at: now,
            heartbeat_at: now,
        }
    }

    pub fn age(&self) -> chrono::Duration {
        Utc::now().signed_duration_since(self.heartbeat_at)
    }

    pub fn is_stale(&self) -> bool {
        self.age().to_std().unwrap_or(Duration::ZERO) > STALE_LOCK_THRESHOLD
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

    #[error("lock is not stale enough to recover automatically")]
    NotStale,

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("lock file is present but unreadable/corrupt: {0}")]
    Corrupt(String),
}

/// An acquired lock. Dropping/`release`-ing it removes `lock.json`.
#[derive(Debug)]
pub struct LockGuard {
    path: PathBuf,
    released: bool,
}

impl LockGuard {
    pub fn release(mut self) {
        self.do_release();
    }

    fn do_release(&mut self) {
        if !self.released {
            let _ = fs::remove_file(&self.path);
            self.released = true;
        }
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        self.do_release();
    }
}

fn read_lock(path: &Path) -> Result<LockInfo, LockError> {
    let raw = fs::read_to_string(path)?;
    serde_json::from_str(&raw).map_err(|e| LockError::Corrupt(e.to_string()))
}

fn write_lock(path: &Path, info: &LockInfo) -> Result<(), LockError> {
    let json = serde_json::to_string_pretty(info)
        .map_err(|e| LockError::Corrupt(format!("failed to serialize lock: {e}")))?;
    // Atomic create: fails if another process created it first.
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(json.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

/// Attempts to acquire the exclusive Project lock. Fails with
/// [`LockError::Held`] if another live-looking lock is present; callers who
/// need stale-lock recovery must call [`recover_stale_lock`] first.
pub fn acquire(lock_path: &Path, project_id: ProjectId) -> Result<LockGuard, LockError> {
    let info = LockInfo::new_for(project_id);
    match write_lock(lock_path, &info) {
        Ok(()) => Ok(LockGuard {
            path: lock_path.to_path_buf(),
            released: false,
        }),
        Err(LockError::Io(e)) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let existing = read_lock(lock_path)?;
            Err(LockError::Held {
                pid: existing.pid,
                host: existing.host,
                acquired_at: existing.acquired_at,
            })
        }
        Err(other) => Err(other),
    }
}

/// Reads the current lock, if any, without acquiring it.
pub fn inspect(lock_path: &Path) -> Result<Option<LockInfo>, LockError> {
    if !lock_path.exists() {
        return Ok(None);
    }
    read_lock(lock_path).map(Some)
}

/// Explicitly removes a confirmed-stale lock so the Project can be opened.
/// This is never invoked automatically: recovery is a deliberate operation.
pub fn recover_stale_lock(lock_path: &Path) -> Result<(), LockError> {
    let existing = read_lock(lock_path)?;
    if !existing.is_stale() {
        return Err(LockError::NotStale);
    }
    fs::remove_file(lock_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn second_acquire_is_refused_while_first_is_held() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("lock.json");
        let project_id = ProjectId::new();
        let guard = acquire(&path, project_id).unwrap();
        let err = acquire(&path, project_id).unwrap_err();
        assert!(matches!(err, LockError::Held { .. }));
        guard.release();
        // Now that it is released, acquiring again must succeed.
        acquire(&path, project_id).unwrap();
    }

    #[test]
    fn release_removes_lock_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("lock.json");
        let guard = acquire(&path, ProjectId::new()).unwrap();
        assert!(path.exists());
        guard.release();
        assert!(!path.exists());
    }

    #[test]
    fn stale_lock_requires_explicit_recovery() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("lock.json");
        let mut info = LockInfo::new_for(ProjectId::new());
        info.heartbeat_at = Utc::now() - chrono::Duration::hours(2);
        write_lock(&path, &info).unwrap();

        // A fresh (non-stale) view would refuse recovery.
        let err = recover_stale_lock(&path);
        assert!(err.is_ok(), "an aged lock past the threshold recovers");

        // Once recovered, the lock file is gone and can be reacquired.
        assert!(!path.exists());
        acquire(&path, ProjectId::new()).unwrap();
    }

    #[test]
    fn fresh_lock_refuses_stale_recovery() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("lock.json");
        let guard = acquire(&path, ProjectId::new()).unwrap();
        let err = recover_stale_lock(&path).unwrap_err();
        assert!(matches!(err, LockError::NotStale));
        guard.release();
    }
}
