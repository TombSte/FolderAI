#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod browse;
mod file_transfer;
mod filesystem;
pub mod local_identity;
mod organization;
pub mod storage;
mod workspace_controls;
mod workspace_session;
mod workspace_view;

fn main() {
    tauri::Builder::default()
        .manage(browse::BrowseRequests::default())
        .manage(file_transfer::Transfers::default())
        .manage(organization::OrganizationReferences::default())
        .manage(storage::Storage::default())
        .manage(workspace_session::Sessions::default())
        .setup(|app| {
            storage::initialize(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            browse::browse,
            browse::cancel_browse,
            workspace_session::load_workspace,
            workspace_session::save_workspace,
            workspace_session::save_startup,
            workspace_session::open_workspace_window,
            workspace_session::close_workspace_window,
            workspace_view::load_workspace_views,
            workspace_view::save_workspace_view,
            workspace_controls::load_workspace_controls,
            workspace_controls::save_workspace_controls,
            organization::load_organization,
            organization::lookup_organization_tags,
            organization::update_organization,
            file_transfer::mutate_file,
            file_transfer::copy_regular_file,
            file_transfer::stop_copy,
            file_transfer::move_file_or_directory,
        ])
        .build(tauri::generate_context!())
        .expect("build FolderAI")
        .run(workspace_session::on_run_event);
}
