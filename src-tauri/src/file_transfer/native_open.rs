use super::{FileError, captured_directory_path, native::Directory, parse_path};
use crate::{
    local_identity::{LiveIdentity, LiveObservation, Locator},
    storage::{Storage, StorageError, Store},
};
use rusqlite::{OptionalExtension, params};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
use tauri::{Manager, WebviewWindow};

const TYPE_PREFIX: &str = "native.open.association.v1.";
const TERMINAL_PREFERENCE: &str = "native.open.terminal.v1";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApplicationChoice {
    id: String,
    name: String,
    is_system_default: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OpenChoices {
    applications: Vec<ApplicationChoice>,
    preferred_id: Option<String>,
}

#[derive(Clone, Serialize)]
struct TerminalChoice {
    id: &'static str,
    name: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TerminalChoices {
    terminals: Vec<TerminalChoice>,
    preferred_id: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct OpenResult {
    fallback: bool,
}

struct FileTarget {
    original: Locator,
    origin: Directory,
    identity: LiveIdentity,
}

impl FileTarget {
    fn capture(path: &Path) -> Result<Self, FileError> {
        let leaf = path.file_name().ok_or(FileError::InvalidRequest)?;
        let origin = Directory::open(path.parent().ok_or(FileError::InvalidRequest)?)?;
        let at = Locator::new(captured_directory_path(&origin)?.join(leaf))
            .ok_or(FileError::InvalidRequest)?;
        let identity = LiveIdentity::open(&at)?;
        if identity.kind()? != "file" {
            return Err(FileError::Unsupported);
        }
        let LiveObservation::Present(original) = identity.observe()? else {
            return Err(FileError::SourceChanged);
        };
        let captured = Self {
            original,
            origin,
            identity,
        };
        captured.verify()?;
        Ok(captured)
    }

    fn path(&self) -> &Path {
        self.original.path()
    }

    fn verify(&self) -> Result<(), FileError> {
        self.origin.verify()?;
        if self.identity.observe()? != LiveObservation::Present(self.original.clone()) {
            return Err(FileError::SourceChanged);
        }
        Ok(())
    }
}

struct Application {
    choice: ApplicationChoice,
    handler: platform::Handler,
}

struct NativeChoices {
    content_type: Option<String>,
    applications: Vec<Application>,
}

fn files(paths: Vec<String>) -> Result<Vec<FileTarget>, FileError> {
    let mut seen = BTreeSet::new();
    let mut targets = Vec::new();
    for path in paths {
        let target = FileTarget::capture(&parse_path(path)?)?;
        if seen.insert(target.path().to_owned()) {
            targets.push(target);
        }
    }
    if targets.is_empty() {
        return Err(FileError::InvalidRequest);
    }
    Ok(targets)
}

fn value(store: &Store, key: &str) -> Result<Option<String>, StorageError> {
    store
        .connection
        .query_row("SELECT value FROM preferences WHERE key=?1", [key], |row| {
            row.get::<_, String>(0)
        })
        .optional()?
        .map(|stored| {
            serde_json::from_str::<String>(&stored).map_err(|_| StorageError::Unavailable)
        })
        .transpose()
}

fn save(storage: &Storage, keys: &[String], id: Option<&str>) -> Result<(), FileError> {
    let encoded = id
        .map(|id| serde_json::to_string(id).map_err(|_| FileError::InvalidRequest))
        .transpose()?;
    storage
        .access(|store| {
            let transaction = store.connection.transaction()?;
            for key in keys {
                match &encoded {
                    Some(encoded) => {
                        transaction.execute(
                            "INSERT INTO preferences (key,value) VALUES (?1,?2)
                         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                            params![key, encoded],
                        )?;
                    }
                    None => {
                        transaction.execute("DELETE FROM preferences WHERE key=?1", [key])?;
                    }
                }
            }
            transaction.commit()?;
            Ok(())
        })
        .map_err(FileError::from)
}

fn preferences(
    storage: &Storage,
    inspections: &[NativeChoices],
) -> Result<Vec<Option<String>>, FileError> {
    storage
        .access(|store| {
            inspections
                .iter()
                .map(|items| {
                    items
                        .content_type
                        .as_ref()
                        .map(|kind| value(store, &format!("{TYPE_PREFIX}{kind}")))
                        .transpose()
                        .map(Option::flatten)
                })
                .collect()
        })
        .map_err(FileError::from)
}

fn common_choices(inspections: &[NativeChoices]) -> Vec<ApplicationChoice> {
    let Some((first, rest)) = inspections.split_first() else {
        return Vec::new();
    };
    let mut choices = Vec::new();
    for application in &first.applications {
        let mut choice = application.choice.clone();
        if rest.iter().all(|selection| {
            let compatible = selection
                .applications
                .iter()
                .find(|candidate| candidate.choice.id == choice.id);
            if let Some(candidate) = compatible {
                choice.is_system_default &= candidate.choice.is_system_default;
            }
            compatible.is_some()
        }) {
            choices.push(choice);
        }
    }
    choices.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
    choices.dedup_by(|left, right| left.id == right.id);
    choices
}

fn inspect(context: &platform::Context, targets: &[FileTarget]) -> Vec<NativeChoices> {
    targets
        .iter()
        .map(|target| platform::inspect(context, target.path()))
        .collect()
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, FileError> + Send + 'static,
) -> Result<T, FileError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|_| FileError::Unavailable)?
}

#[tauri::command]
pub(crate) async fn file_open_choices(
    window: WebviewWindow,
    paths: Vec<String>,
) -> Result<OpenChoices, FileError> {
    let app = window.app_handle().clone();
    blocking(move || {
        let targets = files(paths)?;
        let context = platform::Context::new()?;
        let inspections = inspect(&context, &targets);
        let stored = preferences(&app.state::<Storage>(), &inspections)?;
        let preferred_id = stored
            .first()
            .cloned()
            .flatten()
            .filter(|id| stored.iter().all(|same| same.as_ref() == Some(id)));
        Ok(OpenChoices {
            applications: common_choices(&inspections),
            preferred_id,
        })
    })
    .await
}

#[tauri::command]
pub(crate) async fn open_workspace_files(
    window: WebviewWindow,
    paths: Vec<String>,
    application_id: Option<String>,
) -> Result<OpenResult, FileError> {
    let app = window.app_handle().clone();
    blocking(move || {
        let targets = files(paths)?;
        let context = platform::Context::new()?;
        let inspections = inspect(&context, &targets);
        let stored = if let Some(explicit) = &application_id {
            vec![Some(explicit.clone()); targets.len()]
        } else {
            preferences(&app.state::<Storage>(), &inspections)?
        };
        let mut fallback = false;
        let handlers = inspections
            .iter()
            .zip(stored.iter())
            .map(|(inspection, id)| {
                let choice = id.as_ref().and_then(|id| {
                    inspection
                        .applications
                        .iter()
                        .find(|application| application.choice.id == *id)
                });
                if id.is_some() && choice.is_none() {
                    if application_id.is_some() {
                        return Err(FileError::Unsupported);
                    }
                    fallback = true;
                }
                Ok(choice)
            })
            .collect::<Result<Vec<_>, FileError>>()?;
        for (target, application) in targets.iter().zip(handlers) {
            target.verify()?;
            platform::open_file(&context, target.path(), application.map(|app| &app.handler))?;
        }
        Ok(OpenResult { fallback })
    })
    .await
}

#[tauri::command]
pub(crate) async fn set_file_open_preference(
    window: WebviewWindow,
    paths: Vec<String>,
    application_id: Option<String>,
) -> Result<(), FileError> {
    let app = window.app_handle().clone();
    blocking(move || {
        let targets = files(paths)?;
        let context = platform::Context::new()?;
        let inspections = inspect(&context, &targets);
        if let Some(id) = application_id.as_ref()
            && !common_choices(&inspections)
                .iter()
                .any(|choice| choice.id == *id)
        {
            return Err(FileError::Unsupported);
        }
        let keys = inspections
            .iter()
            .map(|inspection| {
                inspection
                    .content_type
                    .as_ref()
                    .map(|kind| format!("{TYPE_PREFIX}{kind}"))
                    .ok_or(FileError::Unsupported)
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        let keys = keys.into_iter().collect::<Vec<_>>();
        save(&app.state::<Storage>(), &keys, application_id.as_deref())
    })
    .await
}

#[tauri::command]
pub(crate) async fn terminal_choices(window: WebviewWindow) -> Result<TerminalChoices, FileError> {
    let app = window.app_handle().clone();
    blocking(move || {
        let terminals = platform::terminals()
            .into_iter()
            .map(|(choice, _)| choice)
            .collect();
        let preferred_id = app
            .state::<Storage>()
            .access(|store| value(store, TERMINAL_PREFERENCE))?;
        Ok(TerminalChoices {
            terminals,
            preferred_id,
        })
    })
    .await
}

#[tauri::command]
pub(crate) async fn set_terminal_preference(
    window: WebviewWindow,
    id: Option<String>,
) -> Result<(), FileError> {
    let app = window.app_handle().clone();
    blocking(move || {
        if let Some(id) = id.as_ref()
            && !platform::terminals()
                .iter()
                .any(|(choice, _)| choice.id == id)
        {
            return Err(FileError::Unsupported);
        }
        save(
            &app.state::<Storage>(),
            &[TERMINAL_PREFERENCE.to_owned()],
            id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub(crate) async fn open_workspace_terminal(
    window: WebviewWindow,
    directory: String,
    terminal_id: Option<String>,
) -> Result<OpenResult, FileError> {
    let app = window.app_handle().clone();
    blocking(move || {
        let directory = Directory::open(&parse_path(directory)?)?;
        let location = captured_directory_path(&directory)?;
        let available = platform::terminals();
        let default = available.first().ok_or(FileError::Unsupported)?;
        let explicit = terminal_id.is_some();
        let preferred = match terminal_id {
            Some(id) => Some(id),
            None => app
                .state::<Storage>()
                .access(|store| value(store, TERMINAL_PREFERENCE))?,
        };
        let selected = preferred
            .as_ref()
            .and_then(|id| available.iter().find(|(choice, _)| choice.id == id));
        if explicit && selected.is_none() {
            return Err(FileError::Unsupported);
        }
        directory.verify()?;
        platform::open_terminal(&location, &selected.unwrap_or(default).1)?;
        Ok(OpenResult {
            fallback: preferred.is_some() && selected.is_none(),
        })
    })
    .await
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use block2::RcBlock;
    use objc2::rc::{Retained, autoreleasepool};
    use objc2_app_kit::{NSRunningApplication, NSWorkspace, NSWorkspaceOpenConfiguration};
    use objc2_foundation::{NSArray, NSBundle, NSError, NSString, NSURL};
    use std::{
        ffi::{CStr, CString, OsStr},
        os::unix::ffi::OsStrExt,
        ptr::NonNull,
        sync::mpsc,
        time::Duration,
    };

    pub(super) type Handler = Retained<NSURL>;

    pub(super) struct Context;
    impl Context {
        pub(super) fn new() -> Result<Self, FileError> {
            Ok(Self)
        }
    }

    fn url(path: &Path, directory: bool) -> Result<Retained<NSURL>, FileError> {
        let value =
            CString::new(path.as_os_str().as_bytes()).map_err(|_| FileError::InvalidRequest)?;
        let pointer = NonNull::new(value.as_ptr().cast_mut()).ok_or(FileError::InvalidRequest)?;
        // The byte string stays alive until Foundation has copied it.
        Ok(unsafe {
            NSURL::fileURLWithFileSystemRepresentation_isDirectory_relativeToURL(
                pointer, directory, None,
            )
        })
    }

    fn path(url: &NSURL) -> PathBuf {
        // The returned non-null string is owned by the URL for this read.
        let bytes = unsafe { CStr::from_ptr(url.fileSystemRepresentation().as_ptr()) };
        PathBuf::from(OsStr::from_bytes(bytes.to_bytes()))
    }

    fn content_type(url: &NSURL) -> Option<String> {
        let mut value = None;
        // Launch Services' native UTI string remains usable on the supported
        // deployment range, including dynamically assigned document types.
        #[allow(deprecated)]
        unsafe {
            url.getResourceValue_forKey_error(&mut value, objc2_foundation::NSURLTypeIdentifierKey)
        }
        .ok()?;
        value?
            .downcast::<NSString>()
            .ok()
            .map(|kind| kind.to_string())
    }

    pub(super) fn inspect(_: &Context, target: &Path) -> NativeChoices {
        autoreleasepool(|_| {
            let Ok(url) = url(target, false) else {
                return NativeChoices {
                    content_type: None,
                    applications: Vec::new(),
                };
            };
            let workspace = NSWorkspace::sharedWorkspace();
            let system_default = workspace.URLForApplicationToOpenURL(&url);
            let mut choices = workspace.URLsForApplicationsToOpenURL(&url).to_vec();
            if let Some(default) = &system_default {
                choices.insert(0, default.clone());
            }
            let mut seen = BTreeSet::new();
            let applications = choices
                .into_iter()
                .filter_map(|handler| {
                    let id = NSBundle::bundleWithURL(&handler)?
                        .bundleIdentifier()?
                        .to_string();
                    if !seen.insert(id.clone()) {
                        return None;
                    }
                    let app_path = path(&handler);
                    let name = app_path.file_stem()?.to_string_lossy().into_owned();
                    let is_system_default = system_default
                        .as_ref()
                        .is_some_and(|default| path(default) == app_path);
                    Some(Application {
                        choice: ApplicationChoice {
                            id,
                            name,
                            is_system_default,
                        },
                        handler,
                    })
                })
                .collect();
            NativeChoices {
                content_type: content_type(&url),
                applications,
            }
        })
    }

    fn open(path: &Path, directory: bool, application: Option<&NSURL>) -> Result<(), FileError> {
        autoreleasepool(|_| {
            let url = url(path, directory)?;
            let workspace = NSWorkspace::sharedWorkspace();
            let settings = NSWorkspaceOpenConfiguration::configuration();
            settings.setPromptsUserIfNeeded(false);
            settings.setCreatesNewApplicationInstance(directory);
            if directory {
                settings.setAddsToRecentItems(false);
            }
            let (tx, rx) = mpsc::sync_channel(1);
            let callback =
                RcBlock::new(move |app: *mut NSRunningApplication, error: *mut NSError| {
                    let _ = tx.try_send(!app.is_null() && error.is_null());
                });
            if let Some(handler) = application {
                let urls = NSArray::from_retained_slice(&[url]);
                workspace.openURLs_withApplicationAtURL_configuration_completionHandler(
                    &urls,
                    handler,
                    &settings,
                    Some(&callback),
                );
            } else {
                workspace.openURL_configuration_completionHandler(&url, &settings, Some(&callback));
            }
            match rx.recv_timeout(Duration::from_secs(15)) {
                Ok(true) => Ok(()),
                Ok(false) => Err(FileError::Unavailable),
                Err(_) => Err(FileError::RecoveryPending),
            }
        })
    }

    pub(super) fn open_file(
        _: &Context,
        path: &Path,
        application: Option<&Handler>,
    ) -> Result<(), FileError> {
        open(path, false, application.map(|app| &**app))
    }

    pub(super) fn terminals() -> Vec<(TerminalChoice, Handler)> {
        autoreleasepool(|_| {
            let workspace = NSWorkspace::sharedWorkspace();
            ["com.apple.Terminal"]
                .into_iter()
                .filter_map(|id| {
                    let handler =
                        workspace.URLForApplicationWithBundleIdentifier(&NSString::from_str(id))?;
                    Some((
                        TerminalChoice {
                            id,
                            name: "Terminal",
                        },
                        handler,
                    ))
                })
                .collect()
        })
    }

    pub(super) fn open_terminal(directory: &Path, handler: &Handler) -> Result<(), FileError> {
        open(directory, true, Some(handler))
    }
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::{
        io,
        os::windows::{
            ffi::{OsStrExt, OsStringExt},
            process::CommandExt,
        },
        process::Command,
    };
    use windows::{
        Win32::{
            System::{
                Com::{
                    COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx,
                    CoTaskMemFree, CoUninitialize, IDataObject,
                },
                SystemInformation::GetSystemDirectoryW,
                Threading::CREATE_NEW_CONSOLE,
            },
            UI::{
                Shell::{
                    ASSOC_FILTER_NONE, ASSOCF_NONE, ASSOCSTR_EXECUTABLE, AssocQueryStringW,
                    BHID_DataObject, IAssocHandler, IShellItem, SHAssocEnumHandlers,
                    SHCreateItemFromParsingName, ShellExecuteW,
                },
                WindowsAndMessaging::SW_SHOWNORMAL,
            },
        },
        core::{Error, PCWSTR, PWSTR},
    };

    pub(super) type Handler = IAssocHandler;

    pub(super) struct Context;
    impl Context {
        pub(super) fn new() -> Result<Self, FileError> {
            // The COM apartment stays on the worker which created it.
            unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).ok() }
                .map_err(native_error)?;
            Ok(Self)
        }
    }
    impl Drop for Context {
        fn drop(&mut self) {
            // Balance every successful COM initialization on the same thread.
            unsafe { CoUninitialize() };
        }
    }

    fn wide(path: &std::ffi::OsStr) -> Vec<u16> {
        path.encode_wide().chain(std::iter::once(0)).collect()
    }

    fn allocated_string(pointer: PWSTR) -> Option<String> {
        if pointer.is_null() {
            return None;
        }
        // The shell returns a CoTaskMem-allocated NUL-terminated string.
        let result = unsafe { pointer.to_string().ok() };
        unsafe { CoTaskMemFree(Some(pointer.0.cast())) };
        result
    }

    fn default_executable(extension: PCWSTR) -> Option<String> {
        let mut length = 0;
        // Query first for the number of UTF-16 code units including the terminator.
        unsafe {
            AssocQueryStringW(
                ASSOCF_NONE,
                ASSOCSTR_EXECUTABLE,
                extension,
                PCWSTR::null(),
                None,
                &mut length,
            )
        };
        if length == 0 || length > 32_768 {
            return None;
        }
        let mut buffer = vec![0; length as usize];
        let result = unsafe {
            AssocQueryStringW(
                ASSOCF_NONE,
                ASSOCSTR_EXECUTABLE,
                extension,
                PCWSTR::null(),
                Some(PWSTR(buffer.as_mut_ptr())),
                &mut length,
            )
        };
        if result.is_err() {
            return None;
        }
        String::from_utf16(&buffer[..buffer.iter().position(|word| *word == 0)?]).ok()
    }

    pub(super) fn inspect(_: &Context, path: &Path) -> NativeChoices {
        let content_type = path
            .extension()
            .and_then(|extension| extension.to_str())
            .filter(|extension| !extension.is_empty())
            .map(|extension| format!(".{}", extension.to_lowercase()));
        let mut applications = Vec::new();
        if let Some(extension) = &content_type {
            let words = wide(extension.as_ref());
            let extension = PCWSTR(words.as_ptr());
            let system_default = default_executable(extension).map(|name| name.to_lowercase());
            // The extension string remains live throughout the native enumeration.
            if let Ok(enumerator) = unsafe { SHAssocEnumHandlers(extension, ASSOC_FILTER_NONE) } {
                let mut seen = BTreeSet::new();
                loop {
                    let mut next = [None];
                    if unsafe { enumerator.Next(&mut next, None) }.is_err() {
                        break;
                    }
                    let Some(handler) = next[0].take() else { break };
                    let id = unsafe { handler.GetName() }
                        .ok()
                        .and_then(allocated_string)
                        .map(|id| id.to_lowercase());
                    let name = unsafe { handler.GetUIName() }
                        .ok()
                        .and_then(allocated_string);
                    if let (Some(id), Some(name)) = (id, name)
                        && seen.insert(id.clone())
                    {
                        let is_system_default = system_default.as_ref() == Some(&id);
                        applications.push(Application {
                            choice: ApplicationChoice {
                                id,
                                name,
                                is_system_default,
                            },
                            handler,
                        });
                    }
                }
            }
        }
        NativeChoices {
            content_type,
            applications,
        }
    }

    pub(super) fn open_file(
        _: &Context,
        path: &Path,
        application: Option<&Handler>,
    ) -> Result<(), FileError> {
        let words = wide(path.as_os_str());
        let file = PCWSTR(words.as_ptr());
        // Passing a separate path and an empty parameters field avoids shell command parsing.
        unsafe {
            if let Some(handler) = application {
                let item: IShellItem =
                    SHCreateItemFromParsingName(file, None).map_err(native_error)?;
                let data: IDataObject = item
                    .BindToHandler(None, &BHID_DataObject)
                    .map_err(native_error)?;
                return handler.Invoke(&data).map_err(native_error);
            }
            let opened = ShellExecuteW(
                None,
                PCWSTR::null(),
                file,
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            );
            let code = opened.0 as isize;
            if code > 32 {
                Ok(())
            } else if code > 0 {
                Err(io::Error::from_raw_os_error(code as i32).into())
            } else {
                Err(FileError::Unavailable)
            }
        }
    }

    pub(super) fn terminals() -> Vec<(TerminalChoice, PathBuf)> {
        let mut buffer = vec![0; 32_768];
        // The buffer is writable for the entire OS call.
        let count = unsafe { GetSystemDirectoryW(Some(&mut buffer)) } as usize;
        if count == 0 || count >= buffer.len() {
            return Vec::new();
        }
        let system = PathBuf::from(std::ffi::OsString::from_wide(&buffer[..count]));
        [
            ("command-prompt", "Command Prompt", "cmd.exe"),
            (
                "windows-powershell",
                "Windows PowerShell",
                r"WindowsPowerShell\v1.0\powershell.exe",
            ),
        ]
        .into_iter()
        .filter_map(|(id, name, binary)| {
            let binary = system.join(binary);
            binary
                .is_file()
                .then_some((TerminalChoice { id, name }, binary))
        })
        .collect()
    }

    pub(super) fn open_terminal(directory: &Path, handler: &Path) -> Result<(), FileError> {
        let mut command = Command::new(handler);
        command
            .current_dir(directory)
            .creation_flags(CREATE_NEW_CONSOLE.0);
        if handler
            .file_name()
            .is_some_and(|name| name.eq_ignore_ascii_case("cmd.exe"))
        {
            command.arg("/d");
        }
        command.spawn()?;
        Ok(())
    }

    fn native_error(error: Error) -> FileError {
        let code = error.code().0 as u32;
        if code >> 16 == 0x8007 {
            return io::Error::from_raw_os_error((code & 0xffff) as i32).into();
        }
        FileError::Unavailable
    }
}
