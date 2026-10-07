//! Native scheduling remains active when the renderer is minimized or reloaded.
use super::{state::OpenProject, AppError, AppState};
use crate::{
    backup_recovery::automatic,
    domain::ProjectId,
    preferences::{self, PreferencesStore},
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{mpsc, Arc, Mutex, Weak},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const INTERVAL: Duration = Duration::from_secs(15 * 60);
const POLL: Duration = Duration::from_secs(5);
const PREFERENCE_ERROR: &str = "Automatic backups are waiting for your application settings to be repaired. Open Settings to review them.";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomaticBackupStatus {
    pub enabled: Option<bool>,
    pub interval_minutes: u32,
    pub retention_count: usize,
    pub running: bool,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_success_revision: Option<i64>,
    pub last_success_path: Option<String>,
    pub backup_directory: Option<String>,
    pub next_due_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}
impl Default for AutomaticBackupStatus {
    fn default() -> Self {
        Self {
            enabled: None,
            interval_minutes: 15,
            retention_count: automatic::RETENTION_COUNT,
            running: false,
            last_success_at: None,
            last_success_revision: None,
            last_success_path: None,
            backup_directory: None,
            next_due_at: None,
            error: None,
        }
    }
}
type Statuses = Arc<Mutex<HashMap<ProjectId, AutomaticBackupStatus>>>;
struct Schedule {
    project: Weak<OpenProject>,
    due: Instant,
    checked_root: Option<PathBuf>,
    backed_up_revision: Option<i64>,
    last_path: Option<PathBuf>,
    cleanup_error: Option<String>,
}

pub struct BackupEngine {
    store: PreferencesStore,
    app_data: PathBuf,
    schedules: HashMap<ProjectId, Schedule>,
    statuses: Statuses,
}
impl BackupEngine {
    fn new(preferences_path: PathBuf, app_data: PathBuf, statuses: Statuses) -> Self {
        Self {
            store: PreferencesStore::new(preferences_path),
            app_data,
            schedules: HashMap::new(),
            statuses,
        }
    }
    fn update(&self, id: ProjectId, change: impl FnOnce(&mut AutomaticBackupStatus)) {
        change(
            self.statuses
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .entry(id)
                .or_default(),
        );
    }
    // Both clocks are supplied so cadence, sleep/resume and retry tests never wait.
    fn tick(&mut self, state: &AppState, now: Instant, wall: DateTime<Utc>) {
        let projects = state
            .open_projects
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        self.schedules.retain(|id, _| projects.contains_key(id));
        self.statuses
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|id, _| projects.contains_key(id));
        if projects.is_empty() {
            return;
        }
        let preferences = self.store.load();
        for (id, open) in projects {
            if self
                .schedules
                .get(&id)
                .is_none_or(|s| s.project.upgrade().is_none_or(|p| !Arc::ptr_eq(&p, &open)))
            {
                self.schedules.insert(
                    id,
                    Schedule {
                        project: Arc::downgrade(&open),
                        due: open.opened_at + INTERVAL,
                        checked_root: None,
                        backed_up_revision: None,
                        last_path: None,
                        cleanup_error: None,
                    },
                );
                self.statuses
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(id, AutomaticBackupStatus::default());
            }
            let prefs = match &preferences {
                Ok(p) => p,
                Err(_) => {
                    self.update(id, |s| {
                        s.enabled = None;
                        s.next_due_at = None;
                        s.error = Some(PREFERENCE_ERROR.into());
                    });
                    continue;
                }
            };
            let root = prefs
                .default_backups_dir
                .clone()
                .unwrap_or_else(|| self.app_data.join("Automatic Backups"));
            let due = self.schedules[&id].due;
            self.update(id, |s| {
                s.enabled = Some(prefs.automatic_backups_enabled);
                if s.error.as_deref() == Some(PREFERENCE_ERROR) {
                    s.error = None;
                }
                s.backup_directory = Some(root.display().to_string());
                s.next_due_at = prefs.automatic_backups_enabled.then(|| {
                    wall + chrono::Duration::from_std(due.saturating_duration_since(now))
                        .unwrap_or_default()
                });
            });
            if !prefs.automatic_backups_enabled {
                continue;
            }
            if now < due {
                continue;
            }
            // Exactly one attempt after any sleep; retry failures at the normal cadence.
            self.schedules.get_mut(&id).unwrap().due = now + INTERVAL;
            self.update(id, |s| {
                s.next_due_at = Some(wall + chrono::Duration::minutes(15))
            });
            // A short queue claim serializes this with manual snapshots and close.
            // A close that already took the worker wins and creates no extra backup.
            let guard = open.worker.lock().unwrap_or_else(|e| e.into_inner());
            let Some(worker) = guard.as_ref() else {
                continue;
            };
            if state
                .open_projects
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&id)
                .is_none_or(|registered| !Arc::ptr_eq(registered, &open))
            {
                continue;
            }
            self.update(id, |s| s.running = true);
            let result = (|| -> Result<Option<automatic::AutomaticSnapshot>, String> {
                if prefs.default_backups_dir.is_none() {
                    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
                }
                preferences::validate_directory(&root).map_err(|e| e.to_string())?;
                let current = worker
                    .read_meta()
                    .map_err(|e| e.to_string())?
                    .last_committed_revision;
                if self.schedules[&id].checked_root.as_ref() != Some(&root) {
                    let latest = automatic::latest(&root, id).map_err(|e| e.to_string())?;
                    let schedule = self.schedules.get_mut(&id).unwrap();
                    schedule.checked_root = Some(root.clone());
                    schedule.backed_up_revision = latest.as_ref().map(|s| s.revision);
                    schedule.last_path = latest.as_ref().map(|s| s.path.clone());
                    if let Some(snapshot) = latest {
                        self.update(id, |s| {
                            s.last_success_at = Some(snapshot.created_at);
                            s.last_success_revision = Some(snapshot.revision);
                            s.last_success_path = Some(snapshot.path.display().to_string());
                        });
                    }
                }
                // Initial empty Projects need no snapshot; every authored revision
                // without a validated recovery point is eligible at the first interval.
                if (current == 0 && open.opened_revision == 0)
                    || (self.schedules[&id].backed_up_revision == Some(current)
                        && self.schedules[&id]
                            .last_path
                            .as_ref()
                            .is_some_and(|path| automatic::is_valid(path, &root, id, current)))
                {
                    return Ok(None);
                }
                automatic::create(worker, &open.paths, &root, wall)
                    .map(Some)
                    .map_err(|e| e.to_string())
            })();
            self.update(id, |s| s.running = false);
            match result {
                Ok(Some(snapshot)) => {
                    let schedule = self.schedules.get_mut(&id).unwrap();
                    schedule.backed_up_revision = Some(snapshot.revision);
                    schedule.last_path = Some(snapshot.path.clone());
                    schedule.cleanup_error = snapshot.cleanup_error.clone();
                    self.update(id, |s| {
                        s.last_success_at = Some(snapshot.created_at);
                        s.last_success_revision = Some(snapshot.revision);
                        s.last_success_path = Some(snapshot.path.display().to_string());
                        s.error = snapshot.cleanup_error;
                    });
                }
                Ok(None) => self.update(id, |s| {
                    // Skipping an unchanged Project is not a successful retry of
                    // failed retention. Keep that warning until a new snapshot
                    // completes cleanup, without creating duplicate backups.
                    s.error = self.schedules[&id].cleanup_error.clone();
                }),
                Err(_) => self.update(id, |s| {
                    s.error = Some("Automatic backup could not finish. Check that the backup folder is available, writable, and has free space. Linked folders cannot be used; choose a regular local folder in Settings if needed. Your Project and earlier backups are unchanged; Worldcrafter will try again in 15 minutes.".into());
                }),
            }
        }
    }
}

