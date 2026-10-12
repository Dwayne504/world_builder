//! A no-clobber publication protocol for explicitly reviewed external files.
//! Never use the small internal JSON publisher on an arbitrary author-selected file.
use super::manuscript_export::ExportError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Fingerprint {
    pub identity: String,
    pub bytes: u64,
    pub modified_nanos: Option<u128>,
    pub sha256: String,
}
fn invalid(message: &str) -> ExportError {
    ExportError::Invalid(message.into())
}

fn redirect(path: &Path, metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::{
            fs::{MetadataExt, OpenOptionsExt},
            io::AsRawHandle,
        };
        use windows_sys::Win32::Storage::FileSystem::*;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0 {
            return false;
        }
        let Ok(file) = fs::OpenOptions::new()
            .read(true)
            .access_mode(FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
            .open(path)
        else {
            return true;
        };
        let mut info = FILE_ATTRIBUTE_TAG_INFO {
            FileAttributes: 0,
            ReparseTag: 0,
        };
        // SAFETY: a live handle and a writable buffer of the exact documented C layout.
        let success = unsafe {
            GetFileInformationByHandleEx(
                file.as_raw_handle(),
                FileAttributeTagInfo,
                (&mut info as *mut FILE_ATTRIBUTE_TAG_INFO).cast(),
                std::mem::size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
            )
        };
        // Only documented, non-name-surrogate Cloud Files tags may pass.
        // MS-FSCC 2.1.2.1: CLOUD through CLOUD_F; all unknown tags fail closed.
        success == 0
            || (info.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
                && info.ReparseTag & !0xF000 != 0x9000_001A)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        false
    }
}
fn checked_parent(path: &Path) -> Result<PathBuf, ExportError> {
    if !path.is_absolute()
        || path.file_name().is_none()
        || !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("md"))
    {
        return Err(invalid("Choose an absolute filename ending in .md."));
    }
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| invalid("Choose a Unicode filename."))?;
    let stem = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_ascii_uppercase();
    if name.ends_with([' ', '.'])
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        || matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        )
        || ["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix).is_some_and(|s| {
                matches!(
                    s,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        })
    {
        return Err(invalid(
            "Choose a regular filename without reserved names or special path characters.",
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| invalid("Choose a folder for the export."))?;
    let mut part = PathBuf::new();
    for component in parent.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(invalid(
                "Choose a direct folder path without parent traversal.",
            ));
        }
        part.push(component);
        if matches!(component, std::path::Component::Prefix(_)) {
            continue;
        }
        let metadata = fs::symlink_metadata(&part)?;
        if !metadata.is_dir() || redirect(&part, &metadata) {
            return Err(invalid(
                "Choose a regular folder, not a linked folder, for the export.",
            ));
        }
        if part
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("wcproj") || e.eq_ignore_ascii_case("wcbackup"))
            || (part.join("manifest.json").is_file() && part.join("data/project.sqlite").is_file())
        {
            return Err(invalid(
                "Save the manuscript outside Worldcrafter Project and backup packages.",
            ));
        }
    }
    Ok(parent.canonicalize()?)
}
pub(super) fn safe_target(path: &Path) -> Result<PathBuf, ExportError> {
    Ok(checked_parent(path)?.join(path.file_name().unwrap()))
}
fn fingerprint_file(file: &mut File) -> Result<Fingerprint, ExportError> {
    let before = file.metadata()?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    let modified = |m: &fs::Metadata| {
        m.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
    };
    let after = file.metadata()?;
    if before.len() != after.len() || modified(&before) != modified(&after) {
        return Err(invalid(
            "The destination changed while being reviewed. Choose it again.",
        ));
    }
    #[cfg(windows)]
    let identity = {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
        };
        let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        // SAFETY: the handle is live and info is a writable, correctly sized C struct.
        if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        format!(
            "{}:{}:{}",
            info.dwVolumeSerialNumber, info.nFileIndexHigh, info.nFileIndexLow
        )
    };
    #[cfg(unix)]
    let identity = {
        use std::os::unix::fs::MetadataExt;
        format!("{}:{}", after.dev(), after.ino())
    };
    #[cfg(not(any(windows, unix)))]
    return Err(invalid(
        "File identity checks are unavailable on this platform.",
    ));
    Ok(Fingerprint {
        identity,
        bytes: after.len(),
        modified_nanos: modified(&after),
        sha256: format!("{:x}", hash.finalize()),
    })
}
pub(super) fn fingerprint(path: &Path) -> Result<Option<Fingerprint>, ExportError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && !redirect(path, &metadata) => {
            Ok(Some(fingerprint_file(&mut File::open(path)?)?))
        }
        Ok(_) => Err(invalid(
            "The destination is not a regular file. Choose another filename.",
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}
fn sync_parent(path: &Path) {
    if let Ok(dir) = File::open(path) {
        let _ = dir.sync_all();
    }
}

/// Publish into an absent name; never replace a concurrent destination. Windows
/// uses a same-volume rename so ordinary FAT/exFAT export folders also work.
fn publish_absent(from: &Path, to: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH};
        let from: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: both pointers refer to live, NUL-terminated Windows paths.
        // No REPLACE_EXISTING or COPY_ALLOWED: an occupied destination fails.
        // https://learn.microsoft.com/windows/win32/api/winbase/nf-winbase-movefileexw
        if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), MOVEFILE_WRITE_THROUGH) } == 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
    #[cfg(not(windows))]
    {
        fs::hard_link(from, to)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Recovery {
    version: u32,
    id: uuid::Uuid,
    filename: String,
    original: Fingerprint,
}
fn restore_prior(original: &Path, target: &Path) -> Result<(), ExportError> {
    publish_absent(original, target).map_err(|_| ExportError::Recovery {
        path: original.display().to_string(),
        message: "The destination changed during export. Its earlier contents are preserved in the recovery file; keep both files and choose a new export filename.".into(),
    })?;
    sync_parent(target.parent().unwrap());
    Ok(())
}
/// Report interruption evidence without modifying any author-selected file.
/// Choosing or cancelling Save As must never resurrect a deleted destination.
pub(super) fn check_recovery(target: &Path) -> Result<(), ExportError> {
    if fs::symlink_metadata(target).is_ok() {
        return Ok(());
    }
    let parent = checked_parent(target)?;
    for item in fs::read_dir(&parent)? {
        let item = item?;
        let name = item.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.starts_with(".worldcrafter-export-") || !name.ends_with(".recovery") {
            continue;
        }
        let metadata = fs::symlink_metadata(item.path())?;
        if !metadata.is_dir() || redirect(&item.path(), &metadata) {
            continue;
        }
        let record_path = item.path().join("recovery.json");
        let Ok(metadata) = fs::symlink_metadata(&record_path) else {
            continue;
        };
        if !metadata.is_file() || redirect(&record_path, &metadata) || metadata.len() > 64 * 1024 {
            continue;
        }
        let Ok(bytes) = fs::read(&record_path) else {
            continue;
        };
        let Ok(record) = serde_json::from_slice::<Recovery>(&bytes) else {
            continue;
        };
        if record.version != 1
            || name != format!(".worldcrafter-export-{}.recovery", record.id)
            || target.file_name().and_then(|s| s.to_str()) != Some(record.filename.as_str())
        {
            continue;
        }
        let original = item.path().join("original.md");
        if fingerprint(&original).ok().flatten() != Some(record.original) {
            continue;
        }
        return Err(ExportError::Recovery { path:original.display().to_string(), message:"An earlier export was interrupted. Its previous contents are preserved here. Recover that file yourself, or choose a different export filename.".into() });
    }
    Ok(())
}

pub(super) fn publish(
    target: &Path,
    expected: Option<&Fingerprint>,
    bytes: &[u8],
) -> Result<(), ExportError> {
    publish_with(target, expected, bytes, publish_absent)
}
fn publish_with(
    target: &Path,
    expected: Option<&Fingerprint>,
    bytes: &[u8],
    publish: impl FnOnce(&Path, &Path) -> std::io::Result<()>,
) -> Result<(), ExportError> {
    publish_checked(target, expected, bytes, publish, fingerprint)
}
fn publish_checked(
    target: &Path,
    expected: Option<&Fingerprint>,
    bytes: &[u8],
    publish: impl FnOnce(&Path, &Path) -> std::io::Result<()>,
    inspect_prior: impl Fn(&Path) -> Result<Option<Fingerprint>, ExportError>,
) -> Result<(), ExportError> {
    let checked = safe_target(target)?;
    if checked != target || fingerprint(target)?.as_ref() != expected {
        return Err(invalid("The destination changed after review. Choose and review it again; nothing was overwritten."));
    }
    let parent = target.parent().unwrap();
    let id = uuid::Uuid::new_v4();
    let next = parent.join(format!(".worldcrafter-export-{id}.next"));
    let recovery = parent.join(format!(".worldcrafter-export-{id}.recovery"));
    let mut staged = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&next)?;
    if let Err(error) = staged.write_all(bytes).and_then(|_| staged.sync_all()) {
        drop(staged);
        let _ = fs::remove_file(&next);
        return Err(error.into());
    }
    drop(staged);
    if fs::read(&next)? != bytes {
        let _ = fs::remove_file(&next);
        return Err(invalid(
            "The export file could not be verified. The destination is unchanged.",
        ));
    }
    let result = (|| {
        #[cfg(not(windows))]
        {
            // Other platforms use hard links for no-clobber publication. Prove
            // that capability before moving an existing destination.
            let probe = parent.join(format!(".worldcrafter-export-{id}.probe"));
            fs::hard_link(&next, &probe)?;
            fs::remove_file(probe)?;
        }
        safe_target(target)?;
        if fingerprint(target)?.as_ref() != expected {
            return Err(invalid(
                "The destination changed after review. Choose it again; nothing was overwritten.",
            ));
        }
        let mut original = None;
        let mut hold = None;
        if let Some(expected) = expected {
            // Windows denies concurrent data writes for the whole replacement,
            // while allowing the owned rename. Other platforms also use the OS
            // advisory lock, with fingerprints and no-clobber publication below.
            let mut options = fs::OpenOptions::new();
            options.read(true);
            #[cfg(windows)]
            {
                use std::os::windows::fs::OpenOptionsExt;
                options.share_mode(0x1 | 0x4);
            }
            let mut file = options.open(target)?;
            #[cfg(not(windows))]
            file.try_lock().map_err(|_| {
                invalid("The destination is open for editing. Close it or choose a new filename.")
            })?;
            if &fingerprint_file(&mut file)? != expected {
                return Err(invalid(
                    "The destination changed after review. Choose it again.",
                ));
            }
            hold = Some(file);
            fs::create_dir(&recovery)?;
            let record = Recovery {
                version: 1,
                id,
                filename: target
                    .file_name()
                    .unwrap()
                    .to_str()
                    .ok_or_else(|| invalid("Choose a Unicode filename."))?
                    .into(),
                original: expected.clone(),
            };
            let mut record_file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(recovery.join("recovery.json"))?;
            record_file
                .write_all(&serde_json::to_vec(&record).map_err(|e| invalid(&e.to_string()))?)?;
            record_file.sync_all()?;
            drop(record_file);
            sync_parent(&recovery);
            let prior = recovery.join("original.md");
            fs::rename(target, &prior)?;
            sync_parent(parent);
            if inspect_prior(&prior).ok().flatten().as_ref() != Some(expected) {
                restore_prior(&prior, target)?;
                return Err(invalid("The destination changed during export. Its contents were restored; choose and review it again."));
            }
            original = Some(prior);
        }
        // Publishing the complete synced file into an empty slot cannot replace
        // a file created by another process in the meantime.
        if let Err(error) = publish(&next, target) {
            if let Some(prior) = &original {
                restore_prior(prior, target)?;
            }
            return Err(error.into());
        }
        sync_parent(parent);
        if let Some(prior) = &original {
            if inspect_prior(prior).ok().flatten().as_ref() != expected {
                return Err(ExportError::Recovery { path:prior.display().to_string(),message:"A concurrently edited previous file was preserved. The new export exists; review both before continuing.".into() });
            }
            // Only named files created by this operation are removed. A changed
            // or unexpected recovery directory is never recursively cleared.
            fs::remove_file(prior).map_err(|_| ExportError::Recovery { path: prior.display().to_string(), message: "The export was written, but its earlier copy could not be removed. Review the recovery file before removing it yourself.".into() })?;
            drop(hold.take());
            let _ = fs::remove_file(recovery.join("recovery.json"));
            let _ = fs::remove_dir(&recovery);
        }
        drop(hold);
        Ok(())
    })();
    let _ = fs::remove_file(next);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    fn denied() -> std::io::Error {
        std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "injected filesystem failure",
        )
    }
    #[test]
    fn publication_failure_preserves_prior_and_never_creates_partial_destination() {
        let dir = tempdir().unwrap();
        let path = safe_target(&dir.path().join("author.md")).unwrap();
        assert!(publish_with(&path, None, b"new", |_, _| Err(denied())).is_err());
        assert!(!path.exists());
        fs::write(&path, b"old").unwrap();
        let before = fingerprint(&path).unwrap();
        assert!(publish_with(&path, before.as_ref(), b"new", |_, _| Err(denied())).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"old");
        // No adjacent unknown author file is ever cleaned up.
        let unknown = dir.path().join(".worldcrafter-export-unrecognized.next");
        fs::write(&unknown, b"keep").unwrap();
        publish(&path, fingerprint(&path).unwrap().as_ref(), b"complete").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"complete");
        assert_eq!(fs::read(unknown).unwrap(), b"keep");
    }
    #[test]
    fn concurrent_new_file_is_never_replaced_and_original_has_actionable_recovery() {
        let dir = tempdir().unwrap();
        let path = safe_target(&dir.path().join("author.md")).unwrap();
        fs::write(&path, b"original").unwrap();
        let before = fingerprint(&path).unwrap();
        let error = publish_with(&path, before.as_ref(), b"export", |from, to| {
            fs::write(to, b"concurrent").unwrap();
            fs::hard_link(from, to)
        })
        .unwrap_err();
        assert_eq!(fs::read(&path).unwrap(), b"concurrent");
        let ExportError::Recovery { path: prior, .. } = error else {
            panic!("expected recovery path")
        };
        assert_eq!(fs::read(&prior).unwrap(), b"original");
        // A cancelled/review-only choice must not resurrect a later-deleted file.
        fs::remove_file(&path).unwrap();
        assert!(matches!(
            check_recovery(&path),
            Err(ExportError::Recovery { .. })
        ));
        assert!(!path.exists());
        assert_eq!(fs::read(prior).unwrap(), b"original");
    }
    #[test]
    fn fingerprint_failure_after_displacement_restores_or_identifies_prior() {
        let dir = tempdir().unwrap();
        let path = safe_target(&dir.path().join("author.md")).unwrap();
        fs::write(&path, b"original").unwrap();
        let before = fingerprint(&path).unwrap();
        assert!(publish_checked(
            &path,
            before.as_ref(),
            b"export",
            |from, to| fs::hard_link(from, to),
            |_| Err(denied().into())
        )
        .is_err());
        assert_eq!(fs::read(&path).unwrap(), b"original");
        let calls = std::cell::Cell::new(0);
        let error = publish_checked(
            &path,
            fingerprint(&path).unwrap().as_ref(),
            b"export",
            |from, to| fs::hard_link(from, to),
            |p| {
                calls.set(calls.get() + 1);
                if calls.get() == 1 {
                    fingerprint(p)
                } else {
                    Err(denied().into())
                }
            },
        )
        .unwrap_err();
        let ExportError::Recovery { path: prior, .. } = error else {
            panic!("expected preserved path")
        };
        assert_eq!(fs::read(prior).unwrap(), b"original");
        assert_eq!(fs::read(&path).unwrap(), b"export");
    }
    #[test]
    fn filename_boundaries_and_managed_packages_are_rejected() {
        let dir = tempdir().unwrap();
        for name in [
            "CON.md",
            "con .md",
            "LPT1.md",
            "COM³.md",
            "ordinary:stream.md",
            "bad?.md",
            "bad*.md",
            "trailing.md.",
            "trailing.md ",
            "wrong.txt",
        ] {
            assert!(safe_target(&dir.path().join(name)).is_err(), "{name}");
        }
        for name in ["Project.wcproj", "Backup.wcbackup"] {
            let package = dir.path().join(name);
            fs::create_dir(&package).unwrap();
            assert!(safe_target(&package.join("export.md")).is_err());
        }
        let renamed = dir.path().join("renamed package");
        fs::create_dir_all(renamed.join("data")).unwrap();
        fs::write(renamed.join("manifest.json"), "{}").unwrap();
        fs::write(renamed.join("data/project.sqlite"), "synthetic").unwrap();
        assert!(safe_target(&renamed.join("export.md")).is_err());
        assert!(safe_target(&dir.path().join("absent/export.md")).is_err());
        assert!(safe_target(&dir.path().join("../export.md")).is_err());
        assert!(safe_target(&dir.path().join("résumé 日本語.md")).is_ok());
    }
    #[test]
    fn malformed_or_unowned_recovery_files_are_untouched() {
        let dir = tempdir().unwrap();
        let path = safe_target(&dir.path().join("author.md")).unwrap();
        for (name, record) in [
            (
                ".worldcrafter-export-not-owned.recovery".to_owned(),
                "{}".to_owned(),
            ),
            (
                format!(".worldcrafter-export-{}.recovery", uuid::Uuid::new_v4()),
                "{broken".to_owned(),
            ),
        ] {
            let recovery = dir.path().join(name);
            fs::create_dir(&recovery).unwrap();
            fs::write(recovery.join("recovery.json"), &record).unwrap();
            fs::write(recovery.join("original.md"), b"unknown").unwrap();
            check_recovery(&path).unwrap();
            assert!(!path.exists());
            assert_eq!(
                fs::read_to_string(recovery.join("recovery.json")).unwrap(),
                record
            );
            assert_eq!(fs::read(recovery.join("original.md")).unwrap(), b"unknown");
        }
    }
    #[test]
    fn replacement_with_identical_bytes_and_timestamp_still_needs_review() {
        let dir = tempdir().unwrap();
        let target = safe_target(&dir.path().join("author.md")).unwrap();
        fs::write(&target, b"identical").unwrap();
        let before = fingerprint(&target).unwrap().unwrap();
        let modified = fs::metadata(&target).unwrap().modified().unwrap();
        let replacement = dir.path().join("replacement.md");
        fs::write(&replacement, b"identical").unwrap();
        File::options()
            .write(true)
            .open(&replacement)
            .unwrap()
            .set_modified(modified)
            .unwrap();
        fs::remove_file(&target).unwrap();
        fs::rename(&replacement, &target).unwrap();
        let after = fingerprint(&target).unwrap().unwrap();
        assert_eq!(before.sha256, after.sha256);
        assert_eq!(before.modified_nanos, after.modified_nanos);
        assert_ne!(before.identity, after.identity);
        assert!(publish(&target, Some(&before), b"export").is_err());
        assert_eq!(fs::read(target).unwrap(), b"identical");
    }
    #[cfg(windows)]
    #[test]
    fn open_writer_prevents_overwrite_without_displacing_author_file() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = tempdir().unwrap();
        let path = safe_target(&dir.path().join("author.md")).unwrap();
        fs::write(&path, b"author").unwrap();
        let before = fingerprint(&path).unwrap();
        let hold = fs::OpenOptions::new()
            .write(true)
            .share_mode(0)
            .open(&path)
            .unwrap();
        assert!(publish(&path, before.as_ref(), b"export").is_err());
        drop(hold);
        assert_eq!(fs::read(&path).unwrap(), b"author");
    }
    #[cfg(windows)]
    #[test]
    fn junction_parent_is_refused_without_touching_link_target() {
        use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
        use windows_sys::Win32::Storage::FileSystem::*;
        // A synthetic mount-point reparse record avoids shell-built commands.
        let dir = tempdir().unwrap();
        let linked = dir.path().join("junction");
        let real = dir.path().join("real");
        fs::create_dir(&real).unwrap();
        fs::create_dir(&linked).unwrap();
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
            .open(&linked)
            .unwrap();
        let target = format!("\\??\\{}", real.display());
        let mut name: Vec<u16> = target.encode_utf16().collect();
        name.push(0);
        let mut data = Vec::new();
        data.extend_from_slice(&0xA0000003u32.to_le_bytes());
        data.extend_from_slice(&((8 + name.len() * 2 + 2) as u16).to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&((name.len() * 2 - 2) as u16).to_le_bytes());
        data.extend_from_slice(&((name.len() * 2) as u16).to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        for c in name {
            data.extend_from_slice(&c.to_le_bytes());
        }
        data.extend_from_slice(&0u16.to_le_bytes());
        let mut returned = 0;
        // SAFETY: live handle and a correctly sized mount-point reparse buffer.
        let ok = unsafe {
            windows_sys::Win32::System::IO::DeviceIoControl(
                file.as_raw_handle(),
                0x000900A4,
                data.as_ptr().cast(),
                data.len() as u32,
                std::ptr::null_mut(),
                0,
                &mut returned,
                std::ptr::null_mut(),
            )
        };
        assert_ne!(ok, 0, "{}", std::io::Error::last_os_error());
        drop(file);
        assert!(safe_target(&linked.join("export.md")).is_err());
        assert!(!real.join("export.md").exists());
        fs::remove_dir(linked).unwrap();
    }
}
