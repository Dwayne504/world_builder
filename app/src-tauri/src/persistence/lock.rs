//! Exclusive Project lock with an ownership lease heartbeat.

use std::fs::{self, OpenOptions};
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
    fn new_for(project_id: ProjectId) -> Self {
        let now = Utc::now();
        Self {
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

    pub fn is_stale_with(&self, threshold: Duration) -> bool {
        Utc::now()
            .signed_duration_since(self.heartbeat_at)
            .to_std()
            .unwrap_or(Duration::ZERO)
            > threshold
    }

    pub fn is_stale(&self) -> bool {
        self.is_stale_with(STALE_LOCK_THRESHOLD)
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

#[derive(Debug)]
pub struct LockGuard {
    path: PathBuf,
    session_token: Uuid,
    stop: Option<Sender<()>>,
    heartbeat: Option<JoinHandle<()>>,
}

impl LockGuard {
    pub fn release(mut self) {
        self.stop_heartbeat();
        self.remove_if_owned();
    }

    fn stop_heartbeat(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.heartbeat.take() {
            let _ = thread.join();
        }
    }

    fn remove_if_owned(&self) {
        if matches!(read_lock(&self.path), Ok(info) if info.session_token == self.session_token) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        self.stop_heartbeat();
        self.remove_if_owned();
    }
}

fn read_lock(path: &Path) -> Result<LockInfo, LockError> {
    let raw = fs::read_to_string(path)?;
    serde_json::from_str(&raw).map_err(|e| LockError::Corrupt(e.to_string()))
}

fn write_new_lock(path: &Path, info: &LockInfo) -> Result<(), LockError> {
    let json = serde_json::to_string_pretty(info).map_err(|e| LockError::Corrupt(e.to_string()))?;
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(json.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

fn refresh_heartbeat(path: &Path, token: Uuid) {
    let Ok(mut info) = read_lock(path) else {
        return;
    };
    if info.session_token != token {
        return;
    }
    info.heartbeat_at = Utc::now();
    let Ok(json) = serde_json::to_string_pretty(&info) else {
        return;
    };
    // The guard owns this lock. Truncating in place avoids replacement semantics
    // that differ on Windows; an interrupted write is an explicit corrupt lock.
    if let Ok(mut file) = OpenOptions::new().write(true).truncate(true).open(path) {
        let _ = file.write_all(json.as_bytes());
        let _ = file.sync_all();
    }
}

fn acquire_with_timing(
    lock_path: &Path,
    project_id: ProjectId,
    interval: Duration,
) -> Result<LockGuard, LockError> {
    let info = LockInfo::new_for(project_id);
    match write_new_lock(lock_path, &info) {
        Ok(()) => {
            let (stop_tx, stop_rx) = mpsc::channel();
            let path = lock_path.to_path_buf();
            let token = info.session_token;
            let heartbeat = thread::spawn(move || {
                while stop_rx.recv_timeout(interval).is_err() {
                    refresh_heartbeat(&path, token);
                }
            });
            Ok(LockGuard {
                path: lock_path.to_path_buf(),
                session_token: info.session_token,
                stop: Some(stop_tx),
                heartbeat: Some(heartbeat),
            })
        }
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

pub fn acquire(lock_path: &Path, project_id: ProjectId) -> Result<LockGuard, LockError> {
    acquire_with_timing(lock_path, project_id, HEARTBEAT_INTERVAL)
}

pub fn inspect(lock_path: &Path) -> Result<Option<LockInfo>, LockError> {
    if !lock_path.exists() {
        return Ok(None);
    }
    read_lock(lock_path).map(Some)
}

pub fn recover_stale_lock(lock_path: &Path) -> Result<(), LockError> {
    recover_stale_lock_with(lock_path, STALE_LOCK_THRESHOLD)
}

fn recover_stale_lock_with(lock_path: &Path, threshold: Duration) -> Result<(), LockError> {
    // A separate exclusive recovery claim serializes check-and-remove across
    // platforms and ensures recovery never removes a replacement lock.
    let claim = lock_path.with_extension(format!("recover-{}", Uuid::new_v4()));
    let _claim = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&claim)?;
    let result = (|| {
        let existing = read_lock(lock_path)?;
        if !existing.is_stale_with(threshold) {
            return Err(LockError::NotStale);
        }
        let token = existing.session_token;
        if matches!(read_lock(lock_path), Ok(current) if current.session_token == token) {
            fs::remove_file(lock_path)?;
            Ok(())
        } else {
            Err(LockError::NotStale)
        }
    })();
    let _ = fs::remove_file(claim);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn heartbeat_keeps_an_old_active_lock_fresh_and_stops_on_release() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("lock.json");
        let guard =
            acquire_with_timing(&path, ProjectId::new(), Duration::from_millis(10)).unwrap();
        std::thread::sleep(Duration::from_millis(35));
        assert!(recover_stale_lock_with(&path, Duration::from_millis(20)).is_err());
        guard.release();
        assert!(!path.exists());
    }

    #[test]
    fn release_never_removes_replacement_owned_by_another_session() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("lock.json");
        let guard = acquire_with_timing(&path, ProjectId::new(), Duration::from_secs(60)).unwrap();
        let mut stale = read_lock(&path).unwrap();
        stale.heartbeat_at = Utc::now() - chrono::Duration::hours(1);
        fs::write(&path, serde_json::to_vec(&stale).unwrap()).unwrap();
        recover_stale_lock_with(&path, Duration::from_millis(1)).unwrap();
        let replacement =
            acquire_with_timing(&path, ProjectId::new(), Duration::from_secs(60)).unwrap();
        drop(guard);
        assert!(path.exists());
        assert!(matches!(
            acquire(&path, ProjectId::new()),
            Err(LockError::Held { .. })
        ));
        replacement.release();
    }

    #[test]
    fn only_one_competing_stale_recovery_succeeds() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("lock.json");
        let mut stale = LockInfo::new_for(ProjectId::new());
        stale.heartbeat_at = Utc::now() - chrono::Duration::hours(1);
        fs::write(&path, serde_json::to_vec(&stale).unwrap()).unwrap();
        let a = path.clone();
        let b = path.clone();
        let first =
            thread::spawn(move || recover_stale_lock_with(&a, Duration::from_millis(1)).is_ok());
        let second =
            thread::spawn(move || recover_stale_lock_with(&b, Duration::from_millis(1)).is_ok());
        assert_eq!(
            first.join().unwrap() as u8 + second.join().unwrap() as u8,
            1
        );
    }
}
