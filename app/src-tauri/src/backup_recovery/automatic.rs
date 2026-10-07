//! Positively owned rolling snapshots. Folder names alone never authorize cleanup.
use super::{BackupError, PackagePaths, ProjectDbWorker, ProjectId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub const RETENTION_COUNT: usize = 20;
const MARKER: &str = "automatic-backup.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum InventoryItem {
    Directory,
    File { bytes: u64, sha256: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Ownership {
    schema_version: u32,
    kind: String,
    snapshot_id: uuid::Uuid,
    project_id: ProjectId,
    revision: i64,
    created_at: DateTime<Utc>,
    inventory: BTreeMap<String, InventoryItem>,
}

pub struct AutomaticSnapshot {
    pub path: PathBuf,
    pub revision: i64,
    pub created_at: DateTime<Utc>,
    pub cleanup_error: Option<String>,
}

fn unsafe_path(path: &Path) -> BackupError {
    BackupError::UnsafePath(path.display().to_string())
}

fn is_link(path: &Path, metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 == 0 {
            return false;
        }
        // Cloud Files placeholders do not redirect to another named entity.
        // Unknown tags or an unreadable tag still fail closed.
        windows_reparse_tag(path)
            .map(|tag| tag.is_some_and(|tag| !is_cloud_files_tag(tag)))
            .unwrap_or(true)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        false
    }
}

#[cfg(windows)]
fn is_cloud_files_tag(tag: u32) -> bool {
    // MS-FSCC 2.1.2.1 documents CLOUD (0x9000001A) through CLOUD_F
    // (0x9000F01A), none of which has the name-surrogate bit.
    // https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-fscc/c8e77b37-3909-4fe6-a4ea-2b9d423b1ee4
    tag & !0x0000_F000 == 0x9000_001A
}

#[cfg(windows)]
fn windows_reparse_tag(path: &Path) -> std::io::Result<Option<u32>> {
    use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
    use windows_sys::Win32::Storage::FileSystem::{
        FileAttributeTagInfo, GetFileInformationByHandleEx, FILE_ATTRIBUTE_REPARSE_POINT,
        FILE_ATTRIBUTE_TAG_INFO, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    };
    // Inspect the reparse point itself, never its target. BACKUP_SEMANTICS
    // permits directory handles without requesting data-write access.
    let file = fs::OpenOptions::new()
        .read(true)
        .access_mode(FILE_READ_ATTRIBUTES)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)?;
    let mut info = FILE_ATTRIBUTE_TAG_INFO {
        FileAttributes: 0,
        ReparseTag: 0,
    };
    // SAFETY: file owns a live handle; info has the API's exact C layout,
    // and the pointer remains valid for its documented byte length.
    let success = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileAttributeTagInfo,
            (&mut info as *mut FILE_ATTRIBUTE_TAG_INFO).cast(),
            std::mem::size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
        )
    };
    if success == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok((info.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0).then_some(info.ReparseTag))
}

/// Check every existing ancestor before canonicalization can hide a redirect.
fn checked_absolute(path: &Path) -> Result<PathBuf, BackupError> {
    if !path.is_absolute() {
        return Err(unsafe_path(path));
    }
    let mut part = PathBuf::new();
    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(unsafe_path(path));
        }
        part.push(component);
        if matches!(component, std::path::Component::Prefix(_)) {
            continue;
        }
        let metadata = fs::symlink_metadata(&part)?;
        if is_link(&part, &metadata) {
            return Err(unsafe_path(&part));
        }
    }
    Ok(path.canonicalize()?)
}

