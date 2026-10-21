/**
 * Settings search over individual settings, not just page names.
 *
 * Each settings surface (App, Room, Space) supplies an index of the rows and
 * sections its pages render. A source-scan test keeps every rendered title in
 * its index, so search cannot drift from the pages again.
 */

export type SettingsSearchEntry<T> = {
  page: T;
  /** Exactly the rendered row or section title; used to reveal the row. */
  title: string;
  description?: string;
  /** Other words people use for this setting. */
  synonyms?: string[];
};

export type SettingsSearchPage<T> = {
  id: T;
  name: string;
  /** Extra words that match the page as a whole. */
  keywords?: string[];
};

export type SettingsSearchResult<T> = {
  page: T;
  pageName: string;
  /** Undefined when the result is the page itself. */
  title?: string;
  description?: string;
  rank: number;
};

export type SettingsSearchGroup<T> = {
  page: T;
  pageName: string;
  results: SettingsSearchResult<T>[];
};

export const searchWords = (query: string): string[] =>
  query.toLowerCase().split(/\s+/).filter(Boolean);

const containsAll = (haystack: string, words: string[]): boolean =>
  words.every((word) => haystack.includes(word));

/**
 * Lower rank sorts first: every word in the title, then a title that starts
 * with the first word, then synonyms, then descriptions and page context.
 */
const rankEntry = (
  title: string,
  description: string,
  synonyms: string,
  context: string,
  words: string[]
): number | undefined => {
  const all = [title, synonyms, description, context].join(' ');
  if (!containsAll(all, words)) return undefined;
  if (containsAll(title, words)) {
    return title.startsWith(words[0]) || title.split(/\s+/).some((w) => w.startsWith(words[0]))
      ? 0
      : 1;
  }
  if (containsAll(`${title} ${synonyms}`, words)) return 2;
  return 3;
};

/**
 * Matches case-insensitively on every word, by substring, against each
 * setting's title, description, synonyms and its page name. Results are grouped
 * by page in page order; a page whose own name or keywords match appears even
 * without a matching setting.
 */
export const searchSettings = <T>(
  pages: SettingsSearchPage<T>[],
  index: SettingsSearchEntry<T>[],
  query: string
): SettingsSearchGroup<T>[] => {
  const words = searchWords(query);
  if (words.length === 0) return [];

  return pages.flatMap((page) => {
    const pageText = [page.name, ...(page.keywords ?? [])].join(' ').toLowerCase();
    const results: SettingsSearchResult<T>[] = [];
    index
      .filter((entry) => entry.page === page.id)
      .forEach((entry) => {
        const rank = rankEntry(
          entry.title.toLowerCase(),
          (entry.description ?? '').toLowerCase(),
          (entry.synonyms ?? []).join(' ').toLowerCase(),
          page.name.toLowerCase(),
          words
        );
        if (rank === undefined) return;
        results.push({
          page: page.id,
          pageName: page.name,
          title: entry.title,
          description: entry.description,
          rank,
        });
      });
    const seen = new Set<string>();
    const unique = results
      .sort((a, b) => a.rank - b.rank)
      .filter((result) => {
        const key = result.title ?? '';
        if (seen.has(key)) return false;
        seen.add(key);
        return true;
      });
    if (unique.length === 0 && containsAll(pageText, words)) {
      unique.push({ page: page.id, pageName: page.name, rank: 4 });
    }
    return unique.length > 0 ? [{ page: page.id, pageName: page.name, results: unique }] : [];
  });
};

/** Character ranges in `text` that match any search word, merged and sorted. */
export const matchRanges = (text: string, words: string[]): Array<[number, number]> => {
  const lower = text.toLowerCase();
  const ranges: Array<[number, number]> = [];
  words.forEach((word) => {
    let from = 0;
    for (;;) {
      const at = lower.indexOf(word, from);
      if (at < 0) break;
      ranges.push([at, at + word.length]);
      from = at + word.length;
    }
  });
  ranges.sort((a, b) => a[0] - b[0]);
  return ranges.reduce<Array<[number, number]>>((merged, range) => {
    const last = merged[merged.length - 1];
    if (last && range[0] <= last[1]) last[1] = Math.max(last[1], range[1]);
    else merged.push([...range]);
    return merged;
  }, []);
};

/** A short window of `text` around its first match, with ellipses when cut. */
export const matchSnippet = (text: string, words: string[], max = 90): string => {
  if (text.length <= max) return text;
  const first = matchRanges(text, words)[0];
  const start = first ? Math.max(0, Math.min(first[0] - 24, text.length - max)) : 0;
  const end = Math.min(text.length, start + max);
  return `${start > 0 ? '…' : ''}${text.slice(start, end).trim()}${end < text.length ? '…' : ''}`;
};
