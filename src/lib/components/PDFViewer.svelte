<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import * as pdfjsLib from "pdfjs-dist";
  import { invoke, convertFileSrc } from "@tauri-apps/api/core";
  import { appDataDir } from "@tauri-apps/api/path";
  import { updateProgress } from "../stores/books";
  import type { Book } from "../stores/books";

  let {
    book,
    onTextSelect,
    onPageChange,
    highlighter = $bindable(null),
    searcher = $bindable(null),
    jumpTo = $bindable(null),
  }: {
    book: Book;
    onTextSelect: (text: string, rect?: DOMRect) => void;
    onPageChange?: (page: number, totalPages: number) => void;
    highlighter?: ((color: string) => Promise<void>) | null;
    searcher?:
      | ((query: string) => Promise<{ page: number; snippet: string }[]>)
      | null;
    jumpTo?: ((page: number) => Promise<void>) | null;
  } = $props();

  interface Highlight {
    id: number;
    page: number;
    color: string;
    text: string | null;
    rects: { x: number; y: number; w: number; h: number }[];
  }

  let highlights = $state<Highlight[]>([]);
  let highlightLayer: HTMLDivElement;
  let lastSelection: { text: string; rects: DOMRect[] } | null = null;

  // Use bundled worker via Vite import
  pdfjsLib.GlobalWorkerOptions.workerSrc = new URL(
    "pdfjs-dist/build/pdf.worker.min.mjs",
    import.meta.url
  ).toString();

  let canvas: HTMLCanvasElement;
  let textLayerDiv: HTMLDivElement;
  let annotationLayerDiv: HTMLDivElement;
  let container: HTMLDivElement;
  let pdfDoc: pdfjsLib.PDFDocumentProxy | null = null;
  let currentPage = $state(book.current_page || 1);
  let totalPages = $state(0);
  let scale = $state(1.5);
  let rendering = $state(false);
  let coverSaved = false;
  let saveProgress: ReturnType<typeof setTimeout> | null = null;
  let loadError = $state("");

  async function loadPDF() {
    try {
    const fileUrl = convertFileSrc(book.file_path);
    pdfDoc = await pdfjsLib.getDocument(fileUrl).promise;
    totalPages = pdfDoc.numPages;
    await renderPage(currentPage);
    } catch (e) {
      loadError = String(e);
      console.error("PDFViewer error:", e);
    }
  }

  async function renderPage(num: number) {
    if (!pdfDoc || rendering) return;
    rendering = true;
    try {
      const page = await pdfDoc.getPage(num);
      const dpr = window.devicePixelRatio || 1;
      // CSS-size viewport (layout) vs render viewport (pixel buffer)
      const cssViewport = page.getViewport({ scale });
      const renderViewport = page.getViewport({ scale: scale * dpr });
      const ctx = canvas.getContext("2d")!;

      canvas.width = Math.floor(renderViewport.width);
      canvas.height = Math.floor(renderViewport.height);
      canvas.style.width = `${Math.floor(cssViewport.width)}px`;
      canvas.style.height = `${Math.floor(cssViewport.height)}px`;

      // Match overlay layers to CSS size of canvas
      const cssW = `${Math.floor(cssViewport.width)}px`;
      const cssH = `${Math.floor(cssViewport.height)}px`;
      textLayerDiv.style.width = cssW;
      textLayerDiv.style.height = cssH;
      textLayerDiv.innerHTML = "";
      textLayerDiv.style.setProperty("--scale-factor", String(cssViewport.scale));
      annotationLayerDiv.style.width = cssW;
      annotationLayerDiv.style.height = cssH;
      annotationLayerDiv.innerHTML = "";

      await page.render({ canvasContext: ctx, viewport: renderViewport, canvas }).promise;

      // Render text layer for selection/copy (using CSS-size viewport)
      const textContent = await page.getTextContent();
      const textLayer = new pdfjsLib.TextLayer({
        textContentSource: textContent,
        container: textLayerDiv,
        viewport: cssViewport,
      });
      await textLayer.render();

      // Render clickable link annotations
      await renderLinkAnnotations(page, cssViewport);

      if (!coverSaved && num === 1) {
        coverSaved = true;
        const dataUrl = canvas.toDataURL("image/png");
        const appDataDir = await getAppDataDir();
        await invoke("save_pdf_cover", { bookId: book.id, dataUrl, appDataDir }).catch(() => {});
      }

      onPageChange?.(num, totalPages);
      debounceSaveProgress(num);
      loadHighlights(num);
    } finally {
      rendering = false;
    }
  }

  async function renderLinkAnnotations(page: any, viewport: any) {
    const annotations = await page.getAnnotations();
    for (const ann of annotations) {
      if (ann.subtype !== "Link") continue;

      // Convert PDF rect [x1,y1,x2,y2] (origin bottom-left) to viewport rect
      const rect = pdfjsLib.Util.normalizeRect(
        viewport.convertToViewportRectangle(ann.rect)
      );

      const el = document.createElement("a");
      el.className = "pdf-link";
      el.style.left = `${rect[0]}px`;
      el.style.top = `${rect[1]}px`;
      el.style.width = `${rect[2] - rect[0]}px`;
      el.style.height = `${rect[3] - rect[1]}px`;

      if (ann.url) {
        el.href = ann.url;
        el.target = "_blank";
        el.rel = "noopener";
        el.title = ann.url;
      } else if (ann.dest || ann.action === "GoTo" || ann.unsafeUrl) {
        el.href = "#";
        el.title = "Go to page";
        el.addEventListener("click", async (e) => {
          e.preventDefault();
          try {
            let dest = ann.dest;
            if (typeof dest === "string") {
              dest = await pdfDoc!.getDestination(dest);
            }
            if (!dest || !Array.isArray(dest)) return;
            const idx = await pdfDoc!.getPageIndex(dest[0]);
            await goTo(idx + 1);
          } catch (err) {
            console.warn("Link nav failed:", err);
          }
        });
      } else {
        continue;
      }
      annotationLayerDiv.appendChild(el);
    }
  }

  async function getAppDataDir(): Promise<string> {
    return appDataDir();
  }

  function debounceSaveProgress(page: number) {
    if (saveProgress) clearTimeout(saveProgress);
    saveProgress = setTimeout(() => {
      updateProgress(book.id, page, totalPages);
    }, 800);
  }

  async function goTo(page: number) {
    if (!pdfDoc) return;
    const clamped = Math.max(1, Math.min(totalPages, page));
    currentPage = clamped;
    await renderPage(currentPage);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "ArrowRight" || e.key === "ArrowDown") goTo(currentPage + 1);
    if (e.key === "ArrowLeft" || e.key === "ArrowUp") goTo(currentPage - 1);
  }

  function handleSelection() {
    const sel = window.getSelection();
    const text = sel?.toString().trim();
    if (!text) {
      lastSelection = null;
      return;
    }
    let rect: DOMRect | undefined;
    let rects: DOMRect[] = [];
    try {
      const range = sel!.getRangeAt(0);
      rect = range.getBoundingClientRect();
      rects = Array.from(range.getClientRects());
    } catch {}
    lastSelection = { text, rects };
    onTextSelect(text, rect);
  }

  async function loadHighlights(page: number) {
    try {
      const rows = await invoke<
        { id: number; page: number; color: string; text: string | null; rects: string }[]
      >("get_highlights", { bookId: book.id, page });
      highlights = rows.map((r) => ({
        id: r.id,
        page: r.page,
        color: r.color,
        text: r.text,
        rects: JSON.parse(r.rects),
      }));
    } catch (e) {
      console.error("loadHighlights:", e);
      highlights = [];
    }
  }

  // Save current selection as highlight; rects normalized to scale=1
  async function addHighlightFromSelection(color: string) {
    if (!lastSelection || lastSelection.rects.length === 0) return;
    const layerRect = highlightLayer.getBoundingClientRect();
    const normRects = lastSelection.rects.map((r) => ({
      x: (r.left - layerRect.left) / scale,
      y: (r.top - layerRect.top) / scale,
      w: r.width / scale,
      h: r.height / scale,
    }));
    try {
      await invoke<unknown>("add_highlight", {
        bookId: book.id,
        page: currentPage,
        text: lastSelection.text,
        color,
        rects: JSON.stringify(normRects),
      });
      window.getSelection()?.removeAllRanges();
      lastSelection = null;
      await loadHighlights(currentPage);
    } catch (e) {
      console.error("add_highlight:", e);
    }
  }

  // Expose to parent via bindable props
  $effect(() => {
    highlighter = addHighlightFromSelection;
    searcher = searchPdf;
    jumpTo = goTo;
  });

  async function searchPdf(query: string): Promise<{ page: number; snippet: string }[]> {
    if (!pdfDoc || !query.trim()) return [];
    const q = query.trim();
    const lowerQ = q.toLowerCase();
    const results: { page: number; snippet: string }[] = [];
    const ctxBefore = 40;
    const ctxAfter = 60;
    const maxResults = 200;

    for (let i = 1; i <= pdfDoc.numPages; i++) {
      const page = await pdfDoc.getPage(i);
      try {
        const content = await page.getTextContent();
        const text = content.items.map((it: any) => it.str).join(" ");
        const lower = text.toLowerCase();
        let idx = 0;
        while ((idx = lower.indexOf(lowerQ, idx)) !== -1) {
          const start = Math.max(0, idx - ctxBefore);
          const end = Math.min(text.length, idx + q.length + ctxAfter);
          let snippet = text.substring(start, end).replace(/\s+/g, " ").trim();
          if (start > 0) snippet = "…" + snippet;
          if (end < text.length) snippet = snippet + "…";
          results.push({ page: i, snippet });
          idx += q.length;
          if (results.length >= maxResults) break;
        }
      } finally {
        page.cleanup();
      }
      if (results.length >= maxResults) break;
    }
    return results;
  }

  onMount(() => {
    loadPDF();
    window.addEventListener("keydown", handleKeyDown);
    document.addEventListener("mouseup", handleSelection);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleKeyDown);
    document.removeEventListener("mouseup", handleSelection);
    pdfDoc?.destroy();
    if (saveProgress) clearTimeout(saveProgress);
  });
