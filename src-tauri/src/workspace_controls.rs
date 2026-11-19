use crate::storage::{Storage, StorageError, Store};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, WebviewWindow};

const KEY: &str = "workspace.controls.v1";
const MIN_SIDEBAR_WIDTH: u16 = 120;
const MAX_SIDEBAR_WIDTH: u16 = 480;
const MAX_TOOLBAR_ACTIONS: usize = 13;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Language {
    System,
    It,
    En,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Density {
    Normal,
    Compact,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum SidebarId {
    Locations,
    Favorites,
    Tags,
    SavedSearches,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SidebarSection {
    id: SidebarId,
    visible: bool,
    expanded: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ToolbarAction {
    NewTab,
    NewWindow,
    CloseTab,
    Split,
    Sidebar,
    Cut,
    Copy,
    Paste,
    Rename,
    Trash,
    Delete,
    NewFolder,
    NewFile,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Shortcuts {
    new_tab: String,
    new_window: String,
    close_tab: String,
    address: String,
    hidden: String,
}

impl Default for Shortcuts {
    fn default() -> Self {
        let hidden = if cfg!(target_os = "macos") {
            "Mod+Shift+Period"
        } else {
            "Mod+H"
        };
        Self {
            new_tab: "Mod+T".into(),
            new_window: "Mod+N".into(),
            close_tab: "Mod+W".into(),
            address: "Mod+L".into(),
            hidden: hidden.into(),
        }
    }
}

fn valid_shortcut(binding: &str) -> bool {
    let Some(keys) = binding.strip_prefix("Mod+") else {
        return false;
    };
    let keys = keys.strip_prefix("Alt+").unwrap_or(keys);
    let key = keys.strip_prefix("Shift+").unwrap_or(keys);
    matches!(key.as_bytes(), [b'A'..=b'Z' | b'0'..=b'9'])
        || matches!(
            key,
            "Period"
                | "Comma"
                | "Slash"
                | "Backslash"
                | "Minus"
                | "Equal"
                | "BracketLeft"
                | "BracketRight"
                | "Semicolon"
                | "Quote"
                | "Backquote"
                | "Space"
                | "Enter"
                | "Backspace"
                | "Delete"
                | "Home"
                | "End"
                | "ArrowLeft"
                | "ArrowUp"
                | "ArrowRight"
                | "ArrowDown"
                | "PageUp"
                | "PageDown"
                | "F1"
                | "F2"
                | "F3"
                | "F4"
                | "F5"
                | "F6"
                | "F7"
                | "F8"
                | "F9"
                | "F10"
                | "F11"
                | "F12"
        )
}

fn reserved_shortcut(binding: &str) -> bool {
    matches!(binding, "Mod+A" | "Mod+X" | "Mod+C" | "Mod+V")
        || (cfg!(target_os = "macos")
            && matches!(binding, "Mod+Q" | "Mod+H" | "Mod+M" | "Mod+Alt+H"))
        || (cfg!(windows) && binding == "Mod+Alt+Delete")
}

impl Shortcuts {
    fn validate(&self) -> Result<(), ControlsError> {
        let bindings = [
            self.new_tab.as_str(),
            self.new_window.as_str(),
            self.close_tab.as_str(),
            self.address.as_str(),
            self.hidden.as_str(),
        ];
        for (index, binding) in bindings.iter().copied().enumerate() {
            if !valid_shortcut(binding) {
                return Err(ControlsError::InvalidRequest);
            }
            if bindings[..index].contains(&binding) || reserved_shortcut(binding) {
                return Err(ControlsError::ShortcutConflict);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Controls {
    language: Language,
    theme: Theme,
    density: Density,
    sidebar_width: u16,
    sidebar: [SidebarSection; 4],
    shortcuts: Shortcuts,
    toolbar: Vec<ToolbarAction>,
}

impl Default for Controls {
    fn default() -> Self {
        use SidebarId::*;
        Self {
            language: Language::System,
            theme: Theme::System,
            density: Density::Normal,
            sidebar_width: 208,
            sidebar: [Locations, Favorites, Tags, SavedSearches].map(|id| SidebarSection {
                id,
                visible: true,
                expanded: true,
            }),
            shortcuts: Shortcuts::default(),
            toolbar: Vec::new(),
        }
    }
}

impl Controls {
    fn validate(&self) -> Result<(), ControlsError> {
        if !(MIN_SIDEBAR_WIDTH..=MAX_SIDEBAR_WIDTH).contains(&self.sidebar_width)
            || self.toolbar.len() > MAX_TOOLBAR_ACTIONS
        {
            return Err(ControlsError::InvalidRequest);
        }
        for (index, section) in self.sidebar.iter().enumerate() {
            if self.sidebar[..index]
                .iter()
                .any(|item| item.id == section.id)
            {
                return Err(ControlsError::InvalidRequest);
            }
        }
        for (index, action) in self.toolbar.iter().enumerate() {
            if self.toolbar[..index].contains(action) {
                return Err(ControlsError::InvalidRequest);
            }
        }
        self.shortcuts.validate()
    }
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ControlsError {
    InvalidRequest,
    ShortcutConflict,
    Unavailable,
}

fn read_controls(database: &Connection) -> Result<Controls, StorageError> {
    let contents: Option<String> = database
        .query_row("SELECT value FROM preferences WHERE key=?1", [KEY], |row| {
            row.get(0)
        })
        .optional()?;
    let Some(contents) = contents else {
        return Ok(Controls::default());
    };
    let controls: Controls =
        serde_json::from_str(&contents).map_err(|_| StorageError::Unavailable)?;
    controls.validate().map_err(|_| StorageError::Unavailable)?;
    Ok(controls)
}

fn write_controls(database: &Connection, controls: &Controls) -> Result<(), StorageError> {
    let value = serde_json::to_string(controls).map_err(|_| StorageError::Unavailable)?;
    database.execute(
        "INSERT INTO preferences (key,value) VALUES (?1,?2)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![KEY, value],
    )?;
    Ok(())
}

async fn with_store<T: Send + 'static>(
    app: AppHandle,
    work: impl FnOnce(&mut Store) -> Result<T, StorageError> + Send + 'static,
) -> Result<T, ControlsError> {
    tauri::async_runtime::spawn_blocking(move || app.state::<Storage>().access(work))
        .await
        .map_err(|_| ControlsError::Unavailable)?
        .map_err(|_| ControlsError::Unavailable)
}

#[tauri::command]
pub(crate) async fn load_workspace_controls(
    window: WebviewWindow,
) -> Result<Controls, ControlsError> {
    with_store(window.app_handle().clone(), |store| {
        read_controls(&store.connection)
    })
    .await
}

#[tauri::command]
pub(crate) async fn save_workspace_controls(
    window: WebviewWindow,
    controls: Controls,
) -> Result<(), ControlsError> {
    controls.validate()?;
    with_store(window.app_handle().clone(), move |store| {
        write_controls(&store.connection, &controls)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preferences() -> Connection {
        let database = Connection::open_in_memory().unwrap();
        database.execute_batch(
            "CREATE TABLE preferences (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL CHECK (json_valid(value))) STRICT;"
        ).unwrap();
        database
    }

    #[test]
    fn round_trip_preserves_sidebar_and_toolbar_order_without_changing_other_preferences() {
        let database = preferences();
        database
            .execute(
                "INSERT INTO preferences VALUES ('workspace.startup.v1', '{}')",
                [],
            )
            .unwrap();
        let mut controls = read_controls(&database).unwrap();
        assert_eq!(controls, Controls::default());
        assert!(controls.toolbar.is_empty());
        controls.sidebar.swap(0, 3);
        controls.sidebar[0].expanded = false;
        controls.sidebar_width = 275;
        controls.toolbar = vec![
            ToolbarAction::Split,
            ToolbarAction::NewTab,
            ToolbarAction::Cut,
        ];
        controls.theme = Theme::Dark;
        controls.language = Language::It;
        controls.density = Density::Compact;
        controls.shortcuts.hidden = "Mod+Alt+Shift+Period".into();
        controls.validate().unwrap();
        write_controls(&database, &controls).unwrap();
        assert_eq!(read_controls(&database).unwrap(), controls);
        let startup: String = database
            .query_row(
                "SELECT value FROM preferences WHERE key='workspace.startup.v1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(startup, "{}");
    }

    #[test]
    fn rejects_malformed_and_conflicting_controls() {
        let mut controls = Controls::default();
        controls.validate().unwrap();
        controls.shortcuts.address = controls.shortcuts.new_window.clone();
        assert_eq!(controls.validate(), Err(ControlsError::ShortcutConflict));
        controls.shortcuts.address = "Mod+A".into();
        assert_eq!(controls.validate(), Err(ControlsError::ShortcutConflict));
        #[cfg(target_os = "macos")]
        {
            controls.shortcuts.address = "Mod+Q".into();
            assert_eq!(controls.validate(), Err(ControlsError::ShortcutConflict));
        }
        controls.shortcuts.address = "Mod+Shift+Alt+L".into();
        assert_eq!(controls.validate(), Err(ControlsError::InvalidRequest));

        controls = Controls::default();
        controls.sidebar_width = 481;
        assert_eq!(controls.validate(), Err(ControlsError::InvalidRequest));
        controls.sidebar_width = 208;
        controls.sidebar[1].id = controls.sidebar[0].id;
        assert_eq!(controls.validate(), Err(ControlsError::InvalidRequest));
        controls = Controls::default();
        controls.toolbar = vec![ToolbarAction::Copy, ToolbarAction::Copy];
        assert_eq!(controls.validate(), Err(ControlsError::InvalidRequest));

        let mut unsupported = serde_json::to_value(Controls::default()).unwrap();
        unsupported["toolbar"] = serde_json::json!(["shell"]);
        assert!(serde_json::from_value::<Controls>(unsupported).is_err());
    }
}
