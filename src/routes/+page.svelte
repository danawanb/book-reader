<script lang="ts">
  import { onMount } from "svelte";
  import Library from "$lib/components/Library.svelte";
  import Reader from "$lib/components/Reader.svelte";
  import Settings from "$lib/components/Settings.svelte";
  import { loadBooks, type Book } from "$lib/stores/books";

  let view = $state<"library" | "reader">("library");
  let activeBook = $state<Book | null>(null);
  let showSettings = $state(false);

  function openBook(book: Book) {
    activeBook = book;
    view = "reader";
  }

  function goBack() {
    view = "library";
    activeBook = null;
    loadBooks();
  }

  onMount(() => {
    loadBooks();
  });
</script>

{#if view === "library"}
  <Library onOpen={openBook} onSettings={() => (showSettings = true)} />
{:else if view === "reader" && activeBook}
  <Reader book={activeBook} onBack={goBack} />
{/if}

{#if showSettings}
  <Settings onClose={() => (showSettings = false)} />
{/if}
