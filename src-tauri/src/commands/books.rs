use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::Mutex};
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Book {
    pub id: i64,
    pub title: String,
    pub author: Option<String>,
    pub file_path: String,
    pub file_type: String,
    pub cover_path: Option<String>,
    pub total_pages: Option<i64>,
    pub current_page: i64,
    pub created_at: String,
}

pub struct DbState(pub Mutex<rusqlite::Connection>);

#[tauri::command]
pub fn get_all_books(state: State<DbState>) -> Result<Vec<Book>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, title, author, file_path, file_type, cover_path, total_pages, current_page, created_at
             FROM books ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let books = stmt
        .query_map([], |row| {
            Ok(Book {
                id: row.get(0)?,
                title: row.get(1)?,
                author: row.get(2)?,
                file_path: row.get(3)?,
                file_type: row.get(4)?,
                cover_path: row.get(5)?,
                total_pages: row.get(6)?,
                current_page: row.get(7)?,
                created_at: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(books)
}

#[tauri::command]
pub fn add_book(
    src_path: String,
    app_data_dir: String,
    title: Option<String>,
    author: Option<String>,
    state: State<DbState>,
) -> Result<Book, String> {
    let src = PathBuf::from(&src_path);
    if !src.exists() {
        return Err("File not found".into());
    }

    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    if ext != "pdf" && ext != "epub" {
        return Err("Only PDF and EPUB files are supported".into());
    }

    let books_dir = PathBuf::from(&app_data_dir).join("books");
    fs::create_dir_all(&books_dir).map_err(|e| e.to_string())?;

    let filename = src.file_name().ok_or("Invalid filename")?;
    let dest = books_dir.join(filename);

    // avoid collisions with a counter suffix
    let dest = if dest.exists() {
        let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("book");
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        books_dir.join(format!("{}_{}.{}", stem, ts, ext))
    } else {
        dest
    };

    fs::copy(&src, &dest).map_err(|e| e.to_string())?;

    let final_title = title
        .and_then(|s| {
            let t = s.trim().to_string();
            if t.is_empty() { None } else { Some(t) }
        })
        .unwrap_or_else(|| {
            src.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_string()
        });

    let final_author = author.and_then(|s| {
        let t = s.trim().to_string();
        if t.is_empty() { None } else { Some(t) }
    });

    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO books (title, author, file_path, file_type) VALUES (?1, ?2, ?3, ?4)",
        params![final_title, final_author, dest.to_string_lossy(), ext],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();
    let book = conn
        .query_row(
            "SELECT id, title, author, file_path, file_type, cover_path, total_pages, current_page, created_at
             FROM books WHERE id = ?1",
            params![id],
            |row| Ok(Book {
                id: row.get(0)?,
                title: row.get(1)?,
                author: row.get(2)?,
                file_path: row.get(3)?,
                file_type: row.get(4)?,
                cover_path: row.get(5)?,
                total_pages: row.get(6)?,
                current_page: row.get(7)?,
                created_at: row.get(8)?,
            }),
        )
        .map_err(|e| e.to_string())?;

    Ok(book)
}

#[tauri::command]
pub fn delete_book(id: i64, state: State<DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;

    let (file_path, cover_path): (String, Option<String>) = conn
        .query_row(
            "SELECT file_path, cover_path FROM books WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| e.to_string())?;

    conn.execute("DELETE FROM books WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    let _ = fs::remove_file(&file_path);
    if let Some(cp) = cover_path {
        let _ = fs::remove_file(&cp);
    }

    Ok(())
}

#[tauri::command]
pub fn update_progress(
    book_id: i64,
    page: i64,
    total_pages: i64,
    state: State<DbState>,
) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE books SET current_page = ?1, total_pages = ?2 WHERE id = ?3",
        params![page, total_pages, book_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn save_pdf_cover(
    book_id: i64,
    data_url: String,
    app_data_dir: String,
    state: State<DbState>,
) -> Result<String, String> {
    let prefix = "data:image/png;base64,";
    let b64 = data_url
        .strip_prefix(prefix)
        .ok_or("Invalid data URL")?;

    let bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        b64,
    )
    .map_err(|e| e.to_string())?;

    let covers_dir = PathBuf::from(&app_data_dir).join("covers");
    fs::create_dir_all(&covers_dir).map_err(|e| e.to_string())?;

    let cover_path = covers_dir.join(format!("book_{}.png", book_id));
    fs::write(&cover_path, &bytes).map_err(|e| e.to_string())?;

    let cover_str = cover_path.to_string_lossy().to_string();
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE books SET cover_path = ?1 WHERE id = ?2",
        params![cover_str, book_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(cover_str)
}
