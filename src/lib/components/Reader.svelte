<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import PDFViewer from "./PDFViewer.svelte";
  import EPUBViewer from "./EPUBViewer.svelte";
  import ChatPanel from "./ChatPanel.svelte";
  import BookmarkList from "./BookmarkList.svelte";
  import NotesPanel from "./NotesPanel.svelte";
  import SelectionMenu from "./SelectionMenu.svelte";
  import SearchPanel from "./SearchPanel.svelte";
  import OutlinePanel from "./OutlinePanel.svelte";
  import DictionaryPopup from "./DictionaryPopup.svelte";
  import type { Book } from "../stores/books";

  let { book, onBack }: {
    book: Book;
    onBack: () => void;
  } = $props();

  type Tab = "chat" | "bookmarks" | "notes" | "search" | "contents";

  interface OutlineItem {
    title: string;
    dest: unknown;
    items: OutlineItem[];
  }

  let activeTab = $state<Tab | null>(null);
  let selectedText = $state("");
  let currentPage = $state(book.current_page || 1);
  let totalPages = $state(book.total_pages || 0);
  let viewerError = $state("");

  let menu = $state<{ text: string; x: number; y: number } | null>(null);
  let dictPopup = $state<{ word: string; x: number; y: number } | null>(null);
  let appendRequest = $state<{ text: string; ts: number } | null>(null);
  let pdfHighlighter = $state<((color: string) => Promise<void>) | null>(null);
  let viewerSearcher = $state<
    ((q: string) => Promise<{ page: number; snippet: string }[]>) | null
  >(null);
  let viewerJumpTo = $state<((page: number) => Promise<void>) | null>(null);
  let viewerOutline = $state<OutlineItem[] | null>(null);
  let viewerNavigateToDest = $state<((dest: unknown) => Promise<void>) | null>(null);

  const SIDEBAR_KEY = "sidebar_width";
  let sidebarWidth = $state(loadSidebarWidth());
  let resizing = false;

  function loadSidebarWidth(): number {
    try {
      const v = parseInt(localStorage.getItem(SIDEBAR_KEY) ?? "");
      if (!isNaN(v) && v >= 240) return v;
    } catch {}
    return 360;
  }

  function startResize(e: PointerEvent) {
    e.preventDefault();
    resizing = true;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
    document.body.style.cursor = "ew-resize";
    document.body.style.userSelect = "none";
  }

  function onResizeMove(e: PointerEvent) {
    if (!resizing) return;
    const w = window.innerWidth - e.clientX;
    sidebarWidth = Math.max(240, Math.min(900, w));
  }

  function onResizeEnd(e: PointerEvent) {
    if (!resizing) return;
    resizing = false;
    try {
      (e.target as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {}
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
    try {
      localStorage.setItem(SIDEBAR_KEY, String(sidebarWidth));
    } catch {}
  }

  function toggleTab(tab: Tab) {
    activeTab = activeTab === tab ? null : tab;
  }

  function handlePageChange(page: number, total: number) {
    currentPage = page;
    totalPages = total;
  }

  function handleTextSelect(text: string, rect?: DOMRect) {
    if (!rect) {
      // No rect: just store text, don't show menu (fallback)
      selectedText = text;
      return;
    }
    menu = {
      text,
      x: rect.left + rect.width / 2,
      y: rect.top,
    };
  }

  function closeMenu() {
    menu = null;
  }

  function askAi() {
    if (!menu) return;
    selectedText = menu.text;
    activeTab = "chat";
    closeMenu();
  }

  function copyText() {
    if (!menu) return;
    navigator.clipboard.writeText(menu.text).catch(() => {});
    closeMenu();
  }

  function noteText() {
    if (!menu) return;
    appendRequest = { text: menu.text, ts: Date.now() };
    activeTab = "notes";
    closeMenu();
  }

  async function highlightSelection(color: string) {
    if (!menu || !pdfHighlighter) return;
    await pdfHighlighter(color);
    closeMenu();
  }

  function isSingleWord(text: string): boolean {
    return /^[a-zA-Z][a-zA-Z'-]*$/.test(text.trim());
  }

  function defineWord() {
    if (!menu) return;
    dictPopup = { word: menu.text.trim(), x: menu.x, y: menu.y };
    closeMenu();
  }

  async function handleJump(page: number) {
    if (viewerJumpTo) await viewerJumpTo(page);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
      e.preventDefault();
      activeTab = "search";
      return;
    }
    if (e.key === "Escape") {
      if (menu) closeMenu();
      else if (dictPopup) dictPopup = null;
      else if (activeTab !== null) activeTab = null;
    }
  }

  $effect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  });
</script>

