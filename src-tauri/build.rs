fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "browse",
            "cancel_browse",
            "load_workspace",
            "save_workspace",
            "save_startup",
            "open_workspace_window",
            "close_workspace_window",
            "load_workspace_views",
            "save_workspace_view",
            "load_workspace_controls",
            "save_workspace_controls",
            "load_organization",
            "lookup_organization_tags",
            "update_organization",
            "mutate_file",
            "copy_regular_file",
            "stop_copy",
            "move_file_or_directory",
        ]),
    ))
    .expect("build FolderAI desktop resources");
}
