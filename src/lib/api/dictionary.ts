export interface DictionaryEntry {
  word: string;
  phonetic?: string;
  phonetics: { text?: string; audio?: string }[];
  meanings: {
    partOfSpeech: string;
    definitions: { definition: string; example?: string }[];
    synonyms: string[];
  }[];
}

export async function lookupWord(word: string): Promise<DictionaryEntry[] | null> {
  const res = await fetch(
    `https://api.dictionaryapi.dev/api/v2/entries/en/${encodeURIComponent(word)}`,
  );
  if (res.status === 404) return null;
  if (!res.ok) throw new Error(`Dictionary API error: ${res.status}`);
  return (await res.json()) as DictionaryEntry[];
}
