<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { Book } from "../stores/books";
  import { deleteBook } from "../stores/books";

  let { book, onOpen }: { book: Book; onOpen: (b: Book) => void } = $props();

  let hovering = $state(false);
  let confirmDelete = $state(false);

  async function handleDelete(e: MouseEvent) {
    e.stopPropagation();
    if (!confirmDelete) {
      confirmDelete = true;
      setTimeout(() => (confirmDelete = false), 3000);
      return;
    }
    await deleteBook(book.id);
  }

  function coverUrl(path: string | null) {
    if (!path) return null;
    return convertFileSrc(path);
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="card"
  class:hovering
  onmouseenter={() => (hovering = true)}
  onmouseleave={() => { hovering = false; confirmDelete = false; }}
  onclick={() => onOpen(book)}
>
  <div class="cover">
    {#if book.cover_path}
      <img src={coverUrl(book.cover_path)} alt={book.title} />
    {:else}
      <div class="cover-placeholder">
        <span class="ext">{book.file_type.toUpperCase()}</span>
        <span class="title-placeholder">{book.title}</span>
      </div>
    {/if}

    {#if book.total_pages && book.total_pages > 0}
      <div class="progress-bar">
        <div
          class="progress-fill"
          style="width: {Math.min(100, (book.current_page / book.total_pages) * 100)}%"
        ></div>
      </div>
    {/if}
  </div>

  <div class="info">
    <p class="book-title">{book.title}</p>
    {#if book.author}
      <p class="author">{book.author}</p>
    {/if}
  </div>

  {#if hovering}
    <button
      class="delete-btn"
      class:confirm={confirmDelete}
      onclick={handleDelete}
      title={confirmDelete ? "Click again to confirm delete" : "Delete book"}
    >
      {confirmDelete ? "✓ Sure?" : "×"}
    </button>
  {/if}
</div>

<style>
  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 8px;
    cursor: pointer;
    transition: transform 0.15s ease;
  }
  .card.hovering {
    transform: translateY(-3px);
  }
  .cover {
    position: relative;
    width: 100%;
    aspect-ratio: 2/3;
    border-radius: 8px;
    overflow: hidden;
    background: #1e1e2e;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
  }
  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .cover-placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 16px;
    background: linear-gradient(135deg, #2a2a3e, #1a1a2e);
  }
  .ext {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 2px;
    color: #7f7fa8;
    background: #2e2e4a;
    padding: 4px 8px;
    border-radius: 4px;
  }
  .title-placeholder {
    font-size: 13px;
    color: #cdd6f4;
    text-align: center;
    line-height: 1.4;
    word-break: break-word;
  }
  .progress-bar {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 3px;
    background: rgba(255, 255, 255, 0.1);
  }
  .progress-fill {
    height: 100%;
    background: #89b4fa;
    transition: width 0.3s ease;
  }
  .info {
    padding: 0 2px;
  }
  .book-title {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: #cdd6f4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .author {
    margin: 2px 0 0;
    font-size: 11px;
    color: #6c7086;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .delete-btn {
    position: absolute;
    top: 6px;
    right: 6px;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: none;
    cursor: pointer;
    font-size: 16px;
    line-height: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s;
    background: rgba(243, 139, 168, 0.85);
    color: #1e1e2e;
  }
  .delete-btn.confirm {
    background: rgba(243, 139, 168, 1);
    font-size: 11px;
    width: auto;
    padding: 0 8px;
    border-radius: 12px;
  }
  .delete-btn:hover {
    background: #f38ba8;
  }
</style>
