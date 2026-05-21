<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { Book } from "../stores/books";

  let { book, onJump, currentPage }: {
    book: Book;
    onJump: (page: number) => void;
    currentPage: number;
  } = $props();

  interface Bookmark {
    id: number;
    book_id: number;
    page: number;
    label: string | null;
    created_at: string;
  }

  let bookmarks = $state<Bookmark[]>([]);
  let labelInput = $state("");

  async function load() {
    bookmarks = await invoke<Bookmark[]>("get_bookmarks", { bookId: book.id });
  }

  async function addBookmark() {
    await invoke("add_bookmark", {
      bookId: book.id,
      page: currentPage,
      label: labelInput.trim() || null,
    });
    labelInput = "";
    await load();
  }

  async function remove(id: number) {
    await invoke("delete_bookmark", { id });
    bookmarks = bookmarks.filter((b) => b.id !== id);
  }

  onMount(load);
</script>

<div class="bookmark-list">
  <div class="add-row">
    <input
      placeholder="Label (optional)"
      bind:value={labelInput}
      onkeydown={(e) => e.key === "Enter" && addBookmark()}
    />
    <button onclick={addBookmark}>+ Page {currentPage}</button>
  </div>

  {#if bookmarks.length === 0}
    <p class="empty">No bookmarks yet.</p>
  {:else}
    <ul>
      {#each bookmarks as bm (bm.id)}
        <li>
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <span class="bm-info" onclick={() => onJump(bm.page)}>
            <span class="page-num">Page {bm.page}</span>
            {#if bm.label}<span class="bm-label">{bm.label}</span>{/if}
          </span>
          <button class="remove-btn" onclick={() => remove(bm.id)}>×</button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .bookmark-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 12px;
  }
  .add-row {
    display: flex;
    gap: 6px;
  }
  .add-row input {
    flex: 1;
    background: #313244;
    border: 1px solid #45475a;
    color: #cdd6f4;
    border-radius: 6px;
    padding: 6px 10px;
    font-size: 12px;
    outline: none;
  }
  .add-row input:focus {
    border-color: #89b4fa;
  }
  .add-row button {
    background: #89b4fa;
    color: #1e1e2e;
    border: none;
    border-radius: 6px;
    padding: 6px 10px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }
  .add-row button:hover {
    background: #74c7ec;
  }
  .empty {
    color: #6c7086;
    font-size: 13px;
    text-align: center;
    margin: 12px 0;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  li {
    display: flex;
    align-items: center;
    gap: 6px;
    background: #313244;
    border-radius: 6px;
    padding: 8px 10px;
  }
  .bm-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    cursor: pointer;
  }
  .bm-info:hover .page-num {
    color: #89b4fa;
  }
  .page-num {
    font-size: 13px;
    font-weight: 600;
    color: #cdd6f4;
    transition: color 0.1s;
  }
  .bm-label {
    font-size: 11px;
    color: #6c7086;
  }
  .remove-btn {
    background: none;
    border: none;
    color: #6c7086;
    cursor: pointer;
    font-size: 18px;
    line-height: 1;
    padding: 0 2px;
    transition: color 0.1s;
  }
  .remove-btn:hover {
    color: #f38ba8;
  }
</style>
