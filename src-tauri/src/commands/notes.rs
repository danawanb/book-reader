use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::commands::books::DbState;

#[derive(Debug, Serialize, Deserialize)]
pub struct Note {
    pub id: i64,
    pub book_id: i64,
    pub page: i64,
    pub text: Option<String>,
    pub strokes: Option<String>,
    pub updated_at: String,
}

#[tauri::command]
pub fn get_note(book_id: i64, page: i64, state: State<DbState>) -> Result<Option<Note>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, book_id, page, text, strokes, updated_at FROM notes WHERE book_id = ?1 AND page = ?2",
        params![book_id, page],
        |row| {
            Ok(Note {
                id: row.get(0)?,
                book_id: row.get(1)?,
                page: row.get(2)?,
                text: row.get(3)?,
                strokes: row.get(4)?,
                updated_at: row.get(5)?,
            })
        },
    )
    .optional()
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_note(
    book_id: i64,
    page: i64,
    text: Option<String>,
    strokes: Option<String>,
    state: State<DbState>,
) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;

    let is_empty = text.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true)
        && strokes.as_deref().map(|s| s.trim().is_empty() || s == "[]").unwrap_or(true);

    if is_empty {
        conn.execute(
            "DELETE FROM notes WHERE book_id = ?1 AND page = ?2",
            params![book_id, page],
        )
        .map_err(|e| e.to_string())?;
        return Ok(());
    }

    conn.execute(
        "INSERT INTO notes (book_id, page, text, strokes, updated_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))
         ON CONFLICT(book_id, page) DO UPDATE SET
            text = excluded.text,
            strokes = excluded.strokes,
            updated_at = excluded.updated_at",
        params![book_id, page, text, strokes],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_note_pages(book_id: i64, state: State<DbState>) -> Result<Vec<i64>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT page FROM notes WHERE book_id = ?1 ORDER BY page")
        .map_err(|e| e.to_string())?;

    let pages = stmt
        .query_map(params![book_id], |row| row.get::<_, i64>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(pages)
}
