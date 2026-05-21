<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { appDataDir } from "@tauri-apps/api/path";
  import { books, loadBooks, type Book } from "../stores/books";
  import { extractPdfMetadata } from "../api/pdf";
  import { extractEpubMetadata } from "../api/epub";
  import BookCard from "./BookCard.svelte";

  let { onOpen, onSettings }: {
    onOpen: (book: Book) => void;
    onSettings: () => void;
  } = $props();

  let adding = $state(false);
  let error = $state("");

  async function handleAddBook() {
    error = "";
    const selected = await openDialog({
      multiple: false,
      filters: [{ name: "Books", extensions: ["pdf", "epub"] }],
    });
    if (!selected) return;

    adding = true;
    try {
      const srcPath = typeof selected === "string" ? selected : selected[0];
      const dataDir = await appDataDir();

      let title: string | undefined;
      let author: string | undefined;
      const lower = srcPath.toLowerCase();
      try {
        if (lower.endsWith(".pdf")) {
          const meta = await extractPdfMetadata(srcPath);
          title = meta.title;
          author = meta.author;
        } else if (lower.endsWith(".epub")) {
          const meta = await extractEpubMetadata(srcPath);
          title = meta.title;
          author = meta.author;
        }
      } catch (e) {
        console.warn("Failed to extract metadata:", e);
      }

      await invoke("add_book", {
        srcPath,
        appDataDir: dataDir,
        title,
        author,
      });
      await loadBooks();
    } catch (e) {
      error = String(e);
    } finally {
      adding = false;
    }
  }
</script>

<div class="library">
  <header>
    <h1>Book Reader</h1>
    <div class="header-actions">
      <button class="btn-icon" onclick={onSettings} title="Settings">⚙</button>
      <button class="btn-add" onclick={handleAddBook} disabled={adding}>
        {adding ? "Adding..." : "+ Add Book"}
      </button>
    </div>
  </header>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if $books.length === 0}
    <div class="empty">
      <div class="empty-icon">📚</div>
      <p>No books yet.</p>
      <p class="sub">Click <strong>+ Add Book</strong> to get started.</p>
    </div>
  {:else}
    <div class="grid">
      {#each $books as book (book.id)}
        <BookCard {book} onOpen={onOpen} />
      {/each}
    </div>
  {/if}
</div>

<style>
  .library {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #1e1e2e;
    color: #cdd6f4;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 28px;
    border-bottom: 1px solid #313244;
    flex-shrink: 0;
  }
  h1 {
    margin: 0;
    font-size: 22px;
    font-weight: 700;
    color: #cdd6f4;
  }
  .header-actions {
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .btn-add {
    background: #89b4fa;
    color: #1e1e2e;
    border: none;
    padding: 9px 18px;
    border-radius: 8px;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s;
  }
  .btn-add:hover:not(:disabled) {
    background: #74c7ec;
  }
  .btn-add:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .btn-icon {
    background: #313244;
    color: #cdd6f4;
    border: none;
    width: 36px;
    height: 36px;
    border-radius: 8px;
    font-size: 16px;
    cursor: pointer;
    transition: background 0.15s;
  }
  .btn-icon:hover {
    background: #45475a;
  }
  .error {
    background: rgba(243, 139, 168, 0.15);
    border: 1px solid #f38ba8;
    color: #f38ba8;
    padding: 12px 28px;
    font-size: 13px;
  }
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: #6c7086;
  }
  .empty-icon {
    font-size: 64px;
    margin-bottom: 8px;
  }
  .empty p {
    margin: 0;
    font-size: 16px;
  }
  .empty .sub {
    font-size: 13px;
  }
  .grid {
    padding: 28px;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 24px;
    overflow-y: auto;
    flex: 1;
  }
</style>
