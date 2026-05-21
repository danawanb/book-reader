<script lang="ts">
  let {
    searcher,
    onJump,
    initialQuery = "",
  }: {
    searcher: ((query: string) => Promise<{ page: number; snippet: string }[]>) | null;
    onJump: (page: number) => void;
    initialQuery?: string;
  } = $props();

  let query = $state(initialQuery);
  let results = $state<{ page: number; snippet: string }[]>([]);
  let loading = $state(false);
  let error = $state("");
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let inputEl: HTMLInputElement | undefined;

  $effect(() => {
    if (inputEl) inputEl.focus();
  });

  function onInput() {
    error = "";
    if (debounceTimer) clearTimeout(debounceTimer);
    if (!query.trim()) {
      results = [];
      loading = false;
      return;
    }
    debounceTimer = setTimeout(runSearch, 350);
  }

  async function runSearch() {
    if (!searcher) {
      error = "Search not available yet";
      return;
    }
    loading = true;
    try {
      results = await searcher(query);
    } catch (e) {
      error = String(e);
      results = [];
    } finally {
      loading = false;
    }
  }

  function highlight(snippet: string, q: string): string {
    if (!q) return snippet;
    const escaped = q.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    return snippet.replace(
      new RegExp(`(${escaped})`, "gi"),
      `<mark>$1</mark>`
    );
  }
</script>

<div class="search-panel">
  <div class="search-row">
    <input
      bind:this={inputEl}
      type="search"
      placeholder="Search in this book..."
      bind:value={query}
      oninput={onInput}
      onkeydown={(e) => {
        if (e.key === "Enter") {
          if (debounceTimer) clearTimeout(debounceTimer);
          runSearch();
        }
      }}
    />
  </div>

  <div class="result-summary">
    {#if loading}
      Searching…
    {:else if error}
      <span class="err">{error}</span>
    {:else if query && results.length === 0}
      No matches
    {:else if results.length > 0}
      {results.length}{results.length >= 200 ? "+" : ""} matches
    {/if}
  </div>

  <div class="results">
    {#each results as r, i (i)}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="result" onclick={() => onJump(r.page)}>
        <div class="page-tag">Page {r.page}</div>
        <div class="snippet">{@html highlight(r.snippet, query)}</div>
      </div>
    {/each}
  </div>
</div>

<style>
  .search-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #1e1e2e;
  }
  .search-row {
    padding: 10px 12px;
    border-bottom: 1px solid #313244;
    flex-shrink: 0;
  }
  .search-row input {
    width: 100%;
    background: #313244;
    border: 1px solid #45475a;
    color: #cdd6f4;
    border-radius: 6px;
    padding: 8px 12px;
    font-size: 13px;
    outline: none;
  }
  .search-row input:focus {
    border-color: #89b4fa;
  }
  .result-summary {
    padding: 6px 12px;
    color: #6c7086;
    font-size: 12px;
    border-bottom: 1px solid #313244;
    flex-shrink: 0;
  }
  .result-summary .err {
    color: #f38ba8;
  }
  .results {
    flex: 1;
    overflow-y: auto;
    padding: 6px;
  }
  .result {
    padding: 10px;
    border-radius: 6px;
    cursor: pointer;
    margin-bottom: 4px;
    transition: background 0.1s;
  }
  .result:hover {
    background: #313244;
  }
  .page-tag {
    color: #89b4fa;
    font-size: 11px;
    font-weight: 600;
    margin-bottom: 4px;
  }
  .snippet {
    color: #cdd6f4;
    font-size: 13px;
    line-height: 1.5;
    word-break: break-word;
  }
  .snippet :global(mark) {
    background: rgba(249, 226, 175, 0.4);
    color: #f9e2af;
    padding: 0 2px;
    border-radius: 2px;
  }
</style>
