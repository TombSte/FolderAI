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
    let builder = tauri::Builder::default();
    #[cfg(target_os = "macos")]
    let builder = builder.manage(file_transfer::ReplacementApprovals::default());
    builder
        .manage(browse::BrowseRequests::default())
        .manage(file_transfer::Transfers::default())
        .manage(file_transfer::batch::BatchTransfers::default())
        .manage(file_transfer::CopyRecoveries::default())
        .manage(file_transfer::undo::UndoSlots::default())
        .manage(file_transfer::removal::RemovalApprovals::default())
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
            file_transfer::removal::trash_files,
            file_transfer::zip_archive::extract_zip_here,
            file_transfer::zip_archive::inspect_zip,
            file_transfer::zip_archive::compress_entry_zip,
            file_transfer::removal::prepare_permanent_deletion,
            file_transfer::removal::permanently_delete_files,
            file_transfer::undo::file_undo_available,
            file_transfer::undo::undo_last_file_action,
            file_transfer::copy_regular_file,
            file_transfer::copy_directory,
            file_transfer::recover_copy_transfer,
            file_transfer::prepare_file_replacement,
            file_transfer::replace_regular_file,
            file_transfer::stop_copy,
            file_transfer::has_window_transfers,
            file_transfer::stop_window_transfers,
            file_transfer::batch::start_transfer,
            file_transfer::batch::list_transfers,
            file_transfer::batch::decide_transfer,
            file_transfer::batch::stop_transfer,
            file_transfer::move_file_or_directory,
        ])
        .build(tauri::generate_context!())
        .expect("build FolderAI")
        .run(workspace_session::on_run_event);
}
