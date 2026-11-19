use crate::{
    filesystem,
    storage::{Storage, StorageError, Store},
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager, RunEvent, WebviewWindow, WebviewWindowBuilder, WindowEvent};

const MAIN: &str = "main";
const LABEL_PREFIX: &str = "workspace-";
const STARTUP_KEY: &str = "workspace.startup.v1";
const SNAPSHOT_KEY: &str = "workspace.windows.v1";
const MAX_WINDOWS: usize = 16;
const MAX_TABS: usize = 64;
const MAX_PATH_BYTES: usize = 128 * 1024;
const MAX_WINDOW_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum StartupMode {
    #[default]
    Home,
    Custom,
    Restore,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Startup {
    mode: StartupMode,
    path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TabSnapshot {
    path: Option<String>,
    selected: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PanelSnapshot {
    tabs: Vec<TabSnapshot>,
    active_tab: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WindowSnapshot {
    panels: Vec<PanelSnapshot>,
    active_panel: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoadResult {
    startup: Startup,
    window: Option<WindowSnapshot>,
    restore_incomplete: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SessionError {
    InvalidRequest,
    WindowLimit,
    Unavailable,
}

impl From<StorageError> for SessionError {
    fn from(_: StorageError) -> Self {
        Self::Unavailable
    }
}

fn validate_path(path: &str) -> Result<(), SessionError> {
    if path.len() > MAX_PATH_BYTES || filesystem::native_path(path.to_owned()).is_err() {
        return Err(SessionError::InvalidRequest);
    }
    Ok(())
}

impl Startup {
    fn validate(&self) -> Result<(), SessionError> {
        if self.mode == StartupMode::Custom && self.path.is_none() {
            return Err(SessionError::InvalidRequest);
        }
        if let Some(path) = &self.path {
            validate_path(path)?;
        }
        Ok(())
    }
}

impl WindowSnapshot {
    fn new_path(path: String) -> Self {
        Self {
            panels: vec![PanelSnapshot {
                tabs: vec![TabSnapshot {
                    path: Some(path),
                    selected: None,
                }],
                active_tab: 0,
            }],
            active_panel: 0,
        }
    }

    fn validate(&self) -> Result<(), SessionError> {
        if !(1..=2).contains(&self.panels.len()) || self.active_panel >= self.panels.len() {
            return Err(SessionError::InvalidRequest);
        }
        for panel in &self.panels {
            if panel.tabs.len() > MAX_TABS || panel.active_tab >= panel.tabs.len() {
                return Err(SessionError::InvalidRequest);
            }
            for tab in &panel.tabs {
                if let Some(path) = &tab.path {
                    validate_path(path)?;
                }
                if let Some(selected) = &tab.selected {
                    validate_path(selected)?;
                    if tab.path.is_none() {
                        return Err(SessionError::InvalidRequest);
                    }
                }
            }
        }
        if serde_json::to_vec(self)
            .map_err(|_| SessionError::InvalidRequest)?
            .len()
            > MAX_WINDOW_BYTES
        {
            return Err(SessionError::InvalidRequest);
        }
        Ok(())
    }

    fn retain_active_selection(&mut self) {
        for (panel_index, panel) in self.panels.iter_mut().enumerate() {
            for (tab_index, tab) in panel.tabs.iter_mut().enumerate() {
                if panel_index != self.active_panel || tab_index != panel.active_tab {
                    tab.selected = None;
                }
            }
        }
    }
}

type Archive = BTreeMap<String, WindowSnapshot>;

fn read_preference<T: Default + DeserializeOwned>(
    database: &Connection,
    key: &str,
) -> Result<T, StorageError> {
    let contents: Option<String> = database
        .query_row("SELECT value FROM preferences WHERE key=?1", [key], |row| {
            row.get(0)
        })
        .optional()?;
    match contents {
        Some(contents) => serde_json::from_str(&contents).map_err(|_| StorageError::Unavailable),
        None => Ok(T::default()),
    }
}

fn write_preference(
    database: &Connection,
    key: &str,
    value: &impl Serialize,
) -> Result<(), StorageError> {
    let serialized = serde_json::to_string(value).map_err(|_| StorageError::Unavailable)?;
    database.execute(
        "INSERT INTO preferences VALUES (?1,?2)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, serialized],
    )?;
    Ok(())
}

fn update_archive<T>(
    store: &mut Store,
    update: impl FnOnce(&mut Archive) -> Result<T, StorageError>,
) -> Result<T, StorageError> {
    let transaction = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    let mut archive: Archive = read_preference(&transaction, SNAPSHOT_KEY)?;
    let result = update(&mut archive)?;
    write_preference(&transaction, SNAPSHOT_KEY, &archive)?;
    transaction.commit()?;
    Ok(result)
}

fn read_restore_archive(store: &mut Store, mode: StartupMode) -> Result<Archive, StorageError> {
    let mut archive: Archive = read_preference(&store.connection, SNAPSHOT_KEY)?;
    archive.retain(|label, snapshot| {
        (label == MAIN || workspace_number(label).is_some()) && snapshot.validate().is_ok()
    });
    for snapshot in archive.values_mut() {
        snapshot.retain_active_selection();
    }
    if mode == StartupMode::Restore
        && !archive.contains_key(MAIN)
        && let Some(previous_label) = archive.keys().next().cloned()
    {
        // Reusing the static host for one previously secondary window must
        // not leave a gap where another window's save can prune that row.
        update_archive(store, |persisted| {
            if !persisted.contains_key(MAIN)
                && let Some(snapshot) = persisted.remove(&previous_label)
            {
                persisted.insert(MAIN.to_owned(), snapshot);
            }
            Ok(())
        })?;
        if let Some(snapshot) = archive.remove(&previous_label) {
            archive.insert(MAIN.to_owned(), snapshot);
        }
    }
    Ok(archive)
}

fn workspace_number(label: &str) -> Option<u64> {
    let suffix = label.strip_prefix(LABEL_PREFIX)?;
    let number: u64 = suffix.parse().ok()?;
    (number != 0 && suffix == number.to_string()).then_some(number)
}

struct Registry {
    approved: BTreeMap<String, Option<WindowSnapshot>>,
    failed_restore: BTreeSet<String>,
    restore_started: bool,
    quitting: bool,
    next_id: u64,
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            approved: BTreeMap::from([(MAIN.to_owned(), None)]),
            failed_restore: BTreeSet::new(),
            restore_started: false,
            quitting: false,
            next_id: 1,
        }
    }
}

impl Registry {
    fn start_restore(&mut self, mut archive: Archive, mode: StartupMode) -> Vec<String> {
        self.restore_started = true;
        if let Some(maximum) = archive
            .keys()
            .filter_map(|label| workspace_number(label))
            .max()
        {
            self.next_id = self.next_id.max(maximum.saturating_add(1));
        }
        if mode != StartupMode::Restore {
            return Vec::new();
        }

        self.approved.insert(MAIN.to_owned(), archive.remove(MAIN));
        let mut labels = Vec::new();
        for (label, window) in archive {
            if self.approved.len() >= MAX_WINDOWS {
                break;
            }
            if !self.approved.contains_key(&label) {
                self.approved.insert(label.clone(), Some(window));
                labels.push(label);
            }
        }
        labels
    }

    fn allocate(&mut self, window: Option<WindowSnapshot>) -> Result<String, SessionError> {
        if self.approved.len() + self.failed_restore.len() >= MAX_WINDOWS || self.quitting {
            return Err(SessionError::WindowLimit);
        }
        let label = format!("{LABEL_PREFIX}{}", self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or(SessionError::WindowLimit)?;
        if self.approved.contains_key(&label) {
            return Err(SessionError::WindowLimit);
        }
        self.approved.insert(label.clone(), window);
        Ok(label)
    }

    fn closed(&mut self, label: &str) -> bool {
        let was_open = self.approved.remove(label).is_some();
        was_open && !self.quitting && !self.approved.is_empty()
    }

    fn keep_archive_entry(&self, label: &str) -> bool {
        self.approved.contains_key(label) || self.failed_restore.contains(label)
    }
}

#[derive(Default)]
pub(crate) struct Sessions(Mutex<Registry>);

async fn with_store<T, F>(app: AppHandle, work: F) -> Result<T, SessionError>
where
    T: Send + 'static,
    F: FnOnce(&mut Store) -> Result<T, StorageError> + Send + 'static,
{
    // Storage initializes on its own worker, so we let that startup finish
    // before the first window interprets a lack of preferences as a fresh run.
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<Storage>();
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match state.access(|_| Ok(())) {
                Err(StorageError::Starting) if Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(20));
                }
                Err(error) => return Err(error.into()),
                Ok(()) => return state.access(work).map_err(SessionError::from),
            }
        }
    })
    .await
    .map_err(|_| SessionError::Unavailable)?
}

