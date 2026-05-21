<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import ePub from "epubjs";
  import { invoke, convertFileSrc } from "@tauri-apps/api/core";
  import { appDataDir } from "@tauri-apps/api/path";
  import { updateProgress } from "../stores/books";
  import type { Book } from "../stores/books";

  let {
    book,
    onTextSelect,
    onPageChange,
    searcher = $bindable(null),
    jumpTo = $bindable(null),
    outline = $bindable(null),
    navigateToDest = $bindable(null),
  }: {
    book: Book;
    onTextSelect: (text: string, rect?: DOMRect) => void;
    onPageChange?: (page: number, totalPages: number) => void;
    searcher?:
      | ((query: string) => Promise<{ page: number; snippet: string }[]>)
      | null;
    jumpTo?: ((page: number) => Promise<void>) | null;
    outline?: OutlineItem[] | null;
    navigateToDest?: ((dest: unknown) => Promise<void>) | null;
  } = $props();

  interface OutlineItem {
    title: string;
    dest: unknown;
    items: OutlineItem[];
  }

  let viewerEl: HTMLDivElement;
  let rendition: any = null;
  let ebookDoc: any = null;
  let currentPage = $state(book.current_page || 1);
  let totalPages = $state(0);
  let coverExtracted = false;
  let saveProgress: ReturnType<typeof setTimeout> | null = null;

  async function getAppDataDir(): Promise<string> {
    return appDataDir();
  }

  async function loadEpub() {
    const fileUrl = convertFileSrc(book.file_path);
    ebookDoc = ePub(fileUrl);
    rendition = ebookDoc.renderTo(viewerEl, {
      width: "100%",
      height: "100%",
      spread: "none",
    });

    await ebookDoc.ready;

    // Register & apply dark theme inside the iframe
    rendition.themes.register("dark", {
      body: {
        background: "#1e1e2e !important",
        color: "#cdd6f4 !important",
      },
      "p, span, div, li, td, th, h1, h2, h3, h4, h5, h6, blockquote": {
        color: "#cdd6f4 !important",
      },
      "a, a:link, a:visited": { color: "#89b4fa !important" },
      "a:hover": { color: "#74c7ec !important" },
      "code, pre": {
        background: "#11111b !important",
        color: "#f5e0dc !important",
      },
      "::selection": { background: "rgba(137, 180, 250, 0.4) !important" },
    });
    rendition.themes.select("dark");

    // Extract cover if not done
    if (!book.cover_path && !coverExtracted) {
      coverExtracted = true;
      const appDataDir = await getAppDataDir();
      await invoke("extract_epub_cover", {
        bookId: book.id,
        filePath: book.file_path,
        appDataDir,
      }).catch(() => {});
    }

    // Generate locations for pagination
    await ebookDoc.locations.generate(1000);
    totalPages = ebookDoc.locations.total || 0;

    // Display from saved position
    const startCfi = currentPage > 1
      ? ebookDoc.locations.cfiFromPercentage((currentPage - 1) / Math.max(1, totalPages - 1))
      : undefined;

    if (startCfi) {
      await rendition.display(startCfi);
    } else {
      await rendition.display();
    }

    // Expose searcher + jumpTo + outline + navigate after epub ready
    searcher = searchEpub;
    jumpTo = async (page: number) => {
      if (!ebookDoc || totalPages === 0) return;
      const pct = (page - 1) / Math.max(1, totalPages - 1);
      const cfi = ebookDoc.locations.cfiFromPercentage(Math.max(0, Math.min(1, pct)));
      if (cfi) await rendition.display(cfi);
    };
    navigateToDest = async (dest: unknown) => {
      if (!rendition || typeof dest !== "string") return;
      try {
        await rendition.display(dest);
      } catch (e) {
        console.warn("EPUB navigate failed:", e);
      }
    };

    // Load TOC
    try {
      const toc = (ebookDoc.navigation as any)?.toc ?? [];
      outline = mapEpubToc(toc);
    } catch {
      outline = [];
    }

    rendition.on("relocated", (location: any) => {
      const pct = ebookDoc.locations.percentageFromCfi(location.start.cfi);
      currentPage = Math.round(pct * totalPages) + 1;
      onPageChange?.(currentPage, totalPages);
      debounceSaveProgress(currentPage);
    });

    // Text selection
    rendition.on("selected", (_cfiRange: any, contents: any) => {
      const sel = contents.window.getSelection();
      const text = sel?.toString().trim();
      if (!text) return;

      // Get rect in iframe coordinates, then translate to viewport
      let rect: DOMRect | undefined;
      try {
        const range = sel.getRangeAt(0);
        const iframeRect = (contents.document.defaultView.frameElement as HTMLIFrameElement)
          ?.getBoundingClientRect();
        const r = range.getBoundingClientRect();
        if (iframeRect) {
          rect = new DOMRect(
            r.left + iframeRect.left,
            r.top + iframeRect.top,
            r.width,
            r.height
          );
        }
      } catch {}

      onTextSelect(text, rect);
    });
  }

  function debounceSaveProgress(page: number) {
    if (saveProgress) clearTimeout(saveProgress);
    saveProgress = setTimeout(() => {
      updateProgress(book.id, page, totalPages);
    }, 800);
  }

  async function prev() {
    await rendition?.prev();
  }

  async function next() {
    await rendition?.next();
  }

  function mapEpubToc(items: any[]): OutlineItem[] {
    return items.map((it) => ({
      title: it.label?.trim() ?? "",
      dest: it.href ?? null,
      items: it.subitems ? mapEpubToc(it.subitems) : [],
    }));
  }

  async function searchEpub(query: string): Promise<{ page: number; snippet: string }[]> {
    if (!ebookDoc || !query.trim() || totalPages === 0) return [];
    const q = query.trim();
    const lowerQ = q.toLowerCase();
    const results: { page: number; snippet: string }[] = [];
    const ctxBefore = 40;
    const ctxAfter = 60;
    const maxResults = 200;

    const spine = ebookDoc.spine;
    const items: any[] = (spine as any).spineItems ?? (spine as any).items ?? [];
    for (const item of items) {
      try {
        const doc: Document = await item.load(ebookDoc.load.bind(ebookDoc));
        const text = (doc?.body?.textContent ?? "").replace(/\s+/g, " ");
        const lower = text.toLowerCase();
        let idx = 0;
        while ((idx = lower.indexOf(lowerQ, idx)) !== -1) {
          // find a CFI for the match position; fall back to item start
          const cfi = item.cfiFromRange?.(null) ?? item.cfiBase ?? "";
          const pct = cfi
            ? ebookDoc.locations.percentageFromCfi(cfi) ?? 0
            : 0;
          const page = Math.round(pct * totalPages) + 1;

          const start = Math.max(0, idx - ctxBefore);
          const end = Math.min(text.length, idx + q.length + ctxAfter);
          let snippet = text.substring(start, end).trim();
          if (start > 0) snippet = "…" + snippet;
          if (end < text.length) snippet = snippet + "…";

          results.push({ page, snippet });
          idx += q.length;
          if (results.length >= maxResults) break;
        }
      } catch (e) {
        console.warn("search section failed:", e);
      } finally {
        item.unload?.();
      }
      if (results.length >= maxResults) break;
    }
    return results;
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "ArrowRight" || e.key === "ArrowDown") next();
    if (e.key === "ArrowLeft" || e.key === "ArrowUp") prev();
  }

  onMount(() => {
    loadEpub();
    window.addEventListener("keydown", handleKeyDown);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleKeyDown);
    rendition?.destroy();
    ebookDoc?.destroy();
    if (saveProgress) clearTimeout(saveProgress);
  });
</script>

<div class="epub-viewer">
  <div class="viewer-body" bind:this={viewerEl}></div>

  <nav class="controls">
    <button class="nav" onclick={prev}>←</button>
    <span class="page-info">
      {#if totalPages > 0}
        {currentPage} / {totalPages}
      {:else}
        Loading...
      {/if}
    </span>
    <button class="nav" onclick={next}>→</button>
  </nav>
</div>

<style>
  .epub-viewer {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #1e1e2e;
  }
  .viewer-body {
    flex: 1;
    overflow: hidden;
  }
  .controls {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
    background: #313244;
    border-top: 1px solid #45475a;
    flex-shrink: 0;
  }
  .controls button.nav {
    background: rgba(137, 180, 250, 0.18);
    color: #89b4fa;
    border: 1px solid rgba(137, 180, 250, 0.35);
    padding: 6px 14px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 600;
    min-width: 38px;
    transition: background 0.12s, color 0.12s;
  }
  .controls button.nav:hover {
    background: #89b4fa;
    color: #1e1e2e;
  }
  .page-info {
    color: #cdd6f4;
    font-size: 14px;
    font-weight: 600;
    flex: 1;
    text-align: center;
  }
</style>
