<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
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
    outline = $bindable(null),
    navigateToDest = $bindable(null),
  }: {
    book: Book;
    onTextSelect: (text: string, rect?: DOMRect) => void;
    onPageChange?: (page: number, totalPages: number) => void;
    highlighter?: ((color: string) => Promise<void>) | null;
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

  interface Highlight {
    id: number;
    page: number;
    color: string;
    text: string | null;
    rects: { x: number; y: number; w: number; h: number }[];
  }

  type ViewMode = "paged" | "scroll";

  interface PageEntry {
    pageNum: number;
    cssWidth: number;
    cssHeight: number;
    rendered: boolean;
    rendering: boolean;
    canvasEl: HTMLCanvasElement | null;
    textLayerEl: HTMLDivElement | null;
    annotationLayerEl: HTMLDivElement | null;
    highlightLayerEl: HTMLDivElement | null;
    highlights: Highlight[];
  }

  // Paged-mode state (existing)
  let highlights = $state<Highlight[]>([]);
  let highlightLayer: HTMLDivElement;
  let lastSelection: { text: string; rects: DOMRect[]; pageNum: number } | null = null;
  let loading = $state(true);
  let loadProgress = $state(0);
  let removeMenu = $state<{ id: number; x: number; y: number } | null>(null);
  let clickStartPos: { x: number; y: number } | null = null;

  // Scroll-mode state
  const MAX_RENDERED = 20;
  let viewMode = $state<ViewMode>(loadViewMode());
  let pageList = $state<PageEntry[]>([]);
  let scrollContainer: HTMLDivElement | null = $state(null);
  let scrollObserver: IntersectionObserver | null = null;
  let renderOrder: number[] = []; // LRU: oldest at index 0, newest at end

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

  function loadViewMode(): ViewMode {
    try {
      const v = localStorage.getItem("pdf_view_mode");
      if (v === "scroll" || v === "paged") return v;
    } catch {}
    return "paged";
  }

  async function setViewMode(mode: ViewMode) {
    if (viewMode === mode) return;
    viewMode = mode;
    try {
      localStorage.setItem("pdf_view_mode", mode);
    } catch {}
    if (mode === "scroll") {
      await setupScrollMode();
    } else {
      teardownScrollMode();
      await tick();
      await renderPage(currentPage);
    }
  }

  async function loadPDF() {
    loading = true;
    loadProgress = 0;
    try {
      const fileUrl = convertFileSrc(book.file_path);
      const task = pdfjsLib.getDocument(fileUrl);
      task.onProgress = ({ loaded, total }: { loaded: number; total: number }) => {
        if (total > 0) loadProgress = Math.round((loaded / total) * 100);
      };
      pdfDoc = await task.promise;
      totalPages = pdfDoc.numPages;
      if (viewMode === "scroll") {
        await setupScrollMode();
      } else {
        await renderPage(currentPage);
      }
      try {
        const rawOutline = await pdfDoc.getOutline();
        outline = rawOutline ? mapOutline(rawOutline) : [];
      } catch {
        outline = [];
      }
    } catch (e) {
      loadError = String(e);
      console.error("PDFViewer error:", e);
    } finally {
      loading = false;
    }
  }

  function mapOutline(items: any[]): OutlineItem[] {
    return items.map((it) => ({
      title: it.title ?? "",
      dest: it.dest ?? it.url ?? null,
      items: it.items ? mapOutline(it.items) : [],
    }));
  }

  async function navigateToPdfDest(dest: unknown) {
    if (!pdfDoc || dest == null) return;
    try {
      let explicit: any = dest;
      if (typeof dest === "string") {
        explicit = await pdfDoc.getDestination(dest);
      }
      if (!Array.isArray(explicit)) return;
      const pageIdx = await pdfDoc.getPageIndex(explicit[0]);
      await goTo(pageIdx + 1);
    } catch (e) {
      console.warn("navigateToPdfDest failed:", e);
    }
  }

  // Shared per-page render logic. Renders into provided target elements
  // and returns the CSS-size dimensions actually used.
  async function renderPageInto(
    num: number,
    target: {
      canvas: HTMLCanvasElement;
      textLayer: HTMLDivElement;
      annotationLayer: HTMLDivElement;
    },
  ): Promise<{ cssWidth: number; cssHeight: number }> {
    if (!pdfDoc) return { cssWidth: 0, cssHeight: 0 };
    const page = await pdfDoc.getPage(num);
    // Cap DPR at 1.5: in HiDPI displays canvas memory scales DPR². Capping
    // here saves ~44% canvas RAM with negligible visual difference at typical
    // reading zoom.
    const dpr = Math.min(window.devicePixelRatio || 1, 1.5);
    const cssViewport = page.getViewport({ scale });
    const renderViewport = page.getViewport({ scale: scale * dpr });
    const ctx = target.canvas.getContext("2d")!;

    target.canvas.width = Math.floor(renderViewport.width);
    target.canvas.height = Math.floor(renderViewport.height);
    target.canvas.style.width = `${Math.floor(cssViewport.width)}px`;
    target.canvas.style.height = `${Math.floor(cssViewport.height)}px`;

    const cssW = `${Math.floor(cssViewport.width)}px`;
    const cssH = `${Math.floor(cssViewport.height)}px`;
    target.textLayer.style.width = cssW;
    target.textLayer.style.height = cssH;
    target.textLayer.innerHTML = "";
    target.textLayer.style.setProperty("--scale-factor", String(cssViewport.scale));
    target.annotationLayer.style.width = cssW;
    target.annotationLayer.style.height = cssH;
    target.annotationLayer.innerHTML = "";

    await page.render({
      canvasContext: ctx,
      viewport: renderViewport,
      canvas: target.canvas,
    }).promise;

    const textContent = await page.getTextContent();
    const textLayer = new pdfjsLib.TextLayer({
      textContentSource: textContent,
      container: target.textLayer,
      viewport: cssViewport,
    });
    await textLayer.render();

    await renderLinkAnnotations(page, cssViewport, target.annotationLayer);

    if (!coverSaved && num === 1) {
      coverSaved = true;
      const dataUrl = target.canvas.toDataURL("image/png");
      const appDataDir = await getAppDataDir();
      invoke("save_pdf_cover", { bookId: book.id, dataUrl, appDataDir }).catch(() => {});
    }

    return {
      cssWidth: Math.floor(cssViewport.width),
      cssHeight: Math.floor(cssViewport.height),
    };
  }

  async function renderPage(num: number) {
    if (!pdfDoc || rendering) return;
    rendering = true;
    try {
      await renderPageInto(num, {
        canvas,
        textLayer: textLayerDiv,
        annotationLayer: annotationLayerDiv,
      });
      onPageChange?.(num, totalPages);
      debounceSaveProgress(num);
      loadHighlights(num);
    } finally {
      rendering = false;
    }
  }

  async function renderScrollPage(num: number) {
    const entry = pageList[num - 1];
    if (!entry || entry.rendered || entry.rendering) return;
    if (!entry.canvasEl || !entry.textLayerEl || !entry.annotationLayerEl) return;
    entry.rendering = true;
    try {
      const dims = await renderPageInto(num, {
        canvas: entry.canvasEl,
        textLayer: entry.textLayerEl,
        annotationLayer: entry.annotationLayerEl,
      });
      entry.cssWidth = dims.cssWidth;
      entry.cssHeight = dims.cssHeight;
      entry.rendered = true;
      trackRender(num);
      await loadHighlightsForPage(num);
    } catch (e) {
      console.error(`renderScrollPage(${num}):`, e);
    } finally {
      entry.rendering = false;
    }
  }

  function trackRender(num: number) {
    const idx = renderOrder.indexOf(num);
    if (idx !== -1) renderOrder.splice(idx, 1);
    renderOrder.push(num);
    while (renderOrder.length > MAX_RENDERED) {
      const oldest = renderOrder.shift()!;
      evictPage(oldest);
    }
  }

  function evictPage(num: number) {
    const entry = pageList[num - 1];
    if (!entry || !entry.rendered) return;
    if (entry.canvasEl) {
      // width=0 frees the pixel buffer; CSS style.width tetap, layout aman
      entry.canvasEl.width = 0;
      entry.canvasEl.height = 0;
    }
    if (entry.textLayerEl) entry.textLayerEl.innerHTML = "";
    if (entry.annotationLayerEl) entry.annotationLayerEl.innerHTML = "";
    entry.rendered = false;
  }

  async function renderLinkAnnotations(
    page: any,
    viewport: any,
    target: HTMLDivElement,
  ) {
    const annotations = await page.getAnnotations();
    for (const ann of annotations) {
      if (ann.subtype !== "Link") continue;

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
      target.appendChild(el);
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

  async function setupScrollMode() {
    if (!pdfDoc) return;
    const firstPage = await pdfDoc.getPage(1);
    const v = firstPage.getViewport({ scale });
    const estW = Math.floor(v.width);
    const estH = Math.floor(v.height);

    pageList = Array.from({ length: totalPages }, (_, i) => ({
      pageNum: i + 1,
      cssWidth: estW,
      cssHeight: estH,
      rendered: false,
      rendering: false,
      canvasEl: null,
      textLayerEl: null,
      annotationLayerEl: null,
      highlightLayerEl: null,
      highlights: [],
    }));

    await tick();
    requestAnimationFrame(() => {
      setupScrollObserver();
      scrollToPage(currentPage, false);
    });
  }

  function teardownScrollMode() {
    if (scrollObserver) {
      scrollObserver.disconnect();
      scrollObserver = null;
    }
    pageList = [];
    renderOrder = [];
  }

  function setupScrollObserver() {
    if (!scrollContainer) return;
    if (scrollObserver) scrollObserver.disconnect();
    scrollObserver = new IntersectionObserver(
      (entries) => {
        let bestNum = 0;
        let bestRatio = 0;
        for (const e of entries) {
          const num = parseInt((e.target as HTMLElement).dataset.page ?? "0");
          if (!num) continue;
          const entry = pageList[num - 1];
          if (!entry) continue;
          if (e.isIntersecting) {
            if (!entry.rendered && !entry.rendering) {
              renderScrollPage(num);
            } else if (entry.rendered) {
              trackRender(num);
            }
          }
          if (e.intersectionRatio > bestRatio) {
            bestRatio = e.intersectionRatio;
            bestNum = num;
          }
        }
        if (bestNum > 0 && bestRatio > 0.5 && currentPage !== bestNum) {
          currentPage = bestNum;
          onPageChange?.(bestNum, totalPages);
          debounceSaveProgress(bestNum);
        }
      },
      { root: scrollContainer, rootMargin: "800px 0px", threshold: [0, 0.25, 0.5, 0.75, 1] },
    );
    const wraps = scrollContainer.querySelectorAll<HTMLElement>("[data-page]");
    wraps.forEach((el) => scrollObserver!.observe(el));
  }

  function scrollToPage(num: number, smooth: boolean) {
    if (!scrollContainer) return;
    const el = scrollContainer.querySelector<HTMLElement>(`[data-page="${num}"]`);
    if (el) {
      el.scrollIntoView({ behavior: smooth ? "smooth" : "auto", block: "start" });
    }
  }

  async function goTo(page: number) {
    if (!pdfDoc) return;
    const clamped = Math.max(1, Math.min(totalPages, page));
    currentPage = clamped;
    if (viewMode === "paged") {
      await renderPage(currentPage);
    } else {
      scrollToPage(clamped, false);
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape" && removeMenu) {
      removeMenu = null;
      return;
    }
    if (e.ctrlKey || e.metaKey) {
      if (e.key === "=" || e.key === "+") {
        e.preventDefault();
        changeZoom(0.1);
        return;
      }
      if (e.key === "-" || e.key === "_") {
        e.preventDefault();
        changeZoom(-0.1);
        return;
      }
      if (e.key === "0") {
        e.preventDefault();
        const delta = 1 - scale;
        if (delta !== 0) changeZoom(delta);
        return;
      }
    }
    if (e.key === "ArrowRight" || e.key === "ArrowDown") goTo(currentPage + 1);
    if (e.key === "ArrowLeft" || e.key === "ArrowUp") goTo(currentPage - 1);
  }

  function handlePageMouseDown(e: MouseEvent) {
    clickStartPos = { x: e.clientX, y: e.clientY };
  }

  function handlePageMouseUp(e: MouseEvent) {
    if (!clickStartPos) return;
    const dx = e.clientX - clickStartPos.x;
    const dy = e.clientY - clickStartPos.y;
    const moved = Math.hypot(dx, dy);
    clickStartPos = null;
    if (moved > 5) return;

    if (window.getSelection()?.toString().trim()) return;

    const els = document.elementsFromPoint(e.clientX, e.clientY);
    for (const el of els) {
      if (
        el instanceof HTMLElement &&
        el.classList.contains("highlight-rect")
      ) {
        const id = parseInt(el.dataset.highlightId ?? "0");
        if (id) {
          removeMenu = { id, x: e.clientX, y: e.clientY };
        }
        return;
      }
    }
    removeMenu = null;
  }

  async function deleteHighlight(id: number) {
    try {
      await invoke("delete_highlight", { id });
      removeMenu = null;
      if (viewMode === "paged") {
        await loadHighlights(currentPage);
      } else {
        // Find which page this highlight was on and refresh it
        for (const entry of pageList) {
          if (entry.highlights.some((h) => h.id === id)) {
            await loadHighlightsForPage(entry.pageNum);
            break;
          }
        }
      }
    } catch (e) {
      console.error("delete_highlight:", e);
    }
  }

  function findSelectionPageNum(range: Range): number {
    if (viewMode === "paged") return currentPage;
    const startEl =
      range.startContainer.nodeType === Node.TEXT_NODE
        ? range.startContainer.parentElement
        : (range.startContainer as Element);
    const wrap = startEl?.closest("[data-page]") as HTMLElement | null;
    if (!wrap) return currentPage;
    return parseInt(wrap.dataset.page ?? String(currentPage));
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
    let pageNum = currentPage;
    try {
      const range = sel!.getRangeAt(0);
      rect = range.getBoundingClientRect();
      rects = Array.from(range.getClientRects());
      pageNum = findSelectionPageNum(range);
    } catch {}
    lastSelection = { text, rects, pageNum };
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

  async function loadHighlightsForPage(num: number) {
    const entry = pageList[num - 1];
    if (!entry) return;
    try {
      const rows = await invoke<
        { id: number; page: number; color: string; text: string | null; rects: string }[]
      >("get_highlights", { bookId: book.id, page: num });
      entry.highlights = rows.map((r) => ({
        id: r.id,
        page: r.page,
        color: r.color,
        text: r.text,
        rects: JSON.parse(r.rects),
      }));
    } catch (e) {
      console.error("loadHighlightsForPage:", e);
      entry.highlights = [];
    }
  }

  async function addHighlightFromSelection(color: string) {
    if (!lastSelection || lastSelection.rects.length === 0) return;
    const pageNum = lastSelection.pageNum;
    const layerEl =
      viewMode === "paged"
        ? highlightLayer
        : pageList[pageNum - 1]?.highlightLayerEl;
    if (!layerEl) return;

    const layerRect = layerEl.getBoundingClientRect();
    const normRects = lastSelection.rects.map((r) => ({
      x: (r.left - layerRect.left) / scale,
      y: (r.top - layerRect.top) / scale,
      w: r.width / scale,
      h: r.height / scale,
    }));
    try {
      await invoke<unknown>("add_highlight", {
        bookId: book.id,
        page: pageNum,
        text: lastSelection.text,
        color,
        rects: JSON.stringify(normRects),
      });
      window.getSelection()?.removeAllRanges();
      lastSelection = null;
      if (viewMode === "paged") {
        await loadHighlights(pageNum);
      } else {
        await loadHighlightsForPage(pageNum);
      }
    } catch (e) {
      console.error("add_highlight:", e);
    }
  }

  $effect(() => {
    highlighter = addHighlightFromSelection;
    searcher = searchPdf;
    jumpTo = goTo;
    navigateToDest = navigateToPdfDest;
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

  async function changeZoom(delta: number) {
    const newScale = Math.max(0.25, Math.min(4, scale + delta));
    if (newScale === scale) return;
    scale = newScale;
    if (viewMode === "paged") {
      await renderPage(currentPage);
    } else {
      // Reset rendered state for all pages; observer will re-trigger
      // visible ones. Update placeholder dims based on new scale.
      const ratio = newScale / (newScale - delta);
      for (const entry of pageList) {
        entry.cssWidth = Math.floor(entry.cssWidth * ratio);
        entry.cssHeight = Math.floor(entry.cssHeight * ratio);
        entry.rendered = false;
      }
      renderOrder = [];
      // Force re-render of visible pages
      if (scrollObserver && scrollContainer) {
        scrollObserver.disconnect();
        await tick();
        setupScrollObserver();
      }
    }
  }

  onMount(() => {
    loadPDF();
    window.addEventListener("keydown", handleKeyDown);
    document.addEventListener("mouseup", handleSelection);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleKeyDown);
    document.removeEventListener("mouseup", handleSelection);
    scrollObserver?.disconnect();
    pdfDoc?.destroy();
    if (saveProgress) clearTimeout(saveProgress);
  });
</script>

<div class="pdf-viewer" bind:this={container}>
  {#if loading && !loadError}
    <div class="loading-overlay">
      <div class="spinner"></div>
      <div class="loading-text">
        Loading PDF{loadProgress > 0 ? `… ${loadProgress}%` : "…"}
      </div>
      {#if loadProgress > 0}
        <div class="progress-track">
          <div class="progress-fill" style:width="{loadProgress}%"></div>
        </div>
      {/if}
    </div>
  {/if}
  {#if loadError}
    <div class="load-error">
      <p>Failed to load PDF:</p>
      <pre>{loadError}</pre>
    </div>
  {/if}

  {#if viewMode === "paged"}
    <div class="canvas-wrap paged">
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="page-wrap"
        data-page={currentPage}
        onmousedown={handlePageMouseDown}
        onmouseup={handlePageMouseUp}
      >
        <canvas bind:this={canvas}></canvas>
        <div class="highlight-layer" bind:this={highlightLayer}>
          {#each highlights as h (h.id)}
            {#each h.rects as r, i (i)}
              <div
                class="highlight-rect"
                data-highlight-id={h.id}
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
  {:else}
    <div class="canvas-wrap scroll" bind:this={scrollContainer}>
      <div class="scroll-list">
        {#each pageList as p, i (p.pageNum)}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="page-wrap"
            data-page={p.pageNum}
            style:width="{p.cssWidth}px"
            style:height="{p.cssHeight}px"
            onmousedown={handlePageMouseDown}
            onmouseup={handlePageMouseUp}
          >
            <canvas bind:this={pageList[i].canvasEl}></canvas>
            <div class="highlight-layer" bind:this={pageList[i].highlightLayerEl}>
              {#each p.highlights as h (h.id)}
                {#each h.rects as r, ri (ri)}
                  <div
                    class="highlight-rect"
                    data-highlight-id={h.id}
                    style="background: {h.color}; left: {r.x * scale}px; top: {r.y * scale}px; width: {r.w * scale}px; height: {r.h * scale}px;"
                    title={h.text ?? ""}
                  ></div>
                {/each}
              {/each}
            </div>
            <div class="textLayer" bind:this={pageList[i].textLayerEl}></div>
            <div class="annotationLayer" bind:this={pageList[i].annotationLayerEl}></div>
            {#if !p.rendered}
              <div class="page-placeholder">Page {p.pageNum}</div>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/if}

  {#if removeMenu}
    <div
      class="remove-menu"
      style="left: {removeMenu.x}px; top: {removeMenu.y}px;"
    >
      <button onclick={() => deleteHighlight(removeMenu!.id)}>
        🗑 Remove highlight
      </button>
    </div>
  {/if}

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
      <button class="zoom" onclick={() => changeZoom(-0.1)}>−</button>
      <span class="zoom-pct">{Math.round(scale * 100)}%</span>
      <button class="zoom" onclick={() => changeZoom(0.1)}>+</button>
    </div>
    <button
      class="mode-toggle"
      onclick={() => setViewMode(viewMode === "paged" ? "scroll" : "paged")}
      title={viewMode === "paged" ? "Switch to continuous scroll" : "Switch to paged view"}
    >
      {viewMode === "paged" ? "☰ Scroll" : "▤ Paged"}
    </button>
  </nav>
</div>

<style>
  .pdf-viewer {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #181825;
    overflow: hidden;
    position: relative;
  }
  .load-error {
    padding: 16px;
    color: #f38ba8;
    font-size: 12px;
  }
  .load-error pre { background: #181825; padding: 8px; border-radius: 4px; font-size: 11px; }
  .loading-overlay {
    position: absolute;
    inset: 0;
    z-index: 50;
    background: rgba(24, 24, 37, 0.92);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 14px;
    backdrop-filter: blur(2px);
  }
  .spinner {
    width: 40px;
    height: 40px;
    border: 3px solid #313244;
    border-top-color: #89b4fa;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
  .loading-text {
    color: #cdd6f4;
    font-size: 13px;
  }
  .progress-track {
    width: 200px;
    height: 4px;
    background: #313244;
    border-radius: 2px;
    overflow: hidden;
  }
  .progress-fill {
    height: 100%;
    background: #89b4fa;
    transition: width 0.15s ease;
  }
  .canvas-wrap {
    flex: 1;
    overflow: auto;
    padding: 20px;
  }
  .canvas-wrap.paged {
    text-align: center;
  }
  .canvas-wrap.scroll {
    padding: 16px 0;
  }
  .scroll-list {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
  }
  .page-wrap {
    position: relative;
    display: inline-block;
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.6);
    line-height: 0;
    background: #fff;
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
    cursor: pointer;
  }
  .page-placeholder {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #6c7086;
    font-size: 14px;
    pointer-events: none;
    line-height: normal;
  }
  .remove-menu {
    position: fixed;
    transform: translate(-50%, calc(-100% - 4px));
    background: #181825;
    border: 1px solid #45475a;
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
    padding: 4px;
    z-index: 1000;
  }
  .remove-menu button {
    background: transparent;
    border: none;
    color: #f38ba8;
    padding: 6px 12px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
    transition: background 0.1s;
  }
  .remove-menu button:hover {
    background: rgba(243, 139, 168, 0.15);
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
  .controls button.mode-toggle {
    background: rgba(166, 227, 161, 0.18);
    color: #a6e3a1;
    border-color: rgba(166, 227, 161, 0.35);
    font-size: 13px;
    padding: 6px 12px;
  }
  .controls button.mode-toggle:hover {
    background: #a6e3a1;
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
