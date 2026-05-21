import { writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";

export interface Book {
  id: number;
  title: string;
  author: string | null;
  file_path: string;
  file_type: "pdf" | "epub";
  cover_path: string | null;
  total_pages: number | null;
  current_page: number;
  created_at: string;
}

export const books = writable<Book[]>([]);
export const loading = writable(false);

export async function loadBooks() {
  loading.set(true);
  try {
    const result = await invoke<Book[]>("get_all_books");
    books.set(result);
  } finally {
    loading.set(false);
  }
}

export async function deleteBook(id: number) {
  await invoke("delete_book", { id });
  books.update((bs) => bs.filter((b) => b.id !== id));
}

export async function updateProgress(bookId: number, page: number, totalPages: number) {
  await invoke("update_progress", { bookId, page, totalPages });
  books.update((bs) =>
    bs.map((b) => (b.id === bookId ? { ...b, current_page: page, total_pages: totalPages } : b))
  );
}
