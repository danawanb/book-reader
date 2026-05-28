<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { lookupWord, type DictionaryEntry } from "../api/dictionary";

  let { word, x, y, onClose }: {
    word: string;
    x: number;
    y: number;
    onClose: () => void;
  } = $props();

  type State =
    | { kind: "loading" }
    | { kind: "result"; entries: DictionaryEntry[] }
    | { kind: "notFound" }
    | { kind: "error"; message: string };

  let state = $state<State>({ kind: "loading" });
  let popupEl: HTMLDivElement;

  async function fetchDefinition() {
    state = { kind: "loading" };
    try {
      const entries = await lookupWord(word);
      if (entries === null || entries.length === 0) {
        state = { kind: "notFound" };
      } else {
        state = { kind: "result", entries };
      }
    } catch (e) {
      state = { kind: "error", message: String(e) };
    }
  }

  function playAudio(url: string) {
    new Audio(url).play().catch(() => {});
  }

  function firstAudio(entries: DictionaryEntry[]): string | null {
    for (const e of entries) {
      for (const p of e.phonetics) {
        if (p.audio) return p.audio;
      }
    }
    return null;
  }

  function firstPhoneticText(entries: DictionaryEntry[]): string | null {
    for (const e of entries) {
      if (e.phonetic) return e.phonetic;
      for (const p of e.phonetics) {
        if (p.text) return p.text;
      }
    }
    return null;
  }

  function handleClickOutside(e: MouseEvent) {
    if (popupEl && !popupEl.contains(e.target as Node)) onClose();
  }

  function handleEsc(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }

  onMount(() => {
    fetchDefinition();
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

<div bind:this={popupEl} class="popup" style="left: {x}px; top: {y}px;">
  <div class="header">
    <div class="word-line">
      <span class="word">{word}</span>
      {#if state.kind === "result"}
        {@const phon = firstPhoneticText(state.entries)}
        {@const audio = firstAudio(state.entries)}
        {#if phon}<span class="phonetic">{phon}</span>{/if}
        {#if audio}
          <button class="audio-btn" onclick={() => playAudio(audio)} title="Pronounce">▶</button>
        {/if}
      {/if}
    </div>
    <button class="close-btn" onclick={onClose} title="Close">×</button>
  </div>

  <div class="body">
    {#if state.kind === "loading"}
      <div class="status">Looking up…</div>
    {:else if state.kind === "notFound"}
      <div class="status">No definition found for <em>"{word}"</em>.</div>
    {:else if state.kind === "error"}
      <div class="status error">
        Failed to look up word.
        <button class="retry-btn" onclick={fetchDefinition}>Retry</button>
      </div>
    {:else}
      {#each state.entries[0].meanings.slice(0, 2) as meaning}
        <div class="meaning">
          <div class="pos">{meaning.partOfSpeech}</div>
          {#each meaning.definitions.slice(0, 3) as def, i}
            <div class="def">
              <span class="num">{i + 1}.</span>
              <span>
                {def.definition}
                {#if def.example}
                  <div class="example">"{def.example}"</div>
                {/if}
              </span>
            </div>
          {/each}
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .popup {
    position: fixed;
    transform: translate(-50%, calc(-100% - 8px));
    background: #181825;
    border: 1px solid #45475a;
    border-radius: 10px;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.6);
    color: #cdd6f4;
    width: 360px;
    max-height: 420px;
    overflow-y: auto;
    z-index: 1100;
    font-size: 13px;
  }
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-bottom: 1px solid #313244;
    position: sticky;
    top: 0;
    background: #181825;
  }
  .word-line {
    display: flex;
    align-items: baseline;
    gap: 8px;
    flex-wrap: wrap;
  }
  .word {
    font-size: 16px;
    font-weight: 600;
    color: #89b4fa;
  }
  .phonetic {
    font-size: 12px;
    color: #6c7086;
    font-style: italic;
  }
  .audio-btn {
    background: #313244;
    color: #cdd6f4;
    border: none;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    font-size: 9px;
    cursor: pointer;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .audio-btn:hover {
    background: #45475a;
  }
  .close-btn {
    background: transparent;
    border: none;
    color: #6c7086;
    font-size: 20px;
    line-height: 1;
    width: 24px;
    height: 24px;
    border-radius: 4px;
    cursor: pointer;
    padding: 0;
  }
  .close-btn:hover {
    background: #313244;
    color: #cdd6f4;
  }
  .body {
    padding: 10px 14px 14px;
  }
  .status {
    color: #6c7086;
    padding: 8px 0;
  }
  .status.error {
    color: #f38ba8;
  }
  .retry-btn {
    background: #313244;
    color: #cdd6f4;
    border: none;
    padding: 4px 10px;
    border-radius: 5px;
    margin-left: 8px;
    font-size: 12px;
    cursor: pointer;
    font-family: inherit;
  }
  .retry-btn:hover {
    background: #45475a;
  }
  .meaning {
    margin-bottom: 10px;
  }
  .meaning:last-child {
    margin-bottom: 0;
  }
  .pos {
    font-style: italic;
    color: #f9e2af;
    font-size: 12px;
    margin-bottom: 4px;
  }
  .def {
    display: flex;
    gap: 6px;
    padding: 3px 0;
    line-height: 1.45;
  }
  .num {
    color: #6c7086;
    flex-shrink: 0;
  }
  .example {
    color: #94e2d5;
    font-style: italic;
    font-size: 12px;
    margin-top: 2px;
  }
</style>
