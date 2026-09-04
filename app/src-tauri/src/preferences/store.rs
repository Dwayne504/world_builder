//! Mutex-protected boundary around the preferences file, so concurrent
//! read-modify-write commands (e.g. setting the default Projects and
//! Backups directories at nearly the same time) are serialized into a
//! read-modify-write instead of racing and silently losing one change.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::{load, reset, save, AppPreferences, PreferencesError};

pub struct PreferencesStore {
    path: PathBuf,
    lock: Mutex<()>,
}

impl PreferencesStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        PreferencesStore {
            path: path.into(),
            lock: Mutex::new(()),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<AppPreferences, PreferencesError> {
        let _guard = self
            .lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        load(&self.path)
    }

    /// Loads the current preferences, applies `mutate`, and publishes the
    /// result while holding the store's mutex for the whole
    /// read-modify-write. A load failure (corrupt/unsupported-version
    /// file) is propagated rather than silently discarded, so an update to
    /// one setting never destroys evidence needed to diagnose the other.
    pub fn update(
        &self,
        mutate: impl FnOnce(&mut AppPreferences),
    ) -> Result<AppPreferences, PreferencesError> {
        let _guard = self
            .lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut prefs = load(&self.path)?;
        mutate(&mut prefs);
        save(&self.path, &prefs)?;
        Ok(prefs)
    }

    pub fn reset(&self) -> Result<AppPreferences, PreferencesError> {
        let _guard = self
            .lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        reset(&self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use tempfile::tempdir;

    #[test]
    fn concurrent_updates_to_different_settings_both_survive() {
        let dir = tempdir().unwrap();
        let store = Arc::new(PreferencesStore::new(dir.path().join("preferences.json")));
        let barrier = Arc::new(Barrier::new(2));

        let store_a = store.clone();
        let barrier_a = barrier.clone();
        let projects_dir = dir.path().join("Projects");
        fs_create(&projects_dir);
        let handle_a = thread::spawn(move || {
            barrier_a.wait();
            store_a
                .update(|prefs| prefs.default_projects_dir = Some(projects_dir.clone()))
                .unwrap();
        });

        let store_b = store.clone();
        let barrier_b = barrier.clone();
        let backups_dir = dir.path().join("Backups");
        fs_create(&backups_dir);
        let handle_b = thread::spawn(move || {
            barrier_b.wait();
            store_b
                .update(|prefs| prefs.default_backups_dir = Some(backups_dir.clone()))
                .unwrap();
        });

        handle_a.join().unwrap();
        handle_b.join().unwrap();

        let final_prefs = store.load().unwrap();
        assert_eq!(
            final_prefs.default_projects_dir,
            Some(dir.path().join("Projects"))
        );
        assert_eq!(
            final_prefs.default_backups_dir,
            Some(dir.path().join("Backups"))
        );
    }

    fn fs_create(path: &Path) {
        std::fs::create_dir_all(path).unwrap();
    }
}