fn build_window(app: &AppHandle, label: &str) -> Result<(), SessionError> {
    let mut configuration = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == MAIN)
        .cloned()
        .ok_or(SessionError::Unavailable)?;
    configuration.label = label.to_owned();
    WebviewWindowBuilder::from_config(app, &configuration)
        .and_then(WebviewWindowBuilder::build)
        .map_err(|_| SessionError::Unavailable)
        .map(|_| ())
}

#[tauri::command]
pub(crate) async fn load_workspace(window: WebviewWindow) -> Result<LoadResult, SessionError> {
    let label = window.label().to_owned();
    let app = window.app_handle().clone();
    let data_app = app.clone();
    let (mut response, restore) = with_store(app.clone(), move |store| {
        let startup: Startup = read_preference(&store.connection, STARTUP_KEY)?;
        startup.validate().map_err(|_| StorageError::Unavailable)?;
        let state = data_app.state::<Sessions>();
        let mut registry = state.0.lock().map_err(|_| StorageError::Unavailable)?;
        if !registry.approved.contains_key(&label) {
            return Err(StorageError::Unavailable);
        }
        let restore = if label == MAIN && !registry.restore_started {
            registry.start_restore(read_restore_archive(store, startup.mode)?, startup.mode)
        } else {
            Vec::new()
        };
        let window = registry.approved.get(&label).cloned().flatten();
        let restore_incomplete = !registry.failed_restore.is_empty();
        Ok((
            LoadResult {
                startup,
                window,
                restore_incomplete,
            },
            restore,
        ))
    })
    .await?;
    for label in restore {
        if build_window(&app, &label).is_err() {
            response.restore_incomplete = true;
            if let Ok(mut registry) = app.state::<Sessions>().0.lock() {
                registry.approved.remove(&label);
                registry.failed_restore.insert(label);
            }
        }
    }
    Ok(response)
}