/// Managed independently of AppHandle to avoid keeping the desktop alive in a cycle.
pub struct AutomaticBackups {
    statuses: Statuses,
    stop: Mutex<Option<mpsc::Sender<()>>>,
    thread: Mutex<Option<JoinHandle<()>>>,
    app_data: PathBuf,
}
impl AutomaticBackups {
    pub fn start(
        state: AppState,
        preferences_path: PathBuf,
        app_data: PathBuf,
    ) -> std::io::Result<Self> {
        let statuses = Statuses::default();
        let mut engine = BackupEngine::new(preferences_path, app_data.clone(), statuses.clone());
        let (stop, receiver) = mpsc::channel();
        let handle = thread::Builder::new()
            .name("worldcrafter-backups".into())
            .spawn(move || loop {
                match receiver.recv_timeout(POLL) {
                    Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        engine.tick(&state, Instant::now(), Utc::now())
                    }
                }
            })?;
        Ok(Self {
            statuses,
            stop: Mutex::new(Some(stop)),
            thread: Mutex::new(Some(handle)),
            app_data,
        })
    }
    pub fn status(
        &self,
        state: &AppState,
        store: &PreferencesStore,
        id: ProjectId,
    ) -> Result<AutomaticBackupStatus, AppError> {
        let open = state
            .open_projects
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&id)
            .cloned()
            .ok_or(AppError::ProjectNotOpen(id))?;
        let mut status = self
            .statuses
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&id)
            .cloned()
            .unwrap_or_else(|| AutomaticBackupStatus {
                next_due_at: Some(
                    Utc::now()
                        + chrono::Duration::from_std(
                            (open.opened_at + INTERVAL).saturating_duration_since(Instant::now()),
                        )
                        .unwrap_or_default(),
                ),
                ..Default::default()
            });
        match store.load() {
            Ok(prefs) => {
                status.enabled = Some(prefs.automatic_backups_enabled);
                if status.error.as_deref() == Some(PREFERENCE_ERROR) {
                    status.error = None;
                }
                status.backup_directory = Some(
                    prefs
                        .default_backups_dir
                        .unwrap_or_else(|| self.app_data.join("Automatic Backups"))
                        .display()
                        .to_string(),
                );
                if !prefs.automatic_backups_enabled {
                    status.next_due_at = None;
                }
            }
            Err(_) => {
                status.enabled = None;
                status.next_due_at = None;
                status.error = Some(PREFERENCE_ERROR.into());
            }
        }
        Ok(status)
    }
}
impl Drop for AutomaticBackups {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = stop.send(());
        }
        if let Some(handle) = self.thread.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ProjectService;
    use tempfile::{tempdir, TempDir};
    struct Fixture {
        _dir: TempDir,
        state: AppState,
        store: PreferencesStore,
        engine: BackupEngine,
        statuses: Statuses,
        id: ProjectId,
        path: String,
        start: Instant,
        wall: DateTime<Utc>,
        root: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let dir = tempdir().unwrap();
            let state = AppState::default();
            let project =
                ProjectService::create_project(&state, dir.path(), "Scheduler fixture").unwrap();
            let path = dir.path().join("preferences.json");
            let store = PreferencesStore::new(&path);
            let root = dir.path().join("Backups");
            std::fs::create_dir(&root).unwrap();
            store
                .update(|p| p.default_backups_dir = Some(root.clone()))
                .unwrap();
            let statuses = Statuses::default();
            let engine =
                BackupEngine::new(path, dir.path().join("Application Data"), statuses.clone());
            let start = state.open_projects.lock().unwrap()[&project.project_id].opened_at;
            Self {
                _dir: dir,
                state,
                store,
                engine,
                statuses,
                id: project.project_id,
                path: project.package_path,
                start,
                wall: DateTime::from_timestamp(1_800_000_000, 0).unwrap(),
                root,
            }
        }
        fn tick(&mut self, seconds: u64) {
            self.engine.tick(
                &self.state,
                self.start + Duration::from_secs(seconds),
                self.wall + chrono::Duration::seconds(seconds as i64),
            );
        }
        fn change(&self, label: &str) {
            let summary = ProjectService::get_summary(&self.state, self.id).unwrap();
            ProjectService::rename_project(&self.state, self.id, label, summary.revision).unwrap();
        }
        fn status(&self) -> AutomaticBackupStatus {
            self.statuses
                .lock()
                .unwrap()
                .get(&self.id)
                .cloned()
                .unwrap_or_default()
        }
        fn count(&self) -> usize {
            std::fs::read_dir(self.root.join(self.id.to_string()))
                .map(|rows| {
                    rows.flatten()
                        .filter(|e| e.file_name().to_string_lossy().ends_with(".wcbackup"))
                        .count()
                })
                .unwrap_or(0)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = ProjectService::close_project(&self.state, self.id);
        }
    }

    #[test]
    fn cadence_skips_empty_and_unchanged_projects_and_coalesces_sleep() {
        let mut f = Fixture::new();
        f.tick(0);
        f.tick(900);
        assert_eq!(f.count(), 0);
        f.change("Saved edit");
        f.tick(1799);
        assert_eq!(f.count(), 0);
        f.tick(1800);
        assert_eq!(f.count(), 1);
        assert_eq!(f.status().last_success_revision, Some(1));
        f.tick(2700);
        assert_eq!(f.count(), 1);
        f.change("Edited before sleep");
        f.tick(900 * 100);
        assert_eq!(f.count(), 2);
        f.tick(900 * 100 + 1);
        assert_eq!(f.count(), 2);
        assert_eq!(
            f.status().next_due_at,
            Some(f.wall + chrono::Duration::seconds(900 * 101))
        );
    }
    #[test]
    fn restart_uses_only_valid_recovery_points_and_captures_early_session_edits() {
        let mut f = Fixture::new();
        f.change("Before first scheduler poll");
        f.tick(900);
        assert_eq!(f.count(), 1);
        let path = f.status().last_success_path.unwrap();
        f.engine = BackupEngine::new(
            f.store.path().into(),
            f._dir.path().join("Application Data"),
            f.statuses.clone(),
        );
        f.tick(1800);
        assert_eq!(f.count(), 1);
        // Even an unchanged revision needs a new snapshot if its only recovery
        // point was removed or corrupted outside the app.
        std::fs::write(std::path::Path::new(&path).join("manifest.json"), "corrupt").unwrap();
        f.tick(2700);
        assert_eq!(f.count(), 2);
        assert!(f.status().error.is_none());
    }
    #[test]
    fn unchanged_checks_do_not_hide_unresolved_cleanup_failures() {
        let mut f = Fixture::new();
        f.change("First snapshot");
        f.tick(900);
        let warning = "The latest automatic backup is safe, but cleanup failed.".to_string();
        f.engine.schedules.get_mut(&f.id).unwrap().cleanup_error = Some(warning.clone());
        f.engine.update(f.id, |s| s.error = Some(warning.clone()));
        f.tick(1800);
        assert_eq!(f.count(), 1);
        assert_eq!(f.status().error, Some(warning));
        f.change("Next snapshot retries retention");
        f.tick(2700);
        assert_eq!(f.count(), 2);
        assert!(f.status().error.is_none());
    }

    #[test]
    fn toggles_and_new_backup_locations_apply_in_the_same_session() {
        let mut f = Fixture::new();
        f.change("Editing");
        f.store
            .update(|p| p.automatic_backups_enabled = false)
            .unwrap();
        f.tick(900);
        assert_eq!(f.count(), 0);
        assert_eq!(f.status().enabled, Some(false));
        f.store
            .update(|p| p.automatic_backups_enabled = true)
            .unwrap();
        f.tick(901);
        assert_eq!(f.count(), 1);
        let new = f._dir.path().join("New backup location");
        std::fs::create_dir(&new).unwrap();
        f.store
            .update(|p| p.default_backups_dir = Some(new.clone()))
            .unwrap();
        f.tick(902);
        assert_eq!(f.status().backup_directory, Some(new.display().to_string()));
        f.tick(1801);
        assert!(new.join(f.id.to_string()).is_dir());
        assert_eq!(f.count(), 1);
        assert!(f
            .status()
            .last_success_path
            .unwrap()
            .contains("New backup location"));
    }
    #[test]
    fn failures_preserve_last_success_retry_later_and_leave_preferences_evidence_intact() {
        let mut f = Fixture::new();
        f.change("First");
        f.tick(900);
        let last = f.status().last_success_path.unwrap();
        f.change("Second");
        let missing = f._dir.path().join("Missing location");
        f.store
            .update(|p| p.default_backups_dir = Some(missing.clone()))
            .unwrap();
        f.tick(1800);
        assert!(f.status().error.is_some());
        assert_eq!(f.status().last_success_path, Some(last.clone()));
        assert!(!missing.exists());
        std::fs::create_dir(&missing).unwrap();
        f.tick(1801);
        assert!(!missing.join(f.id.to_string()).exists());
        f.tick(2700);
        assert!(f.status().error.is_none());
        for bytes in [
            b"corrupt".as_slice(),
            br#"{"schema_version":99}"#.as_slice(),
        ] {
            std::fs::write(f.store.path(), bytes).unwrap();
            f.tick(3600);
            assert_eq!(f.status().enabled, None);
            assert!(f.status().error.is_some());
            assert_eq!(std::fs::read(f.store.path()).unwrap(), bytes);
        }
        assert!(std::path::Path::new(&last).is_dir());
    }
    #[test]
    fn missing_default_uses_app_owned_storage_and_close_never_forces_a_snapshot() {
        let mut f = Fixture::new();
        f.store.update(|p| p.default_backups_dir = None).unwrap();
        f.change("Edited");
        f.tick(900);
        assert!(f
            .status()
            .last_success_path
            .unwrap()
            .contains("Automatic Backups"));
        f.change("Last minute edit");
        ProjectService::close_project(&f.state, f.id).unwrap();
        f.tick(1000);
        assert!(!f.statuses.lock().unwrap().contains_key(&f.id));
        // Reopening the same identity gets a fresh cadence and validates old backups.
        ProjectService::open_project(&f.state, std::path::Path::new(&f.path), false).unwrap();
        f.start = f.state.open_projects.lock().unwrap()[&f.id].opened_at;
        f.tick(0);
        assert_eq!(f.status().last_success_revision, None);
        f.tick(900);
        assert_eq!(f.status().last_success_revision, Some(2));
    }
    #[test]
    fn native_scheduler_drop_stops_without_a_fifteen_minute_wait() {
        let f = Fixture::new();
        let started = Instant::now();
        let service = AutomaticBackups::start(
            f.state.clone(),
            f.store.path().into(),
            f._dir.path().join("App Data"),
        )
        .unwrap();
        assert_eq!(
            service.status(&f.state, &f.store, f.id).unwrap().enabled,
            Some(true)
        );
        assert!(service
            .status(&f.state, &f.store, ProjectId::new())
            .is_err());
        drop(service);
        assert!(started.elapsed() < Duration::from_secs(5));
    }
    #[test]
    fn automatic_manual_and_authoring_operations_share_the_worker_queue() {
        let mut f = Fixture::new();
        f.change("Before competing operations");
        let state = f.state.clone();
        let id = f.id;
        let root = f.root.clone();
        let gate = std::sync::Barrier::new(3);
        std::thread::scope(|scope| {
            let manual = scope.spawn(|| {
                gate.wait();
                ProjectService::create_backup(&state, id, &root).unwrap()
            });
            let author = scope.spawn(|| {
                gate.wait();
                ProjectService::rename_project(&state, id, "Competing edit", 1).unwrap()
            });
            gate.wait();
            f.tick(900);
            let manual_path = manual.join().unwrap();
            author.join().unwrap();
            crate::backup_recovery::validate_backup(&manual_path).unwrap();
            assert!(!manual_path.join("automatic-backup.json").exists());
        });
        let status = f.status();
        let revision = status.last_success_revision.unwrap();
        assert!([1, 2].contains(&revision));
        assert!(automatic::is_valid(
            std::path::Path::new(&status.last_success_path.unwrap()),
            &f.root,
            f.id,
            revision
        ));
        assert_eq!(
            ProjectService::get_summary(&f.state, f.id)
                .unwrap()
                .revision,
            2
        );
        assert_eq!(f.count(), 2);
    }

    #[test]
    fn close_during_a_queued_backup_does_not_reopen_or_snapshot_a_closed_project() {
        let mut f = Fixture::new();
        f.change("Pending");
        f.tick(0);
        let open = f.state.open_projects.lock().unwrap()[&f.id].clone();
        let hold = open.worker.lock().unwrap();
        let state = f.state.clone();
        let id = f.id;
        std::thread::scope(|scope| {
            let close = scope.spawn(|| ProjectService::close_project(&state, id));
            // Close removes the registry entry before it waits for this worker.
            while state.open_projects.lock().unwrap().contains_key(&id) {
                std::thread::yield_now();
            }
            f.engine.tick(
                &f.state,
                f.start + INTERVAL,
                f.wall + chrono::Duration::minutes(15),
            );
            drop(hold);
            close.join().unwrap().unwrap();
        });
        assert_eq!(f.count(), 0);
        assert!(!f.statuses.lock().unwrap().contains_key(&f.id));
    }
}
