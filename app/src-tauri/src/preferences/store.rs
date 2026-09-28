//! Mutex-protected boundary around the preferences file, so concurrent
//! read-modify-write commands (e.g. setting the default Projects and
//! Backups directories at nearly the same time) are serialized into a
//! read-modify-write instead of racing and silently losing one change.

use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::{load, reset, save, AppPreferences, PreferencesError};

pub struct PreferencesStore {
    path: PathBuf,
    lock: Mutex<()>,
}

impl PreferencesStore {
    // Keep the lock inode/file in place. Closing the handle releases the OS
    // lock, including after a crash. Separate app processes share this lock.
    fn file_lock(&self) -> Result<File, PreferencesError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.path.with_extension("json.lock"))?;
        file.lock()?;
        Ok(file)
    }
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
        let _file = self.file_lock()?;
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
        let _file = self.file_lock()?;
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
        let _file = self.file_lock()?;
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

        let store_b = Arc::new(PreferencesStore::new(store.path()));
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

    #[test]
    fn updates_preserve_corrupt_and_newer_files() {
        let dir = tempdir().unwrap();
        let store = PreferencesStore::new(dir.path().join("preferences.json"));
        for bytes in [b"broken".as_slice(), br#"{"schema_version":99}"#.as_slice()] {
            std::fs::write(store.path(), bytes).unwrap();
            assert!(store
                .update(|prefs| prefs.default_backups_dir = Some(dir.path().into()))
                .is_err());
            assert_eq!(std::fs::read(store.path()).unwrap(), bytes);
        }
    }
}
