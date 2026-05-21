<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { Book } from "../stores/books";

  let { book, currentPage, appendRequest }: {
    book: Book;
    currentPage: number;
    appendRequest?: { text: string; ts: number } | null;
  } = $props();

  let lastHandledTs = 0;

  $effect(() => {
    if (appendRequest && appendRequest.ts > lastHandledTs) {
      lastHandledTs = appendRequest.ts;
      const quote = `"${appendRequest.text.replace(/\n/g, " ").trim()}"`;
      text = text ? `${text}\n\n${quote}` : quote;
      scheduleSave();
    }
  });

  // Points stored as normalized 0-1 coordinates so drawings scale with canvas.
  interface Point { x: number; y: number; pressure: number; }
  interface Stroke { points: Point[]; color: string; width: number; }

  let text = $state("");
  let strokes = $state<Stroke[]>([]);
  let mode = $state<"text" | "draw" | "erase">("text");
  let color = $state("#f9e2af");
  let brushSize = $state(2);

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D | null = null;
  let drawing = false;
  let activeStroke: Stroke | null = null;
  let loadedKey = "";
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let pending: { page: number; text: string; strokes: Stroke[] } | null = null;
  let cssWidth = 0;
  let cssHeight = 0;
  let resizeObs: ResizeObserver | null = null;

  $effect(() => {
    const key = `${book.id}:${currentPage}`;
    if (key !== loadedKey) {
      loadedKey = key;
      loadNote();
    }
  });

  async function loadNote() {
    // Flush any pending save for the OLD page first
    await commitSave();

    const note = await invoke<{
      text: string | null;
      strokes: string | null;
    } | null>("get_note", { bookId: book.id, page: currentPage });

    text = note?.text ?? "";
    try {
      strokes = note?.strokes ? JSON.parse(note.strokes) : [];
    } catch {
      strokes = [];
    }
    await tick();
    sizeCanvas();
    redraw();
  }

  function sizeCanvas() {
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) return;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.max(1, Math.round(rect.width * dpr));
    canvas.height = Math.max(1, Math.round(rect.height * dpr));
    cssWidth = rect.width;
    cssHeight = rect.height;
    ctx = canvas.getContext("2d");
    if (ctx) {
      // setTransform replaces the matrix (avoids cumulative scaling on re-init)
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.lineCap = "round";
      ctx.lineJoin = "round";
    }
  }

  function redraw() {
    if (!ctx || !canvas || cssWidth === 0) return;
    ctx.clearRect(0, 0, cssWidth, cssHeight);
    for (const s of strokes) drawStroke(s);
    if (activeStroke) drawStroke(activeStroke);
  }

  function drawStroke(s: Stroke) {
    if (!ctx || s.points.length === 0 || cssWidth === 0) return;
    ctx.strokeStyle = s.color;
    ctx.lineWidth = s.width;
    ctx.beginPath();
    ctx.moveTo(s.points[0].x * cssWidth, s.points[0].y * cssHeight);
    for (let i = 1; i < s.points.length; i++) {
      ctx.lineTo(s.points[i].x * cssWidth, s.points[i].y * cssHeight);
    }
    ctx.stroke();
  }

  function pointFromEvent(e: PointerEvent): Point {
    const rect = canvas.getBoundingClientRect();
    return {
      x: (e.clientX - rect.left) / rect.width,
      y: (e.clientY - rect.top) / rect.height,
      pressure: e.pressure > 0 ? e.pressure : 0.5,
    };
  }

  function onPointerDown(e: PointerEvent) {
    if (mode === "text") return;
    e.preventDefault();
    canvas.setPointerCapture(e.pointerId);
    drawing = true;

    if (mode === "erase") {
      // Erase: remove strokes whose any point is close to cursor
      const p = pointFromEvent(e);
      // 14px in CSS pixels, expressed in normalized space
      const tx = 14 / (cssWidth || 1);
      const ty = 14 / (cssHeight || 1);
      strokes = strokes.filter(
        (s) =>
          !s.points.some((sp) => {
            const dx = (sp.x - p.x) / tx;
            const dy = (sp.y - p.y) / ty;
            return dx * dx + dy * dy < 1;
          })
      );
      redraw();
      scheduleSave();
      return;
    }

    activeStroke = {
      points: [pointFromEvent(e)],
      color,
      width: brushSize,
    };
    redraw();
  }

  function onPointerMove(e: PointerEvent) {
    if (!drawing) return;
    if (mode === "erase") {
      const p = pointFromEvent(e);
      const threshold = 14;
      const before = strokes.length;
      strokes = strokes.filter(
        (s) =>
          !s.points.some(
            (sp) => Math.hypot(sp.x - p.x, sp.y - p.y) < threshold
          )
      );
      if (strokes.length !== before) {
        redraw();
        scheduleSave();
      }
      return;
    }
    if (!activeStroke) return;
    activeStroke.points.push(pointFromEvent(e));
    redraw();
  }

  function onPointerUp() {
    if (!drawing) return;
    drawing = false;
    if (activeStroke && activeStroke.points.length > 0) {
      strokes = [...strokes, activeStroke];
      scheduleSave();
    }
    activeStroke = null;
  }

  function clearAll() {
    strokes = [];
    redraw();
    scheduleSave();
  }

  function onTextInput() {
    scheduleSave();
  }

  function scheduleSave() {
    // Capture current state + page at schedule time so async save uses the
    // right page even if user has paged forward by the time it fires.
    pending = {
      page: currentPage,
      text,
      strokes: [...strokes],
    };
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(commitSave, 500);
  }

  async function commitSave() {
    if (saveTimer) {
      clearTimeout(saveTimer);
      saveTimer = null;
    }
    if (!pending) return;
    const p = pending;
    pending = null;
    try {
      await invoke("save_note", {
        bookId: book.id,
        page: p.page,
        text: p.text.trim() || null,
        strokes: p.strokes.length > 0 ? JSON.stringify(p.strokes) : null,
      });
    } catch (e) {
      console.error("save_note failed:", e);
    }
  }

  function handleResize() {
    sizeCanvas();
    redraw();
  }

  onMount(() => {
    sizeCanvas();
    redraw();
    window.addEventListener("resize", handleResize);
    resizeObs = new ResizeObserver(() => {
      sizeCanvas();
      redraw();
    });
    resizeObs.observe(canvas);
  });

  onDestroy(() => {
    window.removeEventListener("resize", handleResize);
    resizeObs?.disconnect();
    commitSave();
  });
