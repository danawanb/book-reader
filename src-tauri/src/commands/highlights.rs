use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::commands::books::DbState;

#[derive(Debug, Serialize, Deserialize)]
pub struct Highlight {
    pub id: i64,
    pub book_id: i64,
    pub page: i64,
    pub text: Option<String>,
    pub color: String,
    pub rects: String,
    pub created_at: String,
}

#[tauri::command]
pub fn add_highlight(
    book_id: i64,
    page: i64,
    text: Option<String>,
    color: String,
    rects: String,
    state: State<DbState>,
) -> Result<Highlight, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO highlights (book_id, page, text, color, rects) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![book_id, page, text, color, rects],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, book_id, page, text, color, rects, created_at FROM highlights WHERE id = ?1",
        params![id],
        |row| {
            Ok(Highlight {
                id: row.get(0)?,
                book_id: row.get(1)?,
                page: row.get(2)?,
                text: row.get(3)?,
                color: row.get(4)?,
                rects: row.get(5)?,
                created_at: row.get(6)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_highlights(
    book_id: i64,
    page: i64,
    state: State<DbState>,
) -> Result<Vec<Highlight>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, book_id, page, text, color, rects, created_at
             FROM highlights WHERE book_id = ?1 AND page = ?2 ORDER BY id",
        )
        .map_err(|e| e.to_string())?;

    let items = stmt
        .query_map(params![book_id, page], |row| {
            Ok(Highlight {
                id: row.get(0)?,
                book_id: row.get(1)?,
                page: row.get(2)?,
                text: row.get(3)?,
                color: row.get(4)?,
                rects: row.get(5)?,
                created_at: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(items)
}

#[tauri::command]
pub fn delete_highlight(id: i64, state: State<DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM highlights WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
