import { writable } from "svelte/store";

const STORAGE_KEY = "book_reader_settings";

interface Settings {
  openaiKey: string;
  openaiModel: string;
}

function loadSettings(): Settings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch {}
  return { openaiKey: "", openaiModel: "gpt-4o-mini" };
}

function createSettings() {
  const { subscribe, set, update } = writable<Settings>(loadSettings());
  return {
    subscribe,
    set(val: Settings) {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(val));
      set(val);
    },
    update(fn: (s: Settings) => Settings) {
      update((s) => {
        const next = fn(s);
        localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
        return next;
      });
    },
  };
}

export const settings = createSettings();
