use super::{FileError, captured_directory_path, native::Directory};
use crate::local_identity::{LiveIdentity, LiveObservation, Locator};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) struct TrashResult {
    pub(super) original: Locator,
    pub(super) restoration: Option<TrashRestore>,
}

pub(super) struct TrashRestore {
    pub(super) location: Locator,
    pub(super) witness: LiveIdentity,
}

pub(super) fn trash_verified(
    path: &Path,
    expected: &LiveIdentity,
) -> Result<TrashResult, FileError> {
    trash_with(path, expected, |path, directory, verify| {
        platform::trash(path, directory, verify)
    })
}

fn trash_with(
    path: &Path,
    expected: &LiveIdentity,
    adapter: impl FnOnce(
        &Path,
        bool,
        &dyn Fn() -> Result<(), FileError>,
    ) -> Result<Option<PathBuf>, FileError>,
) -> Result<TrashResult, FileError> {
    let name = path.file_name().ok_or(FileError::InvalidRequest)?;
    let parent = Directory::open(path.parent().ok_or(FileError::InvalidRequest)?)?;
    let selected = Locator::new(captured_directory_path(&parent)?.join(name))
        .ok_or(FileError::InvalidRequest)?;
    let witness = LiveIdentity::open(&selected)?;
    let LiveObservation::Present(original) = witness.observe()? else {
        return Err(FileError::SourceChanged);
    };

    let same_selected = || -> Result<(), FileError> {
        parent.verify()?;
        let current = LiveIdentity::open(&selected)?;
        if current.comparison_key()? != witness.comparison_key()?
            || expected.comparison_key()? != witness.comparison_key()?
            || expected.observe()? != LiveObservation::Present(original.clone())
            || witness.observe()? != LiveObservation::Present(original.clone())
        {
            return Err(FileError::SourceChanged);
        }
        Ok(())
    };

    let is_directory = witness.kind()? == "directory";
    same_selected()?;
    let result = adapter(original.path(), is_directory, &same_selected);
    let observation = witness.observe();
    match (result, observation) {
        (Ok(_), Ok(LiveObservation::Present(current))) if current == original => {
            Err(FileError::RecoveryPending)
        }
        (Ok(_), Ok(LiveObservation::Removed)) => Err(FileError::RecoveryPending),
        (Ok(Some(reported)), Ok(LiveObservation::Present(current))) => {
            let restoration = (reported == current.path()
                || fs::canonicalize(reported).is_ok_and(|path| path == current.path()))
            .then_some(TrashRestore {
                location: current,
                witness,
            });
            Ok(TrashResult {
                original,
                restoration,
            })
        }
        (Ok(_), _) => Ok(TrashResult {
            original,
            restoration: None,
        }),
        (Err(error), Ok(LiveObservation::Present(current))) if current == original => Err(error),
        (Err(_), _) => Err(FileError::RecoveryPending),
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use objc2::rc::autoreleasepool;
    use objc2_foundation::{
        NSCocoaErrorDomain, NSError, NSFileManager, NSFileNoSuchFileError,
        NSFileReadNoPermissionError, NSFileReadNoSuchFileError, NSFileWriteNoPermissionError,
        NSPOSIXErrorDomain, NSURL,
    };
    use std::{
        ffi::{CStr, CString, OsStr},
        io,
        os::unix::ffi::OsStrExt,
        ptr::NonNull,
    };

    pub(super) fn trash(
        path: &Path,
        is_directory: bool,
        verify: impl FnOnce() -> Result<(), FileError>,
    ) -> Result<Option<PathBuf>, FileError> {
        let raw_path =
            CString::new(path.as_os_str().as_bytes()).map_err(|_| FileError::InvalidRequest)?;
        autoreleasepool(|_| {
            let pointer =
                NonNull::new(raw_path.as_ptr().cast_mut()).ok_or(FileError::InvalidRequest)?;
            // The native argument is a NUL-terminated filesystem byte string.
            let url = unsafe {
                NSURL::fileURLWithFileSystemRepresentation_isDirectory_relativeToURL(
                    pointer,
                    is_directory,
                    None,
                )
            };
            let manager = NSFileManager::defaultManager();
            let mut destination = None;
            verify()?;
            manager
                .trashItemAtURL_resultingItemURL_error(&url, Some(&mut destination))
                .map_err(|error| native_error(&error))?;
            Ok(destination.map(|url| {
                // The returned URL owns the representation until this copy completes.
                let bytes = unsafe { CStr::from_ptr(url.fileSystemRepresentation().as_ptr()) };
                PathBuf::from(OsStr::from_bytes(bytes.to_bytes()))
            }))
        })
    }

    fn native_error(error: &NSError) -> FileError {
        let domain = error.domain();
        // These are Foundation-owned, immutable error-domain constants.
        let (posix, cocoa) = unsafe { (NSPOSIXErrorDomain, NSCocoaErrorDomain) };
        if &*domain == posix {
            return i32::try_from(error.code())
                .map(|code| FileError::from(io::Error::from_raw_os_error(code)))
                .unwrap_or(FileError::Unavailable);
        }
        if &*domain == cocoa {
            if [NSFileReadNoPermissionError, NSFileWriteNoPermissionError].contains(&error.code()) {
                return FileError::PermissionDenied;
            }
            if [NSFileNoSuchFileError, NSFileReadNoSuchFileError].contains(&error.code()) {
                return FileError::NotFound;
            }
        }
        FileError::Unavailable
    }
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::{io, os::windows::ffi::OsStrExt};
    use windows::{
        Win32::{
            System::Com::{
                CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
                CoCreateInstance, CoInitializeEx, CoUninitialize,
            },
            UI::Shell::{
                FOF_NOERRORUI, FOF_SILENT, FOFX_EARLYFAILURE, FOFX_RECYCLEONDELETE, FileOperation,
                IFileOperation, IShellItem, SHCreateItemFromParsingName,
            },
        },
        core::{Error, PCWSTR},
    };

    struct Apartment;

    impl Apartment {
        fn enter() -> Result<Self, FileError> {
            // Every successful COM initialization is balanced on this thread.
            unsafe {
                CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE)
                    .ok()
                    .map_err(native_error)?;
            }
            Ok(Self)
        }
    }

    impl Drop for Apartment {
        fn drop(&mut self) {
            // This guard stays on the initiating worker thread.
            unsafe { CoUninitialize() };
        }
    }

    pub(super) fn trash(
        path: &Path,
        _is_directory: bool,
        verify: impl FnOnce() -> Result<(), FileError>,
    ) -> Result<Option<PathBuf>, FileError> {
        let mut words: Vec<u16> = path.as_os_str().encode_wide().collect();
        if words.contains(&0) {
            return Err(FileError::InvalidRequest);
        }
        words.push(0);
        let _apartment = Apartment::enter()?;
        // The path is a terminated wide string that stays live during parsing.
        // The operation is limited to one local shell item in this apartment.
        unsafe {
            let item: IShellItem =
                SHCreateItemFromParsingName(PCWSTR(words.as_ptr()), None).map_err(native_error)?;
            let operation: IFileOperation =
                CoCreateInstance(&FileOperation, None, CLSCTX_INPROC_SERVER)
                    .map_err(native_error)?;
            operation
                .SetOperationFlags(
                    FOFX_RECYCLEONDELETE | FOFX_EARLYFAILURE | FOF_NOERRORUI | FOF_SILENT,
                )
                .map_err(native_error)?;
            operation.DeleteItem(&item, None).map_err(native_error)?;
            verify()?;
            operation.PerformOperations().map_err(native_error)?;
            if operation
                .GetAnyOperationsAborted()
                .map_err(native_error)?
                .as_bool()
            {
                return Err(FileError::Unavailable);
            }
        }
        // Shell Restore also consumes Recycle Bin metadata. The physical
        // namespace alone does not authorize a by-path restore move.
        Ok(None)
    }

    fn native_error(error: Error) -> FileError {
        let code = error.code().0 as u32;
        if code >> 16 == 0x8007 {
            return FileError::from(io::Error::from_raw_os_error((code & 0xffff) as i32));
        }
        FileError::Unavailable
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use crate::local_identity::canonical_path;
    use uuid::Uuid;

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!("folderai-trash-{}", Uuid::new_v4()));
            fs::create_dir(&dir).unwrap();
            Self(canonical_path(&dir).unwrap())
        }

        fn source(&self) -> (PathBuf, LiveIdentity) {
            let path = self.0.join("sintètico.txt");
            fs::write(&path, b"synthetic").unwrap();
            let expected = LiveIdentity::open(&Locator::new(path.clone()).unwrap()).unwrap();
            (path, expected)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn unavailable_trash_keeps_the_original() {
        let fixture = Fixture::new();
        let (source, expected) = fixture.source();
        let result = trash_with(&source, &expected, |_, _, verify| {
            verify()?;
            Err(FileError::PermissionDenied)
        });
        assert!(matches!(result, Err(FileError::PermissionDenied)));
        assert_eq!(fs::read(&source).unwrap(), b"synthetic");
    }

    #[test]
    fn stale_selection_is_rejected_before_native_dispatch() {
        let fixture = Fixture::new();
        let (source, expected) = fixture.source();
        let original = fixture.0.join("moved");
        fs::rename(&source, &original).unwrap();
        fs::write(&source, b"replacement").unwrap();
        let result = trash_with(&source, &expected, |_, _, _| {
            panic!("stale identity reached native Trash");
        });
        assert!(matches!(result, Err(FileError::SourceChanged)));
        assert_eq!(fs::read(&source).unwrap(), b"replacement");
        assert_eq!(fs::read(&original).unwrap(), b"synthetic");
    }

    #[test]
    fn restoration_requires_the_reported_path_to_name_the_held_original() {
        let fixture = Fixture::new();
        let (source, expected) = fixture.source();
        let destination = fixture.0.join("recycle");
        let result = trash_with(&source, &expected, |path, directory, verify| {
            assert!(!directory);
            verify()?;
            fs::rename(path, &destination)?;
            Ok(Some(destination.clone()))
        })
        .unwrap();
        assert_eq!(result.original.path(), source);
        let restored = result.restoration.unwrap();
        assert_eq!(restored.location.path(), destination);
        assert_eq!(
            restored.witness.comparison_key().unwrap(),
            expected.comparison_key().unwrap()
        );
    }

    #[test]
    fn unrelated_result_path_does_not_offer_an_undo_reference() {
        let fixture = Fixture::new();
        let (source, expected) = fixture.source();
        let moved = fixture.0.join("recycle");
        let unrelated = fixture.0.join("unrelated");
        fs::write(&unrelated, b"other").unwrap();
        let result = trash_with(&source, &expected, |path, _, verify| {
            verify()?;
            fs::rename(path, &moved)?;
            Ok(Some(unrelated.clone()))
        })
        .unwrap();
        assert!(result.restoration.is_none());
        assert_eq!(fs::read(moved).unwrap(), b"synthetic");
        assert_eq!(fs::read(unrelated).unwrap(), b"other");
    }

    #[test]
    #[ignore = "requires access to the native user Trash"]
    fn native_trash_round_trip_preserves_synthetic_file_and_folder() {
        let fixture = Fixture::new();
        let mut cases = vec![fixture.source().0];
        let folder = fixture.0.join("cartèlla");
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("child"), b"synthetic").unwrap();
        cases.push(folder);
        for original in cases {
            let expected = LiveIdentity::open(&Locator::new(original.clone()).unwrap()).unwrap();
            let result = trash_verified(&original, &expected).unwrap();
            let restored = result.restoration.expect("requires Trash identity access");
            assert_eq!(result.original.path(), original);
            assert!(!original.exists());
            fs::rename(restored.location.path(), &original).unwrap();
            let payload = if original.is_dir() {
                original.join("child")
            } else {
                original
            };
            assert_eq!(fs::read(payload).unwrap(), b"synthetic");
        }
    }
}
