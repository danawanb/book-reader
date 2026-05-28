<script lang="ts">
  import { onMount, onDestroy } from "svelte";

  let { x, y, onAsk, onNote, onCopy, onClose, onHighlight, onDefine }: {
    x: number;
    y: number;
    onAsk: () => void;
    onNote: () => void;
    onCopy: () => void;
    onClose: () => void;
    onHighlight?: (color: string) => void;
    onDefine?: () => void;
  } = $props();

  const HIGHLIGHT_COLORS = ["#f9e2af", "#a6e3a1", "#89b4fa", "#f38ba8"];
  let showColors = $state(false);

  let menuEl: HTMLDivElement;

  function handleClickOutside(e: MouseEvent) {
    if (menuEl && !menuEl.contains(e.target as Node)) onClose();
  }

  function handleEsc(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }

  onMount(() => {
    // delay listener so the triggering mouseup doesn't immediately close
    setTimeout(() => {
      document.addEventListener("mousedown", handleClickOutside);
      document.addEventListener("keydown", handleEsc);
    }, 0);
  });

  onDestroy(() => {
    document.removeEventListener("mousedown", handleClickOutside);
    document.removeEventListener("keydown", handleEsc);
  });
</script>

<div
  bind:this={menuEl}
  class="menu"
  style="left: {x}px; top: {y}px;"
>
  <button onclick={onAsk} title="Ask AI">
    <span class="icon">💬</span><span>Ask AI</span>
  </button>
  {#if onDefine}
    <button onclick={onDefine} title="Define word">
      <span class="icon">📖</span><span>Define</span>
    </button>
  {/if}
  {#if onHighlight}
    <button onclick={() => (showColors = !showColors)} title="Highlight" class="hl-toggle">
      <span class="icon">🖍</span><span>Highlight</span>
    </button>
    {#if showColors}
      <div class="color-row">
        {#each HIGHLIGHT_COLORS as c (c)}
          <button
            class="color-swatch"
            style="background: {c};"
            onclick={() => onHighlight!(c)}
            title="Highlight {c}"
            aria-label="Highlight color"
          ></button>
        {/each}
      </div>
    {/if}
  {/if}
  <button onclick={onNote} title="Add to this page's notes">
    <span class="icon">📝</span><span>Note</span>
  </button>
  <button onclick={onCopy} title="Copy text">
    <span class="icon">⎘</span><span>Copy</span>
  </button>
</div>

<style>
  .menu {
    position: fixed;
    transform: translate(-50%, calc(-100% - 8px));
    background: #181825;
    border: 1px solid #45475a;
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
    display: flex;
    padding: 4px;
    gap: 2px;
    z-index: 1000;
  }
  .menu button {
    background: transparent;
    color: #cdd6f4;
    border: none;
    padding: 6px 10px;
    border-radius: 5px;
    cursor: pointer;
    font-size: 12px;
    display: flex;
    align-items: center;
    gap: 5px;
    white-space: nowrap;
    transition: background 0.1s;
  }
  .menu button:hover {
    background: #313244;
  }
  .icon {
    font-size: 13px;
  }
  .color-row {
    display: flex;
    gap: 4px;
    padding: 0 6px;
    align-items: center;
  }
  .color-swatch {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 1px solid rgba(255, 255, 255, 0.2);
    cursor: pointer;
    padding: 0;
  }
  .color-swatch:hover {
    transform: scale(1.15);
  }
</style>