#[tauri::command]
pub(crate) async fn save_workspace(
    window_handle: WebviewWindow,
    mut window: WindowSnapshot,
) -> Result<(), SessionError> {
    window.validate()?;
    window.retain_active_selection();
    let label = window_handle.label().to_owned();
    let app = window_handle.app_handle().clone();
    let data_app = app.clone();
    with_store(app, move |store| {
        let state = data_app.state::<Sessions>();
        let mut registry = state.0.lock().map_err(|_| StorageError::Unavailable)?;
        if !registry.approved.contains_key(&label) {
            return Err(StorageError::Unavailable);
        }
        update_archive(store, |archive| {
            archive.retain(|name, _| registry.keep_archive_entry(name));
            archive.insert(label.clone(), window.clone());
            Ok(())
        })?;
        registry.approved.insert(label, Some(window));
        Ok(())
    })
    .await
}

#[tauri::command]
pub(crate) async fn save_startup(
    window: WebviewWindow,
    startup: Startup,
) -> Result<(), SessionError> {
    startup.validate()?;
    let label = window.label().to_owned();
    let app = window.app_handle().clone();
    let data_app = app.clone();
    with_store(app, move |store| {
        let state = data_app.state::<Sessions>();
        let registry = state.0.lock().map_err(|_| StorageError::Unavailable)?;
        if !registry.approved.contains_key(&label) {
            return Err(StorageError::Unavailable);
        }
        write_preference(&store.connection, STARTUP_KEY, &startup)
    })
    .await
}