fn inventory(root: &Path) -> Result<BTreeMap<String, InventoryItem>, BackupError> {
    fn visit(
        root: &Path,
        directory: &Path,
        depth: usize,
        result: &mut BTreeMap<String, InventoryItem>,
    ) -> Result<(), BackupError> {
        if depth > 64 || result.len() > 100_000 {
            return Err(unsafe_path(directory));
        }
        for child in fs::read_dir(directory)? {
            let child = child?;
            let path = child.path();
            let metadata = fs::symlink_metadata(&path)?;
            if is_link(&path, &metadata) {
                return Err(unsafe_path(&path));
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|_| unsafe_path(&path))?
                .to_str()
                .ok_or_else(|| unsafe_path(&path))?
                .replace('\\', "/");
            if metadata.is_dir() {
                result.insert(relative, InventoryItem::Directory);
                visit(root, &path, depth + 1, result)?;
            } else if metadata.is_file() {
                let mut file = fs::File::open(&path)?;
                let mut hash = Sha256::new();
                let mut buffer = [0u8; 65536];
                loop {
                    let count = file.read(&mut buffer)?;
                    if count == 0 {
                        break;
                    }
                    hash.update(&buffer[..count]);
                }
                result.insert(
                    relative,
                    InventoryItem::File {
                        bytes: metadata.len(),
                        sha256: format!("{:x}", hash.finalize()),
                    },
                );
            } else {
                return Err(unsafe_path(&path));
            }
        }
        Ok(())
    }
    let mut result = BTreeMap::new();
    visit(root, root, 0, &mut result)?;
    Ok(result)
}

fn filename(id: uuid::Uuid) -> String {
    format!("automatic-{id}.wcbackup")
}

fn owned_snapshot(
    path: &Path,
    project_root: &Path,
    project: ProjectId,
) -> Result<Ownership, BackupError> {
    let absolute = checked_absolute(path)?;
    let root = checked_absolute(project_root)?;
    if absolute.parent() != Some(root.as_path()) || !absolute.starts_with(&root) {
        return Err(unsafe_path(path));
    }
    let marker = absolute.join(MARKER);
    if is_link(&marker, &fs::symlink_metadata(&marker)?)
        || fs::metadata(&marker)?.len() > 16 * 1024 * 1024
    {
        return Err(unsafe_path(&marker));
    }
    let record: Ownership = serde_json::from_slice(&fs::read(&marker)?)
        .map_err(|_| BackupError::NotABackup(path.display().to_string()))?;
    if record.schema_version != 1
        || record.kind != "automatic"
        || record.project_id != project
        || record.revision < 0
        || absolute.file_name().and_then(|s| s.to_str())
            != Some(filename(record.snapshot_id).as_str())
    {
        return Err(unsafe_path(path));
    }
    let mut contents = inventory(&absolute)?;
    // The marker describes all other contents and cannot hash itself. Only
    // exclude it here: an asset with the same filename is ordinary content.
    contents.remove(MARKER);
    if contents != record.inventory {
        return Err(BackupError::CorruptSnapshot(path.display().to_string()));
    }
    let manifest = super::validate_backup(&absolute)?;
    if manifest.project_id != project {
        return Err(unsafe_path(path));
    }
    let conn = super::open_snapshot_read_only(&PackagePaths::new(&absolute).db_path())?;
    let revision: i64 = conn.query_row(
        "SELECT last_committed_revision FROM project_meta WHERE id=1",
        [],
        |r| r.get(0),
    )?;
    if revision != record.revision {
        return Err(unsafe_path(path));
    }
    Ok(record)
}

/// Durably flush only the newly constructed snapshot before its marker is
/// published and any earlier recovery point becomes eligible for retention.
fn sync_snapshot_files(root: &Path) -> Result<(), BackupError> {
    for child in fs::read_dir(root)? {
        let path = child?.path();
        let metadata = fs::symlink_metadata(&path)?;
        if is_link(&path, &metadata) {
            return Err(unsafe_path(&path));
        }
        if metadata.is_dir() {
            sync_snapshot_files(&path)?;
        } else if metadata.is_file() {
            fs::OpenOptions::new().write(true).open(&path)?.sync_all()?;
        } else {
            return Err(unsafe_path(&path));
        }
    }
    super::sync_directory(root)?;
    Ok(())
}

/// Caller holds the Project worker mutex through snapshot publication and retention.
pub fn create(
    worker: &ProjectDbWorker,
    live: &PackagePaths,
    backup_root: &Path,
    now: DateTime<Utc>,
) -> Result<AutomaticSnapshot, BackupError> {
    create_with_filesystem(
        worker,
        live,
        backup_root,
        now,
        |from, to| fs::rename(from, to),
        |path| fs::remove_dir_all(path),
    )
}

