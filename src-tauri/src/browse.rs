use crate::filesystem::{self, BrowseError, Directory, ErrorCode};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::{HashMap, hash_map::Entry},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::{Manager, State, WebviewWindow, ipc::Channel};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RequestContext {
    panel_id: String,
    tab_id: String,
    request_id: String,
}

impl RequestContext {
    fn validate(&self) -> Result<(), BrowseError> {
        let valid = [&self.panel_id, &self.tab_id, &self.request_id]
            .into_iter()
            .all(|id| {
                !id.is_empty()
                    && id.len() <= 64
                    && id
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
            });
        if valid {
            Ok(())
        } else {
            Err(BrowseError::new(ErrorCode::InvalidRequest))
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
enum BrowseTarget {
    Home,
    Path { path: String },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BrowseRequest {
    target: BrowseTarget,
    context: RequestContext,
    #[serde(default, rename = "showHidden")]
    show_hidden: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BrowseStarted<'a> {
    kind: &'static str,
    context: &'a RequestContext,
}

#[derive(Serialize)]
pub(crate) struct BrowseResponse {
    context: RequestContext,
    directory: Directory,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct WindowRequest {
    window: String,
    context: RequestContext,
}

#[derive(Default)]
pub(crate) struct BrowseRequests {
    active: Mutex<HashMap<WindowRequest, Arc<AtomicBool>>>,
}

impl BrowseRequests {
    fn start(&self, key: WindowRequest) -> Result<RunningBrowse<'_>, BrowseError> {
        let mut requests = self
            .active
            .lock()
            .map_err(|_| BrowseError::new(ErrorCode::Unavailable))?;
        let Entry::Vacant(entry) = requests.entry(key.clone()) else {
            return Err(BrowseError::new(ErrorCode::InvalidRequest));
        };
        let cancel = Arc::new(AtomicBool::new(false));
        entry.insert(Arc::clone(&cancel));
        Ok(RunningBrowse {
            requests: self,
            key,
            cancel,
        })
    }

    fn cancel(&self, key: &WindowRequest) -> Result<(), BrowseError> {
        let requests = self
            .active
            .lock()
            .map_err(|_| BrowseError::new(ErrorCode::Unavailable))?;
        if let Some(cancel) = requests.get(key) {
            cancel.store(true, Ordering::Relaxed);
        }
        Ok(())
    }
}

struct RunningBrowse<'a> {
    requests: &'a BrowseRequests,
    key: WindowRequest,
    cancel: Arc<AtomicBool>,
}

impl Drop for RunningBrowse<'_> {
    fn drop(&mut self) {
        if let Ok(mut requests) = self.requests.active.lock() {
            requests.remove(&self.key);
        }
    }
}

fn decode<T: DeserializeOwned>(input: serde_json::Value) -> Result<T, BrowseError> {
    serde_json::from_value(input).map_err(|_| BrowseError::new(ErrorCode::InvalidRequest))
}

#[tauri::command]
pub(crate) async fn browse(
    request: serde_json::Value,
    on_status: Channel<BrowseStarted<'_>>,
    window: WebviewWindow,
    requests: State<'_, BrowseRequests>,
) -> Result<BrowseResponse, BrowseError> {
    let request: BrowseRequest = decode(request)?;
    request.context.validate()?;
    let path = match request.target {
        BrowseTarget::Home => {
            let home = window
                .path()
                .home_dir()
                .map_err(|_| BrowseError::new(ErrorCode::Unavailable))?;
            filesystem::validate_path(home)?
        }
        BrowseTarget::Path { path } => filesystem::native_path(path)?,
    };

    let running = requests.start(WindowRequest {
        window: window.label().to_owned(),
        context: request.context.clone(),
    })?;
    on_status
        .send(BrowseStarted {
            kind: "started",
            context: &request.context,
        })
        .map_err(|_| BrowseError::new(ErrorCode::Unavailable))?;

    let cancel = Arc::clone(&running.cancel);
    let directory = tauri::async_runtime::spawn_blocking(move || {
        filesystem::read_directory(&path, &cancel, request.show_hidden)
    })
    .await
    .map_err(|_| BrowseError::new(ErrorCode::Unavailable))??;

    Ok(BrowseResponse {
        context: request.context,
        directory,
    })
}

#[tauri::command]
pub(crate) fn cancel_browse(
    context: serde_json::Value,
    window: WebviewWindow,
    requests: State<'_, BrowseRequests>,
) -> Result<(), BrowseError> {
    let context: RequestContext = decode(context)?;
    context.validate()?;
    requests.cancel(&WindowRequest {
        window: window.label().to_owned(),
        context,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn request(window: &str, panel: &str) -> WindowRequest {
        WindowRequest {
            window: window.into(),
            context: RequestContext {
                panel_id: panel.into(),
                tab_id: "tab".into(),
                request_id: "request".into(),
            },
        }
    }

    #[test]
    fn cancellation_belongs_to_the_window_and_browse_context() {
        let requests = BrowseRequests::default();
        let first = request("main", "left");
        let second = request("main", "right");
        let third = request("other", "left");
        let left = requests.start(first.clone()).unwrap();
        let right = requests.start(second).unwrap();
        let other_window = requests.start(third).unwrap();
        assert!(requests.start(first.clone()).is_err());

        requests.cancel(&first).unwrap();
        assert!(left.cancel.load(Ordering::Relaxed));
        assert!(!right.cancel.load(Ordering::Relaxed));
        assert!(!other_window.cancel.load(Ordering::Relaxed));
        drop(left);
        assert!(requests.start(first).is_ok());
    }

    #[test]
    fn rejects_malformed_commands_and_relative_paths() {
        assert!(decode::<BrowseRequest>(json!({"target": {"kind": "home"}})).is_err());
        let request = decode::<BrowseRequest>(json!({
            "target": {"kind": "path", "path": "relative"},
            "context": {"panelId": "", "tabId": "tab", "requestId": "request"}
        }))
        .unwrap();
        assert!(request.context.validate().is_err());
        assert!(filesystem::native_path("relative".to_owned()).is_err());
    }

    #[test]
    fn hidden_visibility_is_optional_and_strictly_typed() {
        let mut input = json!({
            "target": {"kind": "home"},
            "context": {"panelId": "panel", "tabId": "tab", "requestId": "request"}
        });
        assert!(!decode::<BrowseRequest>(input.clone()).unwrap().show_hidden);
        input["showHidden"] = json!(true);
        assert!(decode::<BrowseRequest>(input.clone()).unwrap().show_hidden);
        input["showHidden"] = json!("true");
        assert!(decode::<BrowseRequest>(input).is_err());
    }
}
