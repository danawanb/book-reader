<script lang="ts">
  interface OutlineItem {
    title: string;
    dest: unknown;
    items: OutlineItem[];
  }

  let {
    outline,
    navigateToDest,
  }: {
    outline: OutlineItem[] | null;
    navigateToDest: ((dest: unknown) => Promise<void>) | null;
  } = $props();

  let expanded = $state<Record<string, boolean>>({});

  function toggle(key: string) {
    expanded[key] = !expanded[key];
    expanded = { ...expanded };
  }

  async function go(dest: unknown) {
    if (!dest || !navigateToDest) return;
    await navigateToDest(dest);
  }
</script>

<div class="outline-panel">
  {#if outline === null}
    <p class="status">Loading outline…</p>
  {:else if outline.length === 0}
    <p class="status">No outline in this book.</p>
  {:else}
    <ul class="root">
      {#each outline as item, i (i)}
        {@render renderItem(item, `${i}`, 0)}
      {/each}
    </ul>
  {/if}
</div>

{#snippet renderItem(item: OutlineItem, key: string, depth: number)}
  <li>
    <div class="row" style:padding-left="{8 + depth * 12}px">
      {#if item.items.length > 0}
        <button
          class="caret"
          class:open={expanded[key]}
          onclick={() => toggle(key)}
          aria-label="Toggle"
        >▸</button>
      {:else}
        <span class="caret-spacer"></span>
      {/if}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <span class="title" onclick={() => go(item.dest)}>
        {item.title || "(untitled)"}
      </span>
    </div>
    {#if item.items.length > 0 && expanded[key]}
      <ul>
        {#each item.items as child, ci (ci)}
          {@render renderItem(child, `${key}-${ci}`, depth + 1)}
        {/each}
      </ul>
    {/if}
  </li>
{/snippet}

<style>
  .outline-panel {
    height: 100%;
    overflow-y: auto;
    background: #1e1e2e;
    padding: 6px 0;
  }
  .status {
    color: #6c7086;
    font-size: 13px;
    text-align: center;
    margin: 24px 12px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 4px;
    padding-right: 8px;
    padding-top: 2px;
    padding-bottom: 2px;
  }
  .caret {
    background: none;
    border: none;
    color: #6c7086;
    cursor: pointer;
    width: 16px;
    height: 16px;
    padding: 0;
    font-size: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.12s;
  }
  .caret.open {
    transform: rotate(90deg);
    color: #cdd6f4;
  }
  .caret-spacer {
    display: inline-block;
    width: 16px;
  }
  .title {
    flex: 1;
    color: #cdd6f4;
    font-size: 13px;
    line-height: 1.4;
    cursor: pointer;
    padding: 4px 6px;
    border-radius: 4px;
    transition: background 0.1s;
    word-break: break-word;
  }
  .title:hover {
    background: #313244;
    color: #89b4fa;
  }
</style>