fn create_with_filesystem(
    worker: &ProjectDbWorker,
    live: &PackagePaths,
    backup_root: &Path,
    now: DateTime<Utc>,
    publish: impl FnOnce(&Path, &Path) -> std::io::Result<()>,
    remove: impl Fn(&Path) -> std::io::Result<()>,
) -> Result<AutomaticSnapshot, BackupError> {
    let root = checked_absolute(backup_root)?;
    super::ensure_outside_live_package(&root, live)?;
    // Managed assets must be regular, owned files before the common copier runs.
    checked_absolute(&live.assets_dir())?;
    inventory(&live.assets_dir())?;
    let project = worker.read_meta()?.project_id;
    let project_root = root.join(project.to_string());
    match fs::create_dir(&project_root) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e.into()),
    }
    let project_root = checked_absolute(&project_root)?;
    if project_root.parent() != Some(root.as_path()) {
        return Err(unsafe_path(&project_root));
    }
    let id = uuid::Uuid::new_v4();
    let destination = project_root.join(filename(id));
    let staging = project_root.join(format!(".automatic-{id}.creating"));
    let paths = super::layout::create_skeleton(&staging)?;
    let result = (|| {
        let snapshot = super::populate_snapshot(worker, live, &paths)?;
        super::validate_backup(&staging)?;
        let ownership = Ownership {
            schema_version: 1,
            kind: "automatic".into(),
            snapshot_id: id,
            project_id: project,
            revision: snapshot.last_committed_revision,
            created_at: now,
            inventory: inventory(&staging)?,
        };
        let mut marker = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(staging.join(MARKER))?;
        marker.write_all(
            &serde_json::to_vec_pretty(&ownership).map_err(|_| unsafe_path(&staging))?,
        )?;
        marker.sync_all()?;
        drop(marker);
        sync_snapshot_files(&staging)?;
        // Publish only the already validated complete snapshot and its ownership.
        if checked_absolute(&staging)?.parent() != Some(checked_absolute(&project_root)?.as_path())
        {
            return Err(unsafe_path(&staging));
        }
        publish(&staging, &destination)?;
        super::sync_directory(&project_root)?;
        owned_snapshot(&destination, &project_root, project)?;
        let cleanup_error = prune_with(&project_root, project, &destination, remove).err().map(|_| {
            "The latest automatic backup is safe, but older automatic backups could not all be removed. Check the backup folder.".into()
        });
        Ok(AutomaticSnapshot {
            path: destination.clone(),
            revision: ownership.revision,
            created_at: now,
            cleanup_error,
        })
    })();
    // An interrupted/failed staging folder is deliberately left alone. It is not
    // published or eligible for retention, and no previous recovery point moves.
    result
}

#[cfg(test)]
fn prune(project_root: &Path, project: ProjectId, new_snapshot: &Path) -> Result<(), BackupError> {
    prune_with(project_root, project, new_snapshot, |path| {
        fs::remove_dir_all(path)
    })
}

