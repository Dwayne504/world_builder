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

pub const STALE_LOCK_THRESHOLD: Duration = Duration::from_secs(30 * 60);
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
    fn new(project_id: ProjectId) -> Self {
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
    pub fn is_stale(&self) -> bool {
        Utc::now()
            .signed_duration_since(self.heartbeat_at)
            .to_std()
            .unwrap_or_default()
            > STALE_LOCK_THRESHOLD
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
    #[error("a stale lock requires explicit recovery")]
    RecoveryRequired,
    #[error("lock is not stale enough to recover")]
    NotStale,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("lock file is present but unreadable/corrupt: {0}")]
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
    let guard = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(guard_path(lock_path))?;
    if let Err(error) = guard.try_lock() {
        let info = read_metadata(lock_path)?;
        return Err(if matches!(error, std::fs::TryLockError::WouldBlock) {
            LockError::Held {
                pid: info.pid,
                host: info.host,
                acquired_at: info.acquired_at,
            }
        } else {
            LockError::Io(error.into())
        });
    }
    if lock_path.exists() {
        let previous = read_metadata(lock_path)?;
        if !recover_stale {
            let _ = guard.unlock();
            return Err(LockError::RecoveryRequired);
        }
        if !previous.is_stale() {
            let _ = guard.unlock();
            return Err(LockError::NotStale);
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
    fn stale_metadata_cannot_recover_while_an_os_owner_is_active() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("lock.json");
        let owner = acquire(&path, ProjectId::new(), false).unwrap();
        let mut metadata = read_metadata(&path).unwrap();
        metadata.heartbeat_at = Utc::now() - chrono::Duration::hours(1);
        write_metadata(&path, &metadata).unwrap();
        assert!(matches!(
            acquire(&path, ProjectId::new(), true),
            Err(LockError::Held { .. })
        ));
        owner.release();
    }
}
