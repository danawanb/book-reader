# Book Reader


![Book-Reader](https://github.com/user-attachments/assets/9a2eaf5b-1447-48d7-8ad9-62c8c9d550f7)
![Book-Reader](https://github.com/user-attachments/assets/941c2d88-32aa-46ef-afeb-16486750a31a)

A desktop PDF/EPUB reader with built-in AI chat, per-page notes (text + drawing),
visual highlights, bookmarks, and in-book search.

Built with Tauri 2 + Svelte 5 + TypeScript.

> **🌀 Vibe-coded.** This project was built end-to-end in a freeform pair-programming
> session with Claude (Anthropic). Architecture, code, and most of the decisions came
> out of conversation rather than upfront design — no spec doc, just iteration.
> Read the code with that in mind: it's working, but it isn't a polished reference
> codebase.

## Features

### Reading
- **PDF viewer** — PDF.js with crisp rendering (DPR-aware), text selection,
  zoom (25%–400%), clickable internal links (table-of-contents navigation)
- **EPUB viewer** — epub.js with dark theme injected into book content
- **Reading progress** — last page is saved per book; resumes where you left off
- **Auto-extract metadata** — title and author pulled from PDF info / EPUB OPF

### Library
- Grid view with auto-extracted **cover images** (PDF page 1 / EPUB cover image)
- Progress bar per book
- Add books via file picker (PDF & EPUB)

### Annotations
- **Visual highlights** (PDF) — 4 colors, persist across sessions, scale with zoom
- **Bookmarks** — per page with optional label, click to jump
- **Notes per page** — text area + freehand drawing canvas with:
  - Pen, eraser, color picker, brush size
  - Pointer Events support — works with mouse, pen tablet (pressure), touchscreen
  - Strokes stored as normalized coordinates → scale correctly when sidebar resizes

### AI Chat (OpenAI)
- Streaming responses from `gpt-4o-mini` / `gpt-4o` / `gpt-4-turbo`
- Selected text auto-populates the chat input as context
- Per-book reading-assistant system prompt

### Productivity
- **In-book search** (`Ctrl+F`) — case-insensitive, snippet preview, jump to page
- **Selection menu** — appears on text select: Ask AI, Highlight, Note, Copy
- **Resizable sidebar** — drag handle; width is remembered per session
- **Keyboard nav** — `← →` to flip pages, `Esc` to close menus/sidebar

## Tech stack

- **Frontend**: Svelte 5 (runes) + TypeScript + Vite + SvelteKit
- **Desktop runtime**: Tauri 2 (Rust)
- **PDF**: `pdfjs-dist` v5
- **EPUB**: `epubjs`
- **Storage**: SQLite (`rusqlite` bundled)
- **AI**: OpenAI Chat Completions API (streaming)

## Prerequisites

- **Node.js** 20+ and **npm**
- **Rust** stable (`rustup`)
- **Tauri CLI**: `cargo install tauri-cli --version "^2"`
- **Linux**: `webkit2gtk-4.1` system package
- **Windows**: NSIS + WiX Toolset (only needed if you want `.exe` / `.msi` installers)

## Development

```bash
npm install
npm run tauri dev
```

First build downloads ~500 crates and takes a few minutes. Subsequent runs
are near-instant; the Svelte side hot-reloads.

## Build

```bash
npm run tauri build
```

Produces a native binary in `src-tauri/target/release/` and (on Linux) a
`.deb` / `.rpm` / `.AppImage` in `src-tauri/target/release/bundle/`.

## Releases (CI)

A GitHub Actions workflow (`.github/workflows/release.yml`) builds desktop
bundles automatically (Linux + Windows). Trigger it by either:

```bash
# 1. Push a version tag → workflow builds & creates a draft release
git tag v0.1.0
git push origin v0.1.0
```

…or run **Actions → release → Run workflow** from the GitHub UI and enter a
tag name. Bundles are attached to a draft release — review and publish when ready.

Windows note: CI produces `.msi` and `.exe` (NSIS) installers, but they haven't been tested yet.

### Which bundle for which distro?

The CI runner is Ubuntu 22.04, which means:

| Format       | Ubuntu / Debian | Fedora / openSUSE | Arch / others |
| ------------ | --------------- | ----------------- | ------------- |
| `.AppImage`  | ✅ Usually      | ⚠️ Often broken   | ⚠️ YMMV       |
| `.deb`       | ✅ Yes          | ❌ No             | ❌ No         |
| `.rpm`       | ❌ No           | ✅ Yes (tested)   | ⚠️ YMMV       |

**Fedora users: install the `.rpm`.** AppImage bundles its own
`libwebkit2gtk-4.1`, which crashes with `EGL_BAD_PARAMETER` on
Fedora 40+ (newer kernel + Mesa stack):

```
$ ./Book.Reader_0.1.1_amd64.AppImage
Could not create default EGL display: EGL_BAD_PARAMETER. Aborting...
```

Workarounds, in order of preference:

```bash
# Best: install the .rpm — uses your system's webkit, no bundle conflict
sudo dnf install Book.Reader-0.1.1-1.x86_64.rpm

# Or run AppImage via XWayland (often works)
GDK_BACKEND=x11 ./Book.Reader_0.1.1_amd64.AppImage

# Last resort: software rendering (slow)
LIBGL_ALWAYS_SOFTWARE=1 ./Book.Reader_0.1.1_amd64.AppImage
```

**Ubuntu / Debian users**: `.deb` or `.AppImage` both work.

## Configuration

Open **Settings** (gear icon in library) to set your OpenAI API key.
The key is stored in `localStorage`. AI chat is optional — everything else
works without it.

## Data location

User data lives in the standard Tauri app-data directory:

```
~/.local/share/com.danawan.book-reader/
├── library.db        # SQLite: books, bookmarks, notes, highlights
├── books/            # copies of imported PDF/EPUB files
└── covers/           # extracted cover images
```

To inspect:

```bash
sqlite3 ~/.local/share/com.danawan.book-reader/library.db
> .tables
> SELECT title, current_page, total_pages FROM books;
```

## Project structure

```
book-reader/
├── src/
│   ├── routes/+page.svelte          # Entry — switches Library ↔ Reader
│   ├── lib/
│   │   ├── components/
│   │   │   ├── Library.svelte        # Book grid + add
│   │   │   ├── BookCard.svelte       # Cover + progress
│   │   │   ├── Reader.svelte         # Viewer + sidebar layout
│   │   │   ├── PDFViewer.svelte      # PDF.js render + highlights + links
│   │   │   ├── EPUBViewer.svelte     # epub.js render + dark theme
│   │   │   ├── ChatPanel.svelte      # OpenAI streaming chat
│   │   │   ├── NotesPanel.svelte     # Text + drawing canvas
│   │   │   ├── BookmarkList.svelte
│   │   │   ├── SearchPanel.svelte
│   │   │   ├── SelectionMenu.svelte  # Floating selection popup
│   │   │   └── Settings.svelte
│   │   ├── stores/ (books, settings)
│   │   └── api/ (openai, pdf, epub helpers)
│   └── app.html
└── src-tauri/
    ├── src/
    │   ├── lib.rs                    # Tauri setup + command registration
    │   ├── db.rs                     # SQLite schema + migrations
    │   └── commands/
    │       ├── books.rs              # add/get/delete book, progress, cover
    │       ├── bookmarks.rs
    │       ├── notes.rs              # per-page notes (text + strokes)
    │       ├── highlights.rs         # PDF highlights with rects
    │       └── epub.rs               # EPUB OPF parsing + cover extraction
    ├── capabilities/default.json     # Tauri v2 permissions
    └── tauri.conf.json
```

## Database schema

```sql
books       (id, title, author, file_path, file_type, cover_path,
             total_pages, current_page, created_at)
bookmarks   (id, book_id, page, label, created_at)
notes       (id, book_id, page, text, strokes, updated_at)   -- UNIQUE(book_id, page)
highlights  (id, book_id, page, text, color, rects, created_at)
```

## Keyboard shortcuts

| Shortcut         | Action                                     |
| ---------------- | ------------------------------------------ |
| `← / →`          | Previous / next page                       |
| `↑ / ↓`          | Same as `← / →`                            |
| `Ctrl+F`         | Open search panel                          |
| `Esc`            | Close selection menu / close sidebar       |

## Notes

- **Drawing strokes use normalized coordinates** (0–1 of canvas size), so
  resizing the sidebar doesn't distort existing drawings.
- **PDF highlight rects** are stored at `scale=1` and multiplied by current
  zoom at render time — they always sit exactly on the text.
- **EPUB highlights are not yet supported** (would require CFI-based marking).
- Only the first cover render is saved (PDF page 1). Closing the reader
  before page 1 finishes rendering means no cover gets stored.

## License

MIT