#[tauri::command]
pub(crate) async fn open_workspace_window(
    window: WebviewWindow,
    path: Option<String>,
) -> Result<(), SessionError> {
    if let Some(path) = &path {
        validate_path(path)?;
    }
    let app = window.app_handle();
    let state = app.state::<Sessions>();
    let label = {
        let mut registry = state.0.lock().map_err(|_| SessionError::Unavailable)?;
        if !registry.approved.contains_key(window.label()) {
            return Err(SessionError::InvalidRequest);
        }
        registry.allocate(path.map(WindowSnapshot::new_path))?
    };
    if let Err(error) = build_window(app, &label) {
        if let Ok(mut registry) = state.0.lock() {
            registry.approved.remove(&label);
        }
        return Err(error);
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn close_workspace_window(window: WebviewWindow) -> Result<(), SessionError> {
    let authorized = window
        .state::<Sessions>()
        .0
        .lock()
        .map_err(|_| SessionError::Unavailable)?
        .approved
        .contains_key(window.label());
    if !authorized {
        return Err(SessionError::InvalidRequest);
    }
    window.destroy().map_err(|_| SessionError::Unavailable)
}

pub(crate) fn on_run_event(app: &AppHandle, event: RunEvent) {
    let state = app.state::<Sessions>();
    match event {
        RunEvent::ExitRequested { .. } => {
            if let Ok(mut registry) = state.0.lock() {
                registry.quitting = true;
            }
        }
        RunEvent::WindowEvent {
            label,
            event: WindowEvent::Destroyed,
            ..
        } => {
            let should_prune = state
                .0
                .lock()
                .is_ok_and(|mut registry| registry.closed(&label));
            if should_prune {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = with_store(app, move |store| {
                        update_archive(store, |archive| {
                            archive.remove(&label);
                            Ok(())
                        })
                    })
                    .await;
                });
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture_path(name: &str) -> String {
        std::env::temp_dir()
            .join(name)
            .into_os_string()
            .into_string()
            .unwrap()
    }

    fn rolling_window(name: &str) -> WindowSnapshot {
        WindowSnapshot::new_path(fixture_path(name))
    }

    fn database() -> Store {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(
            "CREATE TABLE preferences (key TEXT PRIMARY KEY, value TEXT NOT NULL CHECK(json_valid(value)));"
        ).unwrap();
        Store {
            connection,
            derived_dir: Default::default(),
        }
    }

    #[test]
    fn keeps_unavailable_paths_but_rejects_invalid_or_oversized_snapshots() {
        let mut snapshot = rolling_window("unavailable-workspace");
        assert!(snapshot.validate().is_ok());
        snapshot.panels[0].tabs[0].path = Some("relative".to_owned());
        assert!(snapshot.validate().is_err());
        snapshot = rolling_window("regular");
        snapshot.panels[0].active_tab = 1;
        assert!(snapshot.validate().is_err());
        snapshot = rolling_window("regular");
        snapshot.panels[0].tabs = vec![snapshot.panels[0].tabs[0].clone(); MAX_TABS + 1];
        assert!(snapshot.validate().is_err());
        let unknown = json!({
            "panels": [{
                "tabs": [{"path": null, "selected": null, "assistantDraft": "private"}],
                "activeTab": 0
            }],
            "activePanel": 0
        });
        assert!(serde_json::from_value::<WindowSnapshot>(unknown).is_err());
    }

    #[test]
    fn saves_only_the_active_tabs_selection() {
        let mut snapshot = rolling_window("active");
        snapshot.panels[0].tabs[0].selected = Some(fixture_path("active/file"));
        let mut inactive = snapshot.panels[0].clone();
        inactive.tabs[0].selected = Some(fixture_path("inactive/file"));
        snapshot.panels.push(inactive);
        snapshot.retain_active_selection();
        assert_eq!(
            snapshot.panels[0].tabs[0].selected,
            Some(fixture_path("active/file"))
        );
        assert!(snapshot.panels[1].tabs[0].selected.is_none());
    }

    #[test]
    fn restore_reuses_the_main_host_and_does_not_duplicate_or_overwrite_windows() {
        let mut store = database();
        let first = rolling_window("first");
        let second = rolling_window("second");
        let archive = Archive::from([
            ("workspace-1".to_owned(), first.clone()),
            ("workspace-2".to_owned(), second.clone()),
        ]);
        write_preference(&store.connection, SNAPSHOT_KEY, &archive).unwrap();
        let mut registry = Registry::default();
        let restore = read_restore_archive(&mut store, StartupMode::Restore).unwrap();
        assert_eq!(
            registry.start_restore(restore, StartupMode::Restore),
            vec!["workspace-2"]
        );
        assert_eq!(registry.approved[MAIN].as_ref(), Some(&first));
        assert_eq!(registry.approved["workspace-2"].as_ref(), Some(&second));
        let persisted: Archive = read_preference(&store.connection, SNAPSHOT_KEY).unwrap();
        assert_eq!(
            persisted,
            Archive::from([(MAIN.to_owned(), first), ("workspace-2".to_owned(), second)])
        );
        assert_eq!(registry.allocate(None).unwrap(), "workspace-3");
        assert!(registry.approved["workspace-3"].is_none());
    }

    #[test]
    fn closing_independent_windows_prunes_only_explicitly_closed_records() {
        let mut registry = Registry::default();
        let alternate = registry.allocate(None).unwrap();
        assert!(registry.closed(MAIN));
        assert!(!registry.closed(MAIN));
        assert!(!registry.closed(&alternate));
        assert!(registry.approved.is_empty());

        let mut quitting = Registry::default();
        let alternate = quitting.allocate(None).unwrap();
        quitting.quitting = true;
        assert!(!quitting.closed(MAIN));
        assert!(!quitting.closed(&alternate));
        assert!(quitting.allocate(None).is_err());
    }

    #[test]
    fn failed_capture_preserves_the_last_committed_archive() {
        let mut store = database();
        let valid = Archive::from([(MAIN.to_owned(), rolling_window("previous"))]);
        write_preference(&store.connection, SNAPSHOT_KEY, &valid).unwrap();
        store
            .connection
            .execute_batch(
                "CREATE TRIGGER reject_capture BEFORE UPDATE ON preferences
             BEGIN SELECT RAISE(ABORT, 'fault injected'); END;",
            )
            .unwrap();
        let result = update_archive(&mut store, |archive| {
            archive.insert(MAIN.to_owned(), rolling_window("next"));
            Ok(())
        });
        assert!(result.is_err());
        let saved: Archive = read_preference(&store.connection, SNAPSHOT_KEY).unwrap();
        assert_eq!(saved, valid);
    }
}
