mod commands;
mod db;

use commands::books::DbState;
use std::{path::PathBuf, sync::Mutex};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let data_dir: PathBuf = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;

            let db_path = data_dir.join("library.db");
            let conn = db::init(&db_path).expect("Failed to initialize database");
            app.manage(DbState(Mutex::new(conn)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::books::get_all_books,
            commands::books::add_book,
            commands::books::delete_book,
            commands::books::update_progress,
            commands::books::save_pdf_cover,
            commands::bookmarks::get_bookmarks,
            commands::bookmarks::add_bookmark,
            commands::bookmarks::delete_bookmark,
            commands::epub::extract_epub_cover,
            commands::notes::get_note,
            commands::notes::save_note,
            commands::notes::get_note_pages,
            commands::highlights::add_highlight,
            commands::highlights::get_highlights,
            commands::highlights::delete_highlight,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