<div class="reader">
  <header>
    <button class="back-btn" onclick={onBack}>← Library</button>
    <div class="book-meta">
      <span class="book-title">{book.title}</span>
      {#if book.author}<span class="author">— {book.author}</span>{/if}
    </div>
    <div class="tabs">
      <button
        class:active={activeTab === "contents"}
        onclick={() => toggleTab("contents")}
        title="Table of contents"
      >☰ Contents</button>
      <button
        class:active={activeTab === "search"}
        onclick={() => toggleTab("search")}
        title="Search (Ctrl+F)"
      >🔍 Search</button>
      <button
        class:active={activeTab === "chat"}
        onclick={() => toggleTab("chat")}
      >AI Chat</button>
      <button
        class:active={activeTab === "notes"}
        onclick={() => toggleTab("notes")}
      >Notes</button>
      <button
        class:active={activeTab === "bookmarks"}
        onclick={() => toggleTab("bookmarks")}
      >Bookmarks</button>
    </div>
  </header>

  <div class="body">
    <div class="viewer-pane">
      {#if viewerError}
        <div class="viewer-err">
          <p>Failed to load book:</p>
          <pre>{viewerError}</pre>
        </div>
      {:else if book.file_type === "pdf"}
        <PDFViewer
          {book}
          onTextSelect={handleTextSelect}
          onPageChange={handlePageChange}
          bind:highlighter={pdfHighlighter}
          bind:searcher={viewerSearcher}
          bind:jumpTo={viewerJumpTo}
          bind:outline={viewerOutline}
          bind:navigateToDest={viewerNavigateToDest}
        />
      {:else}
        <EPUBViewer
          {book}
          onTextSelect={handleTextSelect}
          onPageChange={handlePageChange}
          bind:searcher={viewerSearcher}
          bind:jumpTo={viewerJumpTo}
          bind:outline={viewerOutline}
          bind:navigateToDest={viewerNavigateToDest}
        />
      {/if}
    </div>

    {#if activeTab !== null}
      <div class="side-pane" style:width="{sidebarWidth}px">
        <div
          class="resize-handle"
          onpointerdown={startResize}
          onpointermove={onResizeMove}
          onpointerup={onResizeEnd}
          onpointercancel={onResizeEnd}
        ></div>
        {#if activeTab === "chat"}
          <ChatPanel {book} {selectedText} />
        {:else if activeTab === "notes"}
          <NotesPanel {book} {currentPage} {appendRequest} />
        {:else if activeTab === "search"}
          <SearchPanel searcher={viewerSearcher} onJump={handleJump} />
        {:else if activeTab === "contents"}
          <OutlinePanel outline={viewerOutline} navigateToDest={viewerNavigateToDest} />
        {:else}
          <BookmarkList {book} {currentPage} onJump={handleJump} />
        {/if}
      </div>
    {/if}
  </div>
</div>

{#if menu}
  <SelectionMenu
    x={menu.x}
    y={menu.y}
    onAsk={askAi}
    onNote={noteText}
    onCopy={copyText}
    onClose={closeMenu}
    onHighlight={book.file_type === "pdf" ? highlightSelection : undefined}
    onDefine={isSingleWord(menu.text) ? defineWord : undefined}
  />
{/if}

{#if dictPopup}
  <DictionaryPopup
    word={dictPopup.word}
    x={dictPopup.x}
    y={dictPopup.y}
    onClose={() => (dictPopup = null)}
  />
{/if}

<style>
  .reader {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #1e1e2e;
    color: #cdd6f4;
  }
  header {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px 20px;
    border-bottom: 1px solid #313244;
    flex-shrink: 0;
    min-width: 0;
  }
  .back-btn {
    background: #313244;
    color: #cdd6f4;
    border: none;
    padding: 7px 14px;
    border-radius: 7px;
    cursor: pointer;
    font-size: 13px;
    white-space: nowrap;
    transition: background 0.1s;
    flex-shrink: 0;
  }
  .back-btn:hover { background: #45475a; }
  .book-meta {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    overflow: hidden;
  }
  .book-title {
    font-weight: 600;
    font-size: 15px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .author {
    color: #6c7086;
    font-size: 13px;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .tabs {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }
  .tabs button {
    background: #313244;
    color: #6c7086;
    border: none;
    padding: 7px 14px;
    border-radius: 7px;
    cursor: pointer;
    font-size: 13px;
    transition: all 0.1s;
  }
  .tabs button.active {
    background: #89b4fa;
    color: #1e1e2e;
    font-weight: 600;
  }
  .tabs button:not(.active):hover {
    background: #45475a;
    color: #cdd6f4;
  }
  .body {
    display: flex;
    flex: 1;
    overflow: hidden;
  }
  .viewer-pane {
    flex: 1;
    overflow: hidden;
  }
  .side-pane {
    flex-shrink: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    border-left: 1px solid #313244;
    position: relative;
    min-width: 240px;
  }
  .resize-handle {
    position: absolute;
    left: -3px;
    top: 0;
    bottom: 0;
    width: 6px;
    cursor: ew-resize;
    z-index: 10;
    background: transparent;
    transition: background 0.15s;
  }
  .resize-handle:hover {
    background: rgba(137, 180, 250, 0.4);
  }
  .viewer-err {
    padding: 24px;
    color: #f38ba8;
    font-size: 13px;
  }
  .viewer-err pre {
    background: #181825;
    padding: 12px;
    border-radius: 6px;
    overflow: auto;
    font-size: 12px;
    white-space: pre-wrap;
  }
</style>