</script>

<div class="notes-panel">
  <div class="header">
    <span class="title">Notes for page {currentPage}</span>
  </div>

  <textarea
    class="note-text"
    placeholder="Write a note for this page..."
    bind:value={text}
    oninput={onTextInput}
  ></textarea>

  <div class="toolbar">
    <button class:active={mode === "text"} onclick={() => (mode = "text")} title="Text mode">
      Aa
    </button>
    <button class:active={mode === "draw"} onclick={() => (mode = "draw")} title="Draw mode">
      ✎
    </button>
    <button class:active={mode === "erase"} onclick={() => (mode = "erase")} title="Erase">
      ⌫
    </button>
    <input type="color" bind:value={color} disabled={mode !== "draw"} title="Color" />
    <input type="range" min="1" max="12" bind:value={brushSize} disabled={mode !== "draw"} title="Brush size" />
    <button class="clear" onclick={clearAll} title="Clear all">×</button>
  </div>

  <div class="canvas-wrap">
    <canvas
      bind:this={canvas}
      class:draw-cursor={mode === "draw"}
      class:erase-cursor={mode === "erase"}
      onpointerdown={onPointerDown}
      onpointermove={onPointerMove}
      onpointerup={onPointerUp}
      onpointercancel={onPointerUp}
    ></canvas>
  </div>
</div>

<style>
  .notes-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #1e1e2e;
  }
  .header {
    padding: 10px 12px;
    border-bottom: 1px solid #313244;
    flex-shrink: 0;
  }
  .title {
    color: #cdd6f4;
    font-size: 13px;
    font-weight: 600;
  }
  .note-text {
    flex: 0 0 35%;
    background: #181825;
    border: none;
    border-bottom: 1px solid #313244;
    color: #cdd6f4;
    padding: 10px 12px;
    font-size: 13px;
    font-family: inherit;
    resize: none;
    outline: none;
    line-height: 1.5;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 10px;
    background: #181825;
    border-bottom: 1px solid #313244;
    flex-shrink: 0;
  }
  .toolbar button {
    background: #313244;
    color: #cdd6f4;
    border: none;
    width: 30px;
    height: 30px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.1s;
  }
  .toolbar button:hover {
    background: #45475a;
  }
  .toolbar button.active {
    background: #89b4fa;
    color: #1e1e2e;
  }
  .toolbar input[type="color"] {
    width: 28px;
    height: 28px;
    border: 1px solid #45475a;
    border-radius: 4px;
    background: transparent;
    cursor: pointer;
    padding: 0;
  }
  .toolbar input[type="color"]:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .toolbar input[type="range"] {
    flex: 1;
    min-width: 0;
  }
  .toolbar input[type="range"]:disabled {
    opacity: 0.4;
  }
  .toolbar .clear {
    margin-left: auto;
    color: #f38ba8;
  }
  .canvas-wrap {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    background: #11111b;
    position: relative;
  }
  canvas {
    width: 100%;
    height: 100%;
    display: block;
    touch-action: none;
  }
  canvas.draw-cursor {
    cursor: crosshair;
  }
  canvas.erase-cursor {
    cursor: not-allowed;
  }
</style>