fn prune_with(
    project_root: &Path,
    project: ProjectId,
    new_snapshot: &Path,
    remove: impl Fn(&Path) -> std::io::Result<()>,
) -> Result<(), BackupError> {
    // A successful new publication is a precondition, not a caller assertion.
    owned_snapshot(new_snapshot, project_root, project)?;
    let mut candidates = vec![];
    for child in fs::read_dir(project_root)? {
        let child = child?;
        if child.path() == new_snapshot {
            continue;
        }
        // Unknown, manual, safety, renamed, partial and damaged material is preserved.
        if let Ok(record) = owned_snapshot(&child.path(), project_root, project) {
            candidates.push((record.created_at, record.snapshot_id, child.path()));
        }
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    for (_, _, candidate) in candidates.into_iter().skip(RETENTION_COUNT - 1) {
        owned_snapshot(&candidate, project_root, project)?;
        // Final absolute containment and no-redirect check immediately precedes
        // recursive deletion. Never delete the root, a link, or an unowned folder.
        let root = checked_absolute(project_root)?;
        let target = checked_absolute(&candidate)?;
        if target.parent() != Some(root.as_path()) || target == root || !target.starts_with(&root) {
            return Err(unsafe_path(&target));
        }
        remove(&target)?;
    }
    Ok(())
}

pub fn latest(
    backup_root: &Path,
    project: ProjectId,
) -> Result<Option<AutomaticSnapshot>, BackupError> {
    let root = checked_absolute(backup_root)?;
    let project_root = root.join(project.to_string());
    if !project_root.exists() {
        return Ok(None);
    }
    checked_absolute(&project_root)?;
    let mut newest = None;
    for child in fs::read_dir(&project_root)? {
        let child = child?;
        if let Ok(record) = owned_snapshot(&child.path(), &project_root, project) {
            if newest
                .as_ref()
                .is_none_or(|s: &AutomaticSnapshot| s.created_at < record.created_at)
            {
                newest = Some(AutomaticSnapshot {
                    path: child.path(),
                    revision: record.revision,
                    created_at: record.created_at,
                    cleanup_error: None,
                });
            }
        }
    }
    Ok(newest)
}

pub fn is_valid(path: &Path, backup_root: &Path, project: ProjectId, revision: i64) -> bool {
    owned_snapshot(path, &backup_root.join(project.to_string()), project)
        .is_ok_and(|s| s.revision == revision)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{tempdir, TempDir};
    struct Fixture {
        _dir: TempDir,
        live: PackagePaths,
        worker: ProjectDbWorker,
        root: PathBuf,
        project: ProjectId,
    }
    impl Fixture {
        fn new() -> Self {
            let dir = tempdir().unwrap();
            let live =
                crate::package::layout::create_skeleton(&dir.path().join("Source.wcproj")).unwrap();
            let project = ProjectId::new();
            let worker = ProjectDbWorker::spawn(
                live.db_path(),
                project,
                Some(crate::persistence::InitialProjectMeta {
                    working_name: "Synthetic source".into(),
                    format_version: 1,
                }),
            )
            .unwrap();
            crate::package::Manifest::new(
                project,
                1,
                crate::persistence::migrations::CURRENT_SCHEMA_VERSION,
                "Synthetic source",
            )
            .write(&live.manifest_path())
            .unwrap();
            let root = dir.path().join("Backups é # fixture");
            fs::create_dir(&root).unwrap();
            Self {
                _dir: dir,
                live,
                worker,
                root,
                project,
            }
        }
        fn snapshot(&self, sequence: i64) -> AutomaticSnapshot {
            self.worker
                .rename_project(
                    self.worker.read_meta().unwrap().last_committed_revision,
                    format!("Revision {sequence}"),
                )
                .unwrap();
            create(
                &self.worker,
                &self.live,
                &self.root,
                DateTime::from_timestamp(1_800_000_000 + sequence * 900, 0).unwrap(),
            )
            .unwrap()
        }
        fn project_root(&self) -> PathBuf {
            self.root.join(self.project.to_string())
        }
    }

    #[test]
    fn automatic_snapshot_is_complete_restorable_owned_and_validation_is_read_only() {
        let f = Fixture::new();
        fs::write(
            f.live.assets_dir().join("invented.txt"),
            "Synthetic attachment",
        )
        .unwrap();
        let saved = f.snapshot(1);
        assert_eq!(saved.revision, 1);
        let ownership = owned_snapshot(&saved.path, &f.project_root(), f.project).unwrap();
        assert_eq!(ownership.kind, "automatic");
        let before = inventory(&saved.path).unwrap();
        for _ in 0..2 {
            super::super::validate_backup(&saved.path).unwrap();
        }
        assert_eq!(inventory(&saved.path).unwrap(), before);
        assert!(!saved.path.join("data/project.sqlite-wal").exists());
        assert!(!saved.path.join("data/project.sqlite-shm").exists());
        let copy = super::super::restore_as_copy(
            &saved.path,
            &f._dir.path().join("copies"),
            Some("Restored fixture"),
        )
        .unwrap();
        assert_ne!(
            crate::package::Manifest::read(&copy.join("manifest.json"))
                .unwrap()
                .project_id,
            f.project
        );
        assert_eq!(
            fs::read(copy.join("assets/invented.txt")).unwrap(),
            b"Synthetic attachment"
        );
        assert_eq!(inventory(&saved.path).unwrap(), before);
    }

    #[test]
    fn immutable_validation_rejects_committed_wal_and_other_journals_without_writing() {
        let f = Fixture::new();
        let saved = f.snapshot(1);
        let paths = PackagePaths::new(&saved.path);
        let conn = rusqlite::Connection::open(paths.db_path()).unwrap();
        conn.pragma_update(None, "journal_mode", "WAL").unwrap();
        conn.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
        conn.execute(
            "UPDATE project_meta SET last_committed_revision=999 WHERE id=1",
            [],
        )
        .unwrap();
        let before = inventory(&saved.path).unwrap();
        assert!(
            fs::metadata(paths.db_path().with_extension("sqlite-wal"))
                .unwrap()
                .len()
                > 0
        );
        assert!(matches!(
            super::super::validate_backup(&saved.path),
            Err(BackupError::UnexpectedJournal)
        ));
        assert!(super::super::restore_as_copy(
            &saved.path,
            &f._dir.path().join("Rejected copy"),
            None
        )
        .is_err());
        assert_eq!(inventory(&saved.path).unwrap(), before);
        drop(conn);
        // Even empty remnants can be evidence of an interrupted copy. Validate
        // nothing that requires ignoring or repairing them.
        for suffix in ["sqlite-wal", "sqlite-shm", "sqlite-journal"] {
            let path = paths.db_path().with_extension(suffix);
            fs::write(&path, []).unwrap();
            let before = inventory(&saved.path).unwrap();
            assert!(matches!(
                super::super::validate_backup(&saved.path),
                Err(BackupError::UnexpectedJournal)
            ));
            assert_eq!(inventory(&saved.path).unwrap(), before);
            fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn retention_keeps_newest_twenty_and_never_removes_unowned_or_changed_folders() {
        let f = Fixture::new();
        let manual = super::super::create_backup(&f.worker, &f.live, &f.root).unwrap();
        let renamed_manual = f.project_root().join(filename(uuid::Uuid::new_v4()));
        fs::rename(&manual, &renamed_manual).unwrap();
        let altered = f.snapshot(1);
        fs::write(
            altered.path.join("personal-note.txt"),
            "Never remove unrelated content",
        )
        .unwrap();
        let unsupported = f.snapshot(2);
        let marker = unsupported.path.join(MARKER);
        let mut future: serde_json::Value =
            serde_json::from_slice(&fs::read(&marker).unwrap()).unwrap();
        future["schema_version"] = 99.into();
        fs::write(&marker, serde_json::to_vec(&future).unwrap()).unwrap();
        let corrupt = f.snapshot(3);
        fs::write(corrupt.path.join(MARKER), "malformed").unwrap();
        let renamed = f.snapshot(4);
        let renamed_path = f.project_root().join("user-kept.wcbackup");
        fs::rename(renamed.path, &renamed_path).unwrap();
        let partial = f.project_root().join(".automatic-interrupted.creating");
        fs::create_dir(&partial).unwrap();
        fs::write(partial.join("source.txt"), "preserve").unwrap();
        let mut valid = vec![];
        for n in 5..=26 {
            valid.push(f.snapshot(n).path);
        }
        assert!(!valid[0].exists());
        assert!(!valid[1].exists());
        assert!(valid[2..].iter().all(|p| p.exists()));
        for path in [
            &renamed_manual,
            &altered.path,
            &unsupported.path,
            &corrupt.path,
            &renamed_path,
            &partial,
        ] {
            assert!(path.exists(), "preserve {}", path.display());
        }
        assert_eq!(
            fs::read(altered.path.join("personal-note.txt")).unwrap(),
            b"Never remove unrelated content"
        );
        assert_eq!(
            fs::read(marker).unwrap(),
            serde_json::to_vec(&future).unwrap()
        );
    }

    #[test]
    fn failed_publication_or_invalid_new_snapshot_cannot_prune_previous_snapshots() {
        let f = Fixture::new();
        let saved = f.snapshot(1);
        // A damaged manifest prevents publication after the database staging copy.
        let manifest = fs::read(f.live.manifest_path()).unwrap();
        fs::write(f.live.manifest_path(), "broken").unwrap();
        assert!(create(&f.worker, &f.live, &f.root, Utc::now()).is_err());
        assert!(saved.path.exists());
        assert!(prune(
            &f.project_root(),
            f.project,
            &f.project_root().join("missing.wcbackup")
        )
        .is_err());
        assert!(saved.path.exists());
        fs::write(f.live.manifest_path(), manifest).unwrap();
        let next = f.snapshot(2);
        assert!(next.path.exists());
        assert!(saved.path.exists());
        assert!(fs::read_dir(f.project_root())
            .unwrap()
            .flatten()
            .any(|e| e.file_name().to_string_lossy().ends_with(".creating")));
    }

    #[test]
    fn wrong_project_corrupt_database_and_modified_inventory_are_not_owned() {
        let f = Fixture::new();
        let saved = f.snapshot(1);
        assert!(owned_snapshot(&saved.path, &f.project_root(), ProjectId::new()).is_err());
        fs::write(saved.path.join("data/project.sqlite"), "corrupt bytes").unwrap();
        assert!(owned_snapshot(&saved.path, &f.project_root(), f.project).is_err());
        let next = f.snapshot(2);
        assert!(saved.path.exists());
        assert!(is_valid(&next.path, &f.root, f.project, 2));
        assert!(create(&f.worker, &f.live, &f.live.assets_dir(), Utc::now()).is_err());
    }

    #[test]
    fn filesystem_publication_failure_and_interrupted_retention_preserve_recovery_points() {
        let f = Fixture::new();
        let manual = super::super::create_backup(&f.worker, &f.live, &f.root).unwrap();
        let mut previous = vec![];
        for n in 1..=20 {
            previous.push(f.snapshot(n).path);
        }
        let now = DateTime::from_timestamp(1_900_000_000, 0).unwrap();
        let failed = create_with_filesystem(
            &f.worker,
            &f.live,
            &f.root,
            now,
            |_, _| {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "injected rename denial",
                ))
            },
            |_| panic!("retention must not run after failed publication"),
        );
        assert!(failed.is_err());
        assert!(previous.iter().all(|p| p.is_dir()));
        super::super::validate_backup(&manual).unwrap();
        let saved = create_with_filesystem(
            &f.worker,
            &f.live,
            &f.root,
            now,
            |from, to| fs::rename(from, to),
            |_| {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "injected delete denial",
                ))
            },
        )
        .unwrap();
        assert!(saved.cleanup_error.is_some());
        owned_snapshot(&saved.path, &f.project_root(), f.project).unwrap();
        assert!(previous.iter().all(|p| p.is_dir()));
        let interrupted_path = std::cell::RefCell::new(None);
        let interrupted = create_with_filesystem(
            &f.worker,
            &f.live,
            &f.root,
            now + chrono::Duration::minutes(15),
            |from, to| fs::rename(from, to),
            |path| {
                // Simulate an interrupted recursive removal, after a complete
                // new snapshot exists. The residue loses ownership and stays.
                fs::remove_file(path.join("manifest.json"))?;
                interrupted_path.replace(Some(path.to_path_buf()));
                Err(std::io::Error::other(
                    "injected interruption during cleanup",
                ))
            },
        )
        .unwrap();
        assert!(interrupted.cleanup_error.is_some());
        owned_snapshot(&interrupted.path, &f.project_root(), f.project).unwrap();
        let interrupted_path = interrupted_path.into_inner().unwrap();
        assert!(interrupted_path.is_dir());
        assert!(!interrupted_path.join("manifest.json").exists());
        let recovered = create(
            &f.worker,
            &f.live,
            &f.root,
            now + chrono::Duration::minutes(30),
        )
        .unwrap();
        assert!(recovered.cleanup_error.is_none());
        assert!(
            interrupted_path.is_dir(),
            "incomplete old folder must not regain deletion authority"
        );
        assert!(saved.path.is_dir());
        assert!(interrupted.path.is_dir());
        super::super::validate_backup(&manual).unwrap();
        let valid_count = fs::read_dir(f.project_root())
            .unwrap()
            .flatten()
            .filter(|item| owned_snapshot(&item.path(), &f.project_root(), f.project).is_ok())
            .count();
        assert_eq!(valid_count, RETENTION_COUNT);
    }

    #[cfg(windows)]
    #[test]
    fn windows_cloud_tags_are_the_only_allowed_reparse_family() {
        for variant in 0..=15 {
            assert!(is_cloud_files_tag(0x9000_001A | (variant << 12)));
        }
        for tag in [
            0,
            0xA000_0003,
            0xA000_000C,
            0xA000_001D,
            0x8000_0021,
            0x9000_001C,
            0x9001_001A,
        ] {
            assert!(!is_cloud_files_tag(tag));
        }
        let dir = tempdir().unwrap();
        assert_eq!(windows_reparse_tag(dir.path()).unwrap(), None);
        let target = dir.path().join("target");
        fs::create_dir(&target).unwrap();
        let junction = dir.path().join("junction");
        directory_link(&junction, &target);
        assert_eq!(windows_reparse_tag(&junction).unwrap(), Some(0xA000_0003));
        assert!(checked_absolute(&junction).is_err());
        fs::remove_dir(junction).unwrap();
    }

    #[cfg(unix)]
    fn directory_link(link: &Path, target: &Path) {
        std::os::unix::fs::symlink(target, link).unwrap();
    }
    #[cfg(windows)]
    fn directory_link(link: &Path, target: &Path) {
        use std::os::windows::process::CommandExt;
        let status = std::process::Command::new("cmd")
            .args(["/D", "/C"])
            .raw_arg(format!(
                "mklink /J \"{}\" \"{}\"",
                link.display(),
                target.display()
            ))
            .creation_flags(0x08000000)
            .output()
            .unwrap();
        assert!(
            status.status.success(),
            "junction setup failed: {}",
            String::from_utf8_lossy(&status.stderr)
        );
    }
    #[test]
    fn redirected_roots_and_nested_links_never_authorize_recursive_cleanup() {
        let f = Fixture::new();
        let outside = f._dir.path().join("Keep outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("keep.txt"), "Keep this").unwrap();
        let link = f._dir.path().join("Redirect");
        directory_link(&link, &outside);
        assert!(create(&f.worker, &f.live, &link, Utc::now()).is_err());
        let source_container = f.live.assets_dir().join(MARKER);
        fs::create_dir(&source_container).unwrap();
        let source_link = source_container.join("redirect");
        directory_link(&source_link, &outside);
        assert!(create(&f.worker, &f.live, &f.root, Utc::now()).is_err());
        #[cfg(windows)]
        fs::remove_dir(&source_link).unwrap();
        #[cfg(unix)]
        fs::remove_file(&source_link).unwrap();
        let saved = f.snapshot(1);
        let nested = saved.path.join("assets/redirect");
        directory_link(&nested, &outside);
        assert!(owned_snapshot(&saved.path, &f.project_root(), f.project).is_err());
        let next = f.snapshot(2);
        assert!(prune(&f.project_root(), f.project, &next.path).is_ok());
        assert_eq!(fs::read(outside.join("keep.txt")).unwrap(), b"Keep this");
        // Remove only our test-created links; never recurse through their targets.
        #[cfg(windows)]
        {
            fs::remove_dir(nested).unwrap();
            fs::remove_dir(link).unwrap();
        }
        #[cfg(unix)]
        {
            fs::remove_file(nested).unwrap();
            fs::remove_file(link).unwrap();
        }
    }
}
