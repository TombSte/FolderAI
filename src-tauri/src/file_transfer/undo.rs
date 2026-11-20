use super::{
    FileError, captured_directory_path, move_entry,
    native::{self, Directory},
    tree::SourceTree,
};
use crate::{
    local_identity::{LiveIdentity, LiveObservation, Locator},
    storage::Storage,
};
use serde::Serialize;
use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    fs::{File, Metadata},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, atomic::AtomicBool},
    time::SystemTime,
};
use tauri::{AppHandle, Emitter, EventTarget, Manager, WebviewWindow};
use uuid::Uuid;

#[derive(PartialEq, Eq)]
struct Version {
    len: u64,
    modified: SystemTime,
    attributes: u32,
    #[cfg(target_os = "macos")]
    changed: (i64, i64),
}

impl Version {
    fn from(metadata: Metadata) -> std::io::Result<Self> {
        #[cfg(target_os = "macos")]
        use std::os::unix::fs::MetadataExt;
        #[cfg(windows)]
        use std::os::windows::fs::MetadataExt;
        Ok(Self {
            len: metadata.len(),
            modified: metadata.modified()?,
            #[cfg(target_os = "macos")]
            attributes: metadata.mode(),
            #[cfg(windows)]
            attributes: metadata.file_attributes(),
            #[cfg(target_os = "macos")]
            changed: (metadata.ctime(), metadata.ctime_nsec()),
        })
    }

    fn matches_after_rename(&self, current: &Self) -> bool {
        self.len == current.len
            && self.modified == current.modified
            && self.attributes == current.attributes
    }
}

enum CapturedVersion {
    File(File, Version),
    Tree(SourceTree),
}

pub(super) struct UndoMove {
    previous: Directory,
    previous_name: OsString,
    current: Directory,
    current_name: OsString,
    current_location: Locator,
    witness: LiveIdentity,
    version: CapturedVersion,
}

pub(super) struct PermanentFailure {
    pub(super) error: FileError,
    pub(super) retained_path: Option<PathBuf>,
}

impl From<FileError> for PermanentFailure {
    fn from(error: FileError) -> Self {
        Self {
            error,
            retained_path: None,
        }
    }
}

impl UndoMove {
    pub(super) fn capture(
        previous: &Directory,
        previous_name: &OsStr,
        current: &Directory,
        output: &Path,
        witness: LiveIdentity,
    ) -> Result<Self, FileError> {
        let current_name = output.file_name().ok_or(FileError::InvalidRequest)?;
        let current_location = Locator::new(captured_directory_path(current)?.join(current_name))
            .ok_or(FileError::InvalidRequest)?;
        if witness.observe()? != LiveObservation::Present(current_location.clone()) {
            return Err(FileError::SourceChanged);
        }
        let version = if witness.kind()? == "directory" {
            let tree = SourceTree::capture(current_location.path(), &AtomicBool::new(false))?;
            if !tree.matches_root(&witness)? {
                return Err(FileError::SourceChanged);
            }
            CapturedVersion::Tree(tree)
        } else {
            let file = current.open_file(current_name)?;
            let current_identity = LiveIdentity::from_file(file.try_clone()?)?;
            if current_identity.comparison_key()? != witness.comparison_key()? {
                return Err(FileError::SourceChanged);
            }
            let version = Version::from(file.metadata()?)?;
            CapturedVersion::File(file, version)
        };
        Ok(Self {
            previous: previous.duplicate()?,
            previous_name: previous_name.to_owned(),
            current: current.duplicate()?,
            current_name: current_name.to_owned(),
            current_location,
            witness,
            version,
        })
    }

    pub(super) fn location(&self) -> &Locator {
        &self.current_location
    }

    pub(super) fn matches_witness(&self, identity: &LiveIdentity) -> Result<bool, FileError> {
        Ok(self.witness.comparison_key()? == identity.comparison_key()?
            && self.witness.observe()? == LiveObservation::Present(self.current_location.clone()))
    }

    fn verify(&self) -> Result<(), FileError> {
        self.current.verify()?;
        if self.witness.observe()? != LiveObservation::Present(self.current_location.clone()) {
            return Err(FileError::SourceChanged);
        }
        match &self.version {
            CapturedVersion::File(file, version)
                if &Version::from(file.metadata()?)? != version =>
            {
                return Err(FileError::SourceChanged);
            }
            CapturedVersion::Tree(tree) => tree.verify(&AtomicBool::new(false))?,
            CapturedVersion::File(_, _) => {}
        }
        Ok(())
    }

