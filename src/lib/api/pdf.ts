import * as pdfjsLib from "pdfjs-dist";
import { convertFileSrc } from "@tauri-apps/api/core";

let workerInitialized = false;

function initWorker() {
  if (workerInitialized) return;
  pdfjsLib.GlobalWorkerOptions.workerSrc = new URL(
    "pdfjs-dist/build/pdf.worker.min.mjs",
    import.meta.url
  ).toString();
  workerInitialized = true;
}

export async function extractPdfMetadata(
  filePath: string
): Promise<{ title?: string; author?: string }> {
  initWorker();
  const pdf = await pdfjsLib.getDocument(convertFileSrc(filePath)).promise;
  try {
    const meta = await pdf.getMetadata();
    const info = (meta?.info ?? {}) as Record<string, unknown>;
    return {
      title: cleanString(info.Title),
      author: cleanString(info.Author),
    };
  } finally {
    await pdf.destroy();
  }
}

function cleanString(v: unknown): string | undefined {
  if (typeof v !== "string") return undefined;
  const t = v.trim();
  return t.length > 0 ? t : undefined;
}
