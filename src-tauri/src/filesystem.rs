use serde::Serialize;
use std::{
    fs::{self, DirEntry},
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::SystemTime,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ErrorCode {
    InvalidRequest,
    NotFound,
    PermissionDenied,
    NotDirectory,
    Cancelled,
    Unavailable,
}

#[derive(Debug, Serialize)]
pub(crate) struct BrowseError {
    code: ErrorCode,
    message: &'static str,
}

impl BrowseError {
    pub(crate) fn new(code: ErrorCode) -> Self {
        let message = match code {
            ErrorCode::InvalidRequest => "The browse request is invalid.",
            ErrorCode::NotFound => "The folder is no longer available at this path.",
            ErrorCode::PermissionDenied => "You do not have permission to read this folder.",
            ErrorCode::NotDirectory => "This path does not point to a folder.",
            ErrorCode::Cancelled => "Browsing was cancelled.",
            ErrorCode::Unavailable => "The folder could not be read.",
        };
        Self { code, message }
    }
}

impl From<io::Error> for BrowseError {
    fn from(error: io::Error) -> Self {
        Self::new(match error.kind() {
            io::ErrorKind::NotFound => ErrorCode::NotFound,
            io::ErrorKind::PermissionDenied => ErrorCode::PermissionDenied,
            io::ErrorKind::NotADirectory => ErrorCode::NotDirectory,
            io::ErrorKind::InvalidInput => ErrorCode::InvalidRequest,
            _ => ErrorCode::Unavailable,
        })
    }
}

#[derive(Serialize)]
pub(crate) struct Location {
    path: String,
    label: String,
}

impl Location {
    fn for_path(path: &Path) -> Option<Self> {
        let path_text = path.to_str()?;
        Some(Self {
            path: path_text.to_owned(),
            label: path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(path_text)
                .to_owned(),
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum EntryKind {
    Directory,
    File,
    Link,
    Other,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    name: String,
    location: Option<Location>,
    kind: EntryKind,
    size: Option<u64>,
    modified_ms: Option<i64>,
    created_ms: Option<i64>,
}

#[derive(Serialize)]
pub(crate) struct Directory {
    location: Location,
    breadcrumbs: Vec<Location>,
    entries: Vec<Entry>,
    skipped: usize,
}

pub(crate) fn native_path(path: String) -> Result<PathBuf, BrowseError> {
    if path.contains('\0') {
        return Err(BrowseError::new(ErrorCode::InvalidRequest));
    }
    validate_path(PathBuf::from(path))
}

pub(crate) fn validate_path(path: PathBuf) -> Result<PathBuf, BrowseError> {
    if !path.is_absolute() {
        return Err(BrowseError::new(ErrorCode::InvalidRequest));
    }
    #[cfg(windows)]
    if let Some(std::path::Component::Prefix(prefix)) = path.components().next() {
        use std::path::Prefix;
        if matches!(prefix.kind(), Prefix::DeviceNS(_) | Prefix::Verbatim(_)) {
            return Err(BrowseError::new(ErrorCode::InvalidRequest));
        }
    }
    Ok(path)
}

fn cancelled(cancel: &AtomicBool) -> Result<(), BrowseError> {
    if cancel.load(Ordering::Relaxed) {
        return Err(BrowseError::new(ErrorCode::Cancelled));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn macos_hidden(path: &Path) -> io::Result<bool> {
    use std::{ffi::CString, mem::MaybeUninit, os::unix::ffi::OsStrExt};

    let path = CString::new(path.as_os_str().as_bytes()).map_err(io::Error::other)?;
    let mut attributes = MaybeUninit::<libc::stat>::uninit();
    // lstat reads the directory entry itself, including flags on symbolic links.
    if unsafe { libc::lstat(path.as_ptr(), attributes.as_mut_ptr()) } == -1 {
        return Err(io::Error::last_os_error());
    }
    let attributes = unsafe { attributes.assume_init() };
    Ok(attributes.st_flags & libc::UF_HIDDEN != 0)
}

fn timestamp_ms(value: SystemTime) -> Option<i64> {
    let time = match value.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => duration.as_millis() as i128,
        Err(error) => -(error.duration().as_millis() as i128),
    };
    i64::try_from(time).ok()
}

fn read_entry(entry: DirEntry, show_hidden: bool) -> io::Result<Option<Entry>> {
    let name = entry.file_name();
    let name = name.to_string_lossy();
    if !show_hidden && name.starts_with('.') {
        return Ok(None);
    }

    let path = entry.path();
    #[cfg(target_os = "macos")]
    if !show_hidden && macos_hidden(&path)? {
        return Ok(None);
    }

    // DirEntry metadata describes the link itself.
    let metadata = entry.metadata()?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_HIDDEN;
        if !show_hidden && metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0 {
            return Ok(None);
        }
    }

    let file_type = metadata.file_type();
    let kind = if file_type.is_symlink() {
        EntryKind::Link
    } else if file_type.is_dir() {
        EntryKind::Directory
    } else if file_type.is_file() {
        EntryKind::File
    } else {
        EntryKind::Other
    };
    Ok(Some(Entry {
        name: name.into_owned(),
        location: Location::for_path(&path),
        kind,
        size: file_type.is_file().then_some(metadata.len()),
        modified_ms: metadata.modified().ok().and_then(timestamp_ms),
        created_ms: metadata.created().ok().and_then(timestamp_ms),
    }))
}

pub(crate) fn read_directory(
    path: &Path,
    cancel: &AtomicBool,
    show_hidden: bool,
) -> Result<Directory, BrowseError> {
    cancelled(cancel)?;
    let location =
        Location::for_path(path).ok_or_else(|| BrowseError::new(ErrorCode::Unavailable))?;
    if !fs::metadata(path)?.is_dir() {
        return Err(BrowseError::new(ErrorCode::NotDirectory));
    }
    let mut breadcrumbs: Vec<_> = path
        .ancestors()
        .filter(|ancestor| ancestor.is_absolute())
        .filter_map(Location::for_path)
        .collect();
    breadcrumbs.reverse();

    let mut entries = Vec::new();
    let mut skipped = 0;
    for entry in fs::read_dir(path)? {
        cancelled(cancel)?;
        match entry.and_then(|entry| read_entry(entry, show_hidden)) {
            Ok(Some(entry)) => entries.push(entry),
            Ok(None) => {}
            Err(_) => skipped += 1,
        }
    }
    cancelled(cancel)?;
    Ok(Directory {
        location,
        breadcrumbs,
        entries,
        skipped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("folderai-browse-{}-{id}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn lists_unicode_and_skips_hidden_entries() {
        let fixture = Fixture::new();
        let folder = fixture.0.join("Cartella caffè");
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("Résumé.txt"), "abc").unwrap();
        fs::write(folder.join(".hidden"), "hidden").unwrap();
        let result = read_directory(&folder, &AtomicBool::new(false), false).unwrap();
        assert_eq!(result.location.label, "Cartella caffè");
        assert_eq!(
            result.breadcrumbs.last().unwrap().path,
            result.location.path
        );
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].name, "Résumé.txt");
        assert_eq!(result.entries[0].size, Some(3));
        let creation = fs::metadata(folder.join("Résumé.txt"))
            .unwrap()
            .created()
            .ok()
            .and_then(timestamp_ms);
        assert_eq!(result.entries[0].created_ms, creation);
        assert_eq!(result.skipped, 0);
        let all = read_directory(&folder, &AtomicBool::new(false), true).unwrap();
        assert!(all.entries.iter().any(|entry| entry.name == ".hidden"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn respects_native_hidden_file_flag() {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};

        let fixture = Fixture::new();
        let file = fixture.0.join("flagged");
        fs::write(&file, "hidden").unwrap();
        let path = CString::new(file.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::chflags(path.as_ptr(), libc::UF_HIDDEN) }, 0);
        let visible = read_directory(&fixture.0, &AtomicBool::new(false), false).unwrap();
        assert!(visible.entries.is_empty());
        assert_eq!(visible.skipped, 0);
        let all = read_directory(&fixture.0, &AtomicBool::new(false), true).unwrap();
        assert_eq!(all.entries[0].name, "flagged");
    }

    #[cfg(windows)]
    #[test]
    fn respects_windows_hidden_attribute() {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_HIDDEN, SetFileAttributesW};

        let fixture = Fixture::new();
        let file = fixture.0.join("flagged");
        fs::write(&file, "hidden").unwrap();
        let path: Vec<u16> = file.as_os_str().encode_wide().chain([0]).collect();
        assert_ne!(
            unsafe { SetFileAttributesW(path.as_ptr(), FILE_ATTRIBUTE_HIDDEN) },
            0
        );
        let visible = read_directory(&fixture.0, &AtomicBool::new(false), false).unwrap();
        assert!(visible.entries.is_empty());
        assert_eq!(visible.skipped, 0);
        let all = read_directory(&fixture.0, &AtomicBool::new(false), true).unwrap();
        assert_eq!(all.entries[0].name, "flagged");
    }

    #[test]
    fn returns_scoped_errors_for_missing_file_path_and_cancel() {
        let fixture = Fixture::new();
        let missing = fixture.0.join("missing");
        let file = fixture.0.join("file");
        fs::write(&file, "").unwrap();
        assert!(matches!(
            read_directory(&missing, &AtomicBool::new(false), false),
            Err(error) if error.code == ErrorCode::NotFound
        ));
        assert!(matches!(
            read_directory(&file, &AtomicBool::new(false), false),
            Err(error) if error.code == ErrorCode::NotDirectory
        ));
        assert!(matches!(
            read_directory(&missing, &AtomicBool::new(true), false),
            Err(error) if error.code == ErrorCode::Cancelled
        ));
    }

    #[cfg(unix)]
    #[test]
    fn inaccessible_directories_are_permission_errors() {
        use std::os::unix::fs::PermissionsExt;

        let fixture = Fixture::new();
        fs::set_permissions(&fixture.0, fs::Permissions::from_mode(0o000)).unwrap();
        let result = read_directory(&fixture.0, &AtomicBool::new(false), true);
        fs::set_permissions(&fixture.0, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(matches!(result, Err(error) if error.code == ErrorCode::PermissionDenied));
    }
}
