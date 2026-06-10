use rusqlite::{Connection, Result};
use std::path::PathBuf;

pub fn init(db_path: &PathBuf) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS books (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            title       TEXT NOT NULL,
            author      TEXT,
            file_path   TEXT NOT NULL,
            file_type   TEXT NOT NULL,
            cover_path  TEXT,
            total_pages INTEGER,
            current_page INTEGER DEFAULT 1,
            created_at  TEXT DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS bookmarks (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            book_id    INTEGER NOT NULL,
            page       INTEGER NOT NULL,
            label      TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS notes (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            book_id    INTEGER NOT NULL,
            page       INTEGER NOT NULL,
            text       TEXT,
            strokes    TEXT,
            updated_at TEXT DEFAULT (datetime('now')),
            FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE,
            UNIQUE (book_id, page)
        );
        CREATE TABLE IF NOT EXISTS highlights (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            book_id    INTEGER NOT NULL,
            page       INTEGER NOT NULL,
            text       TEXT,
            color      TEXT NOT NULL,
            rects      TEXT NOT NULL,
            created_at TEXT DEFAULT (datetime('now')),
            FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_highlights_book_page ON highlights(book_id, page);",
    )
}
