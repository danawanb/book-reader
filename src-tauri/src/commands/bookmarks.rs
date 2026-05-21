use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::commands::books::DbState;

#[derive(Debug, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: i64,
    pub book_id: i64,
    pub page: i64,
    pub label: Option<String>,
    pub created_at: String,
}

#[tauri::command]
pub fn get_bookmarks(book_id: i64, state: State<DbState>) -> Result<Vec<Bookmark>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, book_id, page, label, created_at FROM bookmarks WHERE book_id = ?1 ORDER BY page",
        )
        .map_err(|e| e.to_string())?;

    let items = stmt
        .query_map(params![book_id], |row| {
            Ok(Bookmark {
                id: row.get(0)?,
                book_id: row.get(1)?,
                page: row.get(2)?,
                label: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(items)
}

#[tauri::command]
pub fn add_bookmark(
    book_id: i64,
    page: i64,
    label: Option<String>,
    state: State<DbState>,
) -> Result<Bookmark, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO bookmarks (book_id, page, label) VALUES (?1, ?2, ?3)",
        params![book_id, page, label],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();
    let bm = conn
        .query_row(
            "SELECT id, book_id, page, label, created_at FROM bookmarks WHERE id = ?1",
            params![id],
            |row| {
                Ok(Bookmark {
                    id: row.get(0)?,
                    book_id: row.get(1)?,
                    page: row.get(2)?,
                    label: row.get(3)?,
                    created_at: row.get(4)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(bm)
}

#[tauri::command]
pub fn delete_bookmark(id: i64, state: State<DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM bookmarks WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