    fn restore(&self, storage: &Storage) -> Result<(), FileError> {
        self.previous.verify()?;
        self.verify()?;
        move_entry(
            storage,
            &self.current,
            &self.current_name,
            &self.witness,
            &self.previous,
            &self.previous_name,
        )?;
        Ok(())
    }
    pub(super) fn permanently_remove(&self) -> Result<Option<PathBuf>, PermanentFailure> {
        self.verify()?;
        let label = OsString::from(format!(".folderai-delete-{}", Uuid::new_v4()));
        let place = self.current.path().join(&label);
        let retained = |error| PermanentFailure {
            error,
            retained_path: Some(place.clone()),
        };
        let hold = self
            .current
            .create_staging_directory(&label)
            .map_err(FileError::from)?
            .map_err(|_| retained(FileError::CleanupFailed))?;
        let payload = OsStr::new("item");
        let held_path = captured_directory_path(&hold)
            .map_err(retained)?
            .join(payload);
        let held =
            Locator::new(held_path.clone()).ok_or_else(|| retained(FileError::RecoveryPending))?;
        let current = LiveObservation::Present(self.current_location.clone());
        let observe = || self.witness.observe();
        let clean_hold = || -> Result<(), FileError> {
            hold.verify()?;
            self.current.verify()?;
            if !hold.entries()?.is_empty() {
                return Err(FileError::CleanupFailed);
            }
            native::remove_directory(&self.current, &label).map_err(|_| FileError::CleanupFailed)
        };
        if let Err(error) = self.verify() {
            return match clean_hold() {
                Ok(()) => Err(error.into()),
                Err(_) => Err(retained(FileError::CleanupFailed)),
            };
        }
        let attempt = native::publish(&self.current, &self.current_name, &hold, payload);
        let at_hold =
            || matches!(observe(), Ok(LiveObservation::Present(ref location)) if location == &held);
        if !at_hold() {
            if observe().is_ok_and(|observation| observation == current) && clean_hold().is_ok() {
                return Err(attempt
                    .err()
                    .map_or(FileError::RecoveryPending, FileError::from)
                    .into());
            }
            return Err(retained(FileError::RecoveryPending));
        }
        let mut partial = false;
        let removed = match &self.version {
            CapturedVersion::File(file, version) => (|| {
                hold.verify()?;
                let metadata = Version::from(file.metadata()?)?;
                if !version.matches_after_rename(&metadata) || !at_hold() {
                    return Err(FileError::SourceChanged);
                }
                native::remove_file(&hold, payload)?;
                Ok(())
            })(),
            CapturedVersion::Tree(tree) => {
                let mut removed = vec![false; tree.count()];
                let result = tree.remove_from_quarantine(
                    &held_path,
                    &hold,
                    payload,
                    &mut removed,
                    &AtomicBool::new(false),
                    |_| Ok(()),
                );
                partial = removed.iter().any(|entry| *entry);
                result
            }
        };
        if let Err(error) = removed {
            // Restore a verified original only if no child was removed
            // and the original name is still free.
            if !partial
                && at_hold()
                && self.current.verify().is_ok()
                && hold.verify().is_ok()
                && native::publish(&hold, payload, &self.current, &self.current_name).is_ok()
                && observe().is_ok_and(|observation| observation == current)
                && clean_hold().is_ok()
            {
                return Err(error.into());
            }
            return Err(retained(FileError::RecoveryPending));
        }
        Ok(clean_hold().err().map(|_| place))
    }
}

#[derive(Default)]
struct WindowUndo {
    action: Vec<UndoMove>,
    pending: HashMap<Uuid, Vec<Option<UndoMove>>>,
}

#[derive(Default)]
pub(crate) struct UndoSlots(Mutex<HashMap<String, Arc<Mutex<WindowUndo>>>>);

impl UndoSlots {
    fn window(&self, owner: &str) -> Result<Arc<Mutex<WindowUndo>>, FileError> {
        let mut windows = self.0.lock().map_err(|_| FileError::Unavailable)?;
        Ok(windows.entry(owner.to_owned()).or_default().clone())
    }

    pub(crate) fn forget_window(&self, owner: &str) {
        if let Ok(mut windows) = self.0.lock() {
            windows.remove(owner);
        }
    }

    pub(super) fn record(&self, app: &AppHandle, owner: &str, item: Option<UndoMove>) {
        self.record_action(app, owner, item.into_iter().collect());
    }

    pub(super) fn record_action(&self, app: &AppHandle, owner: &str, action: Vec<UndoMove>) {
        if let Ok(window) = self.window(owner)
            && let Ok(mut state) = window.lock()
        {
            state.action = action;
            notify(app, owner);
        }
    }

    pub(super) fn record_batch_item(&self, owner: &str, id: Uuid, item: Option<UndoMove>) {
        if let Ok(window) = self.window(owner)
            && let Ok(mut state) = window.lock()
        {
            state.pending.entry(id).or_default().push(item);
        }
    }