</script>

<div class="pdf-viewer" bind:this={container}>
  {#if loadError}
    <div class="load-error">
      <p>Failed to load PDF:</p>
      <pre>{loadError}</pre>
    </div>
  {/if}
  <div class="canvas-wrap">
    <div class="page-wrap">
      <canvas bind:this={canvas}></canvas>
      <div class="highlight-layer" bind:this={highlightLayer}>
        {#each highlights as h (h.id)}
          {#each h.rects as r, i (i)}
            <div
              class="highlight-rect"
              style="background: {h.color}; left: {r.x * scale}px; top: {r.y * scale}px; width: {r.w * scale}px; height: {r.h * scale}px;"
              title={h.text ?? ""}
            ></div>
          {/each}
        {/each}
      </div>
      <div class="textLayer" bind:this={textLayerDiv}></div>
      <div class="annotationLayer" bind:this={annotationLayerDiv}></div>
    </div>
  </div>

  <nav class="controls">
    <button class="nav" onclick={() => goTo(currentPage - 1)} disabled={currentPage <= 1}>←</button>
    <span class="page-info">
      <input
        type="number"
        min="1"
        max={totalPages}
        value={currentPage}
        onchange={(e) => goTo(parseInt((e.target as HTMLInputElement).value))}
      />
      <span>/ {totalPages}</span>
    </span>
    <button class="nav" onclick={() => goTo(currentPage + 1)} disabled={currentPage >= totalPages}>→</button>
    <div class="zoom-controls">
      <button class="zoom" onclick={() => { scale = Math.max(0.25, scale - 0.1); renderPage(currentPage); }}>−</button>
      <span class="zoom-pct">{Math.round(scale * 100)}%</span>
      <button class="zoom" onclick={() => { scale = Math.min(4, scale + 0.1); renderPage(currentPage); }}>+</button>
    </div>
  </nav>
</div>

<style>
  .pdf-viewer {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #181825;
    overflow: hidden;
  }
  .load-error {
    padding: 16px;
    color: #f38ba8;
    font-size: 12px;
  }
  .load-error pre { background: #181825; padding: 8px; border-radius: 4px; font-size: 11px; }
  .canvas-wrap {
    flex: 1;
    overflow: auto;
    padding: 20px;
    text-align: center;
  }
  .page-wrap {
    position: relative;
    display: inline-block;
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.6);
    line-height: 0;
  }
  canvas {
    display: block;
  }
  .highlight-layer {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .highlight-rect {
    position: absolute;
    mix-blend-mode: multiply;
    border-radius: 2px;
    opacity: 0.55;
    pointer-events: auto;
  }
  .textLayer {
    position: absolute;
    inset: 0;
    overflow: hidden;
    opacity: 1;
    line-height: 1;
    text-align: left;
    --scale-factor: 1;
  }
  .textLayer :global(span),
  .textLayer :global(br) {
    color: transparent;
    position: absolute;
    white-space: pre;
    cursor: text;
    transform-origin: 0% 0%;
  }
  .textLayer :global(::selection) {
    background: rgba(137, 180, 250, 0.4);
  }
  .annotationLayer {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .annotationLayer :global(.pdf-link) {
    position: absolute;
    display: block;
    pointer-events: auto;
    cursor: pointer;
    background: transparent;
    transition: background 0.1s;
  }
  .annotationLayer :global(.pdf-link:hover) {
    background: rgba(137, 180, 250, 0.18);
    outline: 1px solid rgba(137, 180, 250, 0.5);
  }
  .textLayer :global(.endOfContent) {
    display: block;
    position: absolute;
    inset: 100% 0 0;
    z-index: -1;
    cursor: default;
    user-select: none;
  }
  .controls {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 10px 16px;
    background: #313244;
    border-top: 1px solid #45475a;
    flex-shrink: 0;
  }
  .controls button {
    border: 1px solid transparent;
    padding: 6px 14px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 600;
    transition: background 0.12s, color 0.12s, border-color 0.12s;
  }
  .controls button.nav {
    background: rgba(137, 180, 250, 0.18);
    color: #89b4fa;
    border-color: rgba(137, 180, 250, 0.35);
    min-width: 38px;
  }
  .controls button.nav:hover:not(:disabled) {
    background: #89b4fa;
    color: #1e1e2e;
  }
  .controls button.zoom {
    background: rgba(203, 166, 247, 0.18);
    color: #cba6f7;
    border-color: rgba(203, 166, 247, 0.35);
    min-width: 32px;
    padding: 6px 10px;
  }
  .controls button.zoom:hover:not(:disabled) {
    background: #cba6f7;
    color: #1e1e2e;
  }
  .controls button:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .page-info {
    display: flex;
    align-items: center;
    gap: 6px;
    color: #cdd6f4;
    font-size: 14px;
  }
  .page-info input {
    width: 56px;
    text-align: center;
    background: #45475a;
    border: 1px solid #585b70;
    color: #cdd6f4;
    border-radius: 5px;
    padding: 4px 6px;
    font-size: 13px;
    font-weight: 600;
  }
  .page-info input:focus {
    outline: none;
    border-color: #89b4fa;
  }
  .zoom-controls {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .zoom-pct {
    color: #cdd6f4;
    font-size: 13px;
    font-weight: 600;
    min-width: 48px;
    text-align: center;
  }
</style>
