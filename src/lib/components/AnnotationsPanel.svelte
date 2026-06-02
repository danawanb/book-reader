<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  let { bookId, bookTitle, onJump, refreshKey }: {
    bookId: number;
    bookTitle: string;
    onJump: (page: number) => void;
    refreshKey: number;
  } = $props();

  interface Highlight {
    id: number;
    book_id: number;
    page: number;
    text: string | null;
    color: string;
    rects: string;
    created_at: string;
  }

  interface Note {
    id: number;
    book_id: number;
    page: number;
    text: string | null;
    strokes: string | null;
    updated_at: string;
  }

  type Filter = "all" | "highlights" | "notes";

  interface PageGroup {
    page: number;
    highlights: Highlight[];
    note: Note | null;
  }

  let highlights = $state<Highlight[]>([]);
  let notes = $state<Note[]>([]);
  let filter = $state<Filter>("all");
  let loading = $state(true);

  $effect(() => {
    void refreshKey;
    reload();
  });

  async function reload() {
    loading = true;
    try {
      const [hs, ns] = await Promise.all([
        invoke<Highlight[]>("get_highlights_by_book", { bookId }),
        invoke<Note[]>("get_notes_by_book", { bookId }),
      ]);
      highlights = hs;
      notes = ns;
    } catch (e) {
      console.error("Failed to load annotations:", e);
    } finally {
      loading = false;
    }
  }

  const groups = $derived.by<PageGroup[]>(() => {
    const map = new Map<number, PageGroup>();
    const wantH = filter !== "notes";
    const wantN = filter !== "highlights";
    if (wantH) {
      for (const h of highlights) {
        let g = map.get(h.page);
        if (!g) {
          g = { page: h.page, highlights: [], note: null };
          map.set(h.page, g);
        }
        g.highlights.push(h);
      }
    }
    if (wantN) {
      for (const n of notes) {
        let g = map.get(n.page);
        if (!g) {
          g = { page: n.page, highlights: [], note: null };
          map.set(n.page, g);
        }
        g.note = n;
      }
    }
    return [...map.values()].sort((a, b) => a.page - b.page);
  });

  const totalHighlights = $derived(highlights.length);
  const totalNotes = $derived(notes.length);

  function noteHasSketch(n: Note): boolean {
    if (!n.strokes) return false;
    try {
      const arr = JSON.parse(n.strokes);
      return Array.isArray(arr) && arr.length > 0;
    } catch {
      return false;
    }
  }

  function truncate(s: string, max = 220): string {
    const t = s.replace(/\s+/g, " ").trim();
    return t.length > max ? t.slice(0, max - 1) + "…" : t;
  }

  const COLOR_EMOJI: Record<string, string> = {
    "#f9e2af": "🟡",
    "#a6e3a1": "🟢",
    "#89b4fa": "🔵",
    "#f38ba8": "🔴",
    "#cba6f7": "🟣",
  };

  function colorEmoji(color: string): string {
    return COLOR_EMOJI[color.toLowerCase()] ?? "🔖";
  }

  function isoDate(s: string): string {
    return s.slice(0, 10);
  }

  function slugify(s: string): string {
    return s
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 60) || "book";
  }

  function buildMarkdown(): string {
    const lines: string[] = [];
    lines.push(`# ${bookTitle}`);
    lines.push("");
    const today = new Date().toISOString().slice(0, 10);
    lines.push(`_${totalHighlights} highlights · ${totalNotes} notes · exported ${today}_`);
    lines.push("");
    lines.push("---");
    lines.push("");

    const allPages = new Map<number, PageGroup>();
    for (const h of highlights) {
      let g = allPages.get(h.page);
      if (!g) {
        g = { page: h.page, highlights: [], note: null };
        allPages.set(h.page, g);
      }
      g.highlights.push(h);
    }
    for (const n of notes) {
      let g = allPages.get(n.page);
      if (!g) {
        g = { page: n.page, highlights: [], note: null };
        allPages.set(n.page, g);
      }
      g.note = n;
    }
    const sorted = [...allPages.values()].sort((a, b) => a.page - b.page);

    for (const g of sorted) {
      lines.push(`## Page ${g.page}`);
      lines.push("");
      for (const h of g.highlights) {
        const text = (h.text ?? "").trim() || "_(no text captured)_";
        lines.push(`> ${colorEmoji(h.color)} ${text}`);
        lines.push(`> _highlighted ${isoDate(h.created_at)}_`);
        lines.push("");
      }
      if (g.note) {
        const noteText = (g.note.text ?? "").trim();
        if (noteText) {
          lines.push("**Note:**");
          lines.push("");
          lines.push(noteText);
          lines.push("");
        }
        if (noteHasSketch(g.note)) {
          lines.push("(✏️ contains sketch)");
          lines.push("");
        }
      }
      lines.push("---");
      lines.push("");
    }
    return lines.join("\n");
  }

  function exportMarkdown() {
    if (totalHighlights === 0 && totalNotes === 0) return;
    const md = buildMarkdown();
    const blob = new Blob([md], { type: "text/markdown;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${slugify(bookTitle)}-annotations.md`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    setTimeout(() => URL.revokeObjectURL(url), 100);
  }
</script>

<div class="ann-panel">
  <div class="header">
    <div class="title-row">
      <span class="title">Annotations</span>
      <button
        class="export-btn"
        onclick={exportMarkdown}
        disabled={totalHighlights === 0 && totalNotes === 0}
        title="Export to Markdown"
      >⬇ Export</button>
    </div>
    <div class="filter-row">
      <button class:active={filter === "all"} onclick={() => (filter = "all")}>
        All ({totalHighlights + totalNotes})
      </button>
      <button class:active={filter === "highlights"} onclick={() => (filter = "highlights")}>
        🎨 {totalHighlights}
      </button>
      <button class:active={filter === "notes"} onclick={() => (filter = "notes")}>
        📝 {totalNotes}
      </button>
    </div>
  </div>

  <div class="list">
    {#if loading}
      <div class="empty">Loading…</div>
    {:else if groups.length === 0}
      <div class="empty">
        {#if filter === "highlights"}
          No highlights yet.
        {:else if filter === "notes"}
          No notes yet.
        {:else}
          No annotations yet — highlight text or add notes while reading.
        {/if}
      </div>
    {:else}
      {#each groups as g (g.page)}
        <div class="page-group">
          <div class="page-header">Page {g.page}</div>
          {#each g.highlights as h (h.id)}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="item highlight" onclick={() => onJump(g.page)}>
              <span class="color-dot" style="background: {h.color};"></span>
              <span class="item-text">
                {#if h.text && h.text.trim()}
                  {truncate(h.text)}
                {:else}
                  <span class="muted">(no text captured)</span>
                {/if}
              </span>
            </div>
          {/each}
          {#if g.note}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="item note" onclick={() => onJump(g.page)}>
              <span class="note-icon">📝</span>
              <span class="item-text">
                {#if g.note.text && g.note.text.trim()}
                  {truncate(g.note.text)}
                {:else}
                  <span class="muted">(sketch only)</span>
                {/if}
                {#if noteHasSketch(g.note)}
                  <span class="sketch-badge" title="Contains sketch">✏️</span>
                {/if}
              </span>
            </div>
          {/if}
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .ann-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #1e1e2e;
  }
  .header {
    border-bottom: 1px solid #313244;
    background: #181825;
    flex-shrink: 0;
  }
  .title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px 6px;
  }
  .title {
    color: #cdd6f4;
    font-size: 13px;
    font-weight: 600;
  }
  .export-btn {
    background: #313244;
    color: #cdd6f4;
    border: none;
    padding: 5px 11px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 12px;
    font-family: inherit;
    transition: background 0.1s;
  }
  .export-btn:hover:not(:disabled) {
    background: #45475a;
  }
  .export-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .filter-row {
    display: flex;
    gap: 5px;
    padding: 0 12px 10px;
  }
  .filter-row button {
    background: #313244;
    color: #6c7086;
    border: none;
    padding: 4px 10px;
    border-radius: 12px;
    cursor: pointer;
    font-size: 11px;
    font-family: inherit;
    transition: all 0.1s;
  }
  .filter-row button:hover {
    background: #45475a;
    color: #cdd6f4;
  }
  .filter-row button.active {
    background: #89b4fa;
    color: #1e1e2e;
    font-weight: 600;
  }
  .list {
    flex: 1;
    overflow-y: auto;
    padding: 8px 0;
  }
  .empty {
    color: #6c7086;
    font-size: 12px;
    padding: 24px 14px;
    text-align: center;
    line-height: 1.5;
  }
  .page-group {
    margin-bottom: 8px;
  }
  .page-header {
    padding: 6px 14px;
    color: #f9e2af;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.3px;
    text-transform: uppercase;
    background: rgba(24, 24, 37, 0.5);
    position: sticky;
    top: 0;
    z-index: 1;
  }
  .item {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 14px;
    cursor: pointer;
    transition: background 0.1s;
    border-left: 3px solid transparent;
  }
  .item:hover {
    background: #313244;
    border-left-color: #89b4fa;
  }
  .color-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    margin-top: 4px;
    flex-shrink: 0;
    border: 1px solid rgba(255, 255, 255, 0.15);
  }
  .note-icon {
    flex-shrink: 0;
    font-size: 13px;
    line-height: 1.3;
    margin-top: 1px;
  }
  .item-text {
    flex: 1;
    color: #cdd6f4;
    font-size: 12px;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }
  .muted {
    color: #6c7086;
    font-style: italic;
  }
  .sketch-badge {
    margin-left: 6px;
    font-size: 11px;
    opacity: 0.7;
  }
</style>