    pub(super) fn finish_batch(&self, app: &AppHandle, owner: &str, id: Uuid) {
        if let Ok(window) = self.window(owner)
            && let Ok(mut state) = window.lock()
            && let Some(items) = state.pending.remove(&id)
        {
            state.action = items.into_iter().flatten().collect();
            notify(app, owner);
        }
    }
}

fn notify(app: &AppHandle, owner: &str) {
    let _ = app.emit_to(EventTarget::webview_window(owner), "file-undo-changed", ());
}

#[derive(Serialize)]
pub(crate) struct UndoReport {
    directories: Vec<PathBuf>,
    error: Option<FileError>,
}

fn restore_last(state: &mut WindowUndo, storage: &Storage) -> UndoReport {
    let mut report = UndoReport {
        directories: Vec::new(),
        error: None,
    };
    while let Some(item) = state.action.last() {
        if let Err(error) = item.restore(storage) {
            report.error = Some(error);
            // A post-effect failure cannot safely repeat the inverse.
            if error == FileError::RecoveryPending {
                report.directories.extend([
                    item.previous.path().to_owned(),
                    item.current.path().to_owned(),
                ]);
                state.action.pop();
            }
            break;
        }
        report.directories.extend([
            item.previous.path().to_owned(),
            item.current.path().to_owned(),
        ]);
        state.action.pop();
    }
    report
}

#[tauri::command]
pub(crate) fn file_undo_available(window: WebviewWindow) -> Result<bool, FileError> {
    let state = window.state::<UndoSlots>().window(window.label())?;
    let state = state.lock().map_err(|_| FileError::Unavailable)?;
    Ok(!state.action.is_empty())
}

#[tauri::command]
pub(crate) async fn undo_last_file_action(window: WebviewWindow) -> Result<UndoReport, FileError> {
    let app = window.app_handle().clone();
    let owner = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        let window = app.state::<UndoSlots>().window(&owner)?;
        let mut state = window.lock().map_err(|_| FileError::Unavailable)?;
        let report = restore_last(&mut state, &app.state::<Storage>());
        notify(&app, &owner);
        Ok(report)
    })
    .await
    .map_err(|_| FileError::Unavailable)?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("folderai-undo-{}", Uuid::new_v4()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn storage(&self) -> Storage {
            Storage::for_test(&self.0.join("data"), &self.0.join("cache")).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn window_slot_restores_a_rename_without_overwriting_a_conflict() {
        let fixture = Fixture::new();
        let source = fixture.0.join("original.txt");
        fs::write(&source, b"original").unwrap();
        let storage = fixture.storage();
        let moved = super::super::rename(&storage, &source, OsStr::new("renamed.txt")).unwrap();
        let slots = UndoSlots::default();
        let a = slots.window("first").unwrap();
        let b = slots.window("second").unwrap();
        a.lock().unwrap().action.push(moved.undo.unwrap());
        assert!(b.lock().unwrap().action.is_empty());
        fs::write(&source, b"new occupant").unwrap();
        let report = restore_last(&mut a.lock().unwrap(), &storage);
        assert_eq!(report.error, Some(FileError::AlreadyExists));
        assert_eq!(fs::read(&source).unwrap(), b"new occupant");
        assert_eq!(fs::read(&moved.path).unwrap(), b"original");
        fs::remove_file(&source).unwrap();
        let report = restore_last(&mut a.lock().unwrap(), &storage);
        assert!(report.error.is_none());
        assert_eq!(fs::read(&source).unwrap(), b"original");
        assert!(!moved.path.exists());
        assert!(a.lock().unwrap().action.is_empty());
        slots.forget_window("first");
        assert!(
            slots
                .window("first")
                .unwrap()
                .lock()
                .unwrap()
                .action
                .is_empty()
        );
    }

    #[test]
    fn undo_preserves_a_modified_file_inside_a_renamed_folder() {
        let fixture = Fixture::new();
        let original = fixture.0.join("original");
        fs::create_dir(&original).unwrap();
        fs::write(original.join("file"), b"original").unwrap();
        let storage = fixture.storage();
        let moved = super::super::rename(&storage, &original, OsStr::new("renamed")).unwrap();
        fs::write(moved.path.join("file"), b"another version").unwrap();
        let mut state = WindowUndo::default();
        state.action.push(moved.undo.unwrap());
        let report = restore_last(&mut state, &storage);
        assert_eq!(report.error, Some(FileError::SourceChanged));
        assert!(!original.exists());
        assert_eq!(
            fs::read(moved.path.join("file")).unwrap(),
            b"another version"
        );
    }
}
