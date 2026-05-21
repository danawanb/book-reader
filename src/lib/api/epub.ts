import ePub from "epubjs";
import { convertFileSrc } from "@tauri-apps/api/core";

export async function extractEpubMetadata(
  filePath: string
): Promise<{ title?: string; author?: string }> {
  const book = ePub(convertFileSrc(filePath));
  try {
    await book.ready;
    const md = (book as any).packaging?.metadata ?? (book as any).package?.metadata ?? {};
    return {
      title: clean(md.title),
      author: clean(md.creator) ?? clean(md.author),
    };
  } finally {
    try {
      (book as any).destroy?.();
    } catch {}
  }
}

function clean(v: unknown): string | undefined {
  if (typeof v !== "string") return undefined;
  const t = v.trim();
  return t.length > 0 ? t : undefined;
}
