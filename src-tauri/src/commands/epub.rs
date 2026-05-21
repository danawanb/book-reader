use rusqlite::params;
use std::{
    fs,
    io::Read,
    path::PathBuf,
};
use tauri::State;
use zip::ZipArchive;

use crate::commands::books::DbState;

#[tauri::command]
pub fn extract_epub_cover(
    book_id: i64,
    file_path: String,
    app_data_dir: String,
    state: State<DbState>,
) -> Result<String, String> {
    let file = fs::File::open(&file_path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;

    // 1. Find container.xml to get OPF path
    let opf_path = {
        let mut container = archive
            .by_name("META-INF/container.xml")
            .map_err(|_| "Not a valid EPUB: missing container.xml")?;
        let mut contents = String::new();
        container
            .read_to_string(&mut contents)
            .map_err(|e| e.to_string())?;
        extract_opf_path(&contents).ok_or("Cannot find OPF path in container.xml")?
    };

    // 2. Read OPF to find cover image path
    let cover_href = {
        let mut opf_file = archive
            .by_name(&opf_path)
            .map_err(|_| "Cannot open OPF file")?;
        let mut opf_content = String::new();
        opf_file
            .read_to_string(&mut opf_content)
            .map_err(|e| e.to_string())?;
        extract_cover_href(&opf_content)
    };

    let cover_href = cover_href.ok_or("No cover image found in EPUB")?;

    // Resolve relative path from OPF location
    let opf_dir = PathBuf::from(&opf_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let full_cover_path = if opf_dir.is_empty() {
        cover_href.clone()
    } else {
        format!("{}/{}", opf_dir, cover_href)
    };

    // 3. Extract cover image bytes
    let image_bytes = {
        let paths_to_try = [full_cover_path.clone(), cover_href.clone()];
        let mut buf = Vec::new();
        let mut found = false;
        for path in &paths_to_try {
            if let Ok(mut img_file) = archive.by_name(path) {
                img_file.read_to_end(&mut buf).map_err(|e| e.to_string())?;
                found = true;
                break;
            }
        }
        if !found {
            return Err(format!("Cover image not found in EPUB: {}", full_cover_path));
        }
        buf
    };

    // 4. Save to app data dir
    let covers_dir = PathBuf::from(&app_data_dir).join("covers");
    fs::create_dir_all(&covers_dir).map_err(|e| e.to_string())?;

    let ext = PathBuf::from(&cover_href)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg")
        .to_string();
    let cover_dest = covers_dir.join(format!("book_{}.{}", book_id, ext));
    fs::write(&cover_dest, &image_bytes).map_err(|e| e.to_string())?;

    let cover_str = cover_dest.to_string_lossy().to_string();

    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE books SET cover_path = ?1 WHERE id = ?2",
        params![cover_str, book_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(cover_str)
}

fn extract_opf_path(container_xml: &str) -> Option<String> {
    // <rootfile full-path="OEBPS/content.opf" .../>
    let marker = "full-path=\"";
    let start = container_xml.find(marker)? + marker.len();
    let end = container_xml[start..].find('"')? + start;
    Some(container_xml[start..end].to_string())
}

fn extract_cover_href(opf_content: &str) -> Option<String> {
    // Try properties="cover-image" first (EPUB 3)
    if let Some(href) = find_attr_after(opf_content, "properties=\"cover-image\"", "href=\"") {
        return Some(href);
    }
    // Try id="cover-image"
    if let Some(href) = find_attr_after(opf_content, "id=\"cover-image\"", "href=\"") {
        return Some(href);
    }
    // Try id="cover"
    if let Some(href) = find_attr_after(opf_content, "id=\"cover\"", "href=\"") {
        // Only return if it looks like an image
        if href.ends_with(".jpg")
            || href.ends_with(".jpeg")
            || href.ends_with(".png")
            || href.ends_with(".gif")
            || href.ends_with(".webp")
        {
            return Some(href);
        }
    }
    // Fall back: find <meta name="cover" content="..."/> then look up that id
    let meta_marker = "name=\"cover\"";
    if let Some(pos) = opf_content.find(meta_marker) {
        if let Some(content_id) = find_attr_in_slice(&opf_content[pos..], "content=\"") {
            // Find item with that id
            let id_attr = format!("id=\"{}\"", content_id);
            if let Some(href) = find_attr_after(opf_content, &id_attr, "href=\"") {
                return Some(href);
            }
        }
    }
    None
}

fn find_attr_after(haystack: &str, after: &str, attr: &str) -> Option<String> {
    let pos = haystack.find(after)?;
    // Search within a reasonable window around the match
    let window_start = pos.saturating_sub(200);
    let window_end = (pos + after.len() + 400).min(haystack.len());
    let window = &haystack[window_start..window_end];
    find_attr_in_slice(window, attr)
}

fn find_attr_in_slice(slice: &str, attr: &str) -> Option<String> {
    let start = slice.find(attr)? + attr.len();
    let end = slice[start..].find('"')? + start;
    Some(slice[start..end].to_string())
}
