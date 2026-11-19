/**
 * Settings search over individual settings, not just page names.
 *
 * Each settings surface (App, Room, Space) supplies an index of the rows and
 * sections its pages render. A source-scan test keeps every rendered title in
 * its index, so search cannot drift from the pages again.
 *
 * Queries are written the way people ask ("enable gif", "turn on read
 * receipts"): filler words are dropped while a meaningful word remains, a
 * plural also matches its singular, and results are ranked instead of
 * requiring every word.
 */

export type SettingsSearchEntry<T> = {
  page: T;
  /** Exactly the rendered row or section title; used to reveal the row. */
  title: string;
  description?: string;
  /** Other words people use for this setting. */
  synonyms?: string[];
  /** The section the row sits in; equal to `title` for a section itself. */
  section?: string;
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
  /** Higher sorts first. */
  rank: number;
};

export type SettingsSearchGroup<T> = {
  page: T;
  pageName: string;
  results: SettingsSearchResult<T>[];
};

/**
 * Words that say what to do with a setting rather than which one it is.
 * Dropped from a query while at least one other word remains.
 */
export const SEARCH_FILLER_WORDS = new Set([
  'a',
  'an',
  'can',
  'change',
  'disable',
  'disabled',
  'do',
  'does',
  'enable',
  'enabled',
  'find',
  'for',
  'how',
  'i',
  'in',
  'is',
  'it',
  'me',
  'my',
  'of',
  'off',
  'on',
  'option',
  'options',
  'setting',
  'settings',
  'support',
  'supported',
  'switch',
  'the',
  'to',
  'toggle',
  'turn',
  'want',
  'where',
]);

/** Every word typed, lower-cased. */
export const searchWords = (query: string): string[] =>
  query.toLowerCase().split(/\s+/).filter(Boolean);

/**
 * The words a query is matched on: what was typed, without filler words when
 * at least one meaningful word remains.
 */
export const searchTerms = (query: string): string[] => {
  const words = searchWords(query);
  const meaningful = words.filter((word) => !SEARCH_FILLER_WORDS.has(word));
  return meaningful.length > 0 ? meaningful : words;
};

/** The forms of a word that count as a match: "gifs" also matches "gif". */
const wordForms = (word: string): string[] =>
  word.length > 3 && word.endsWith('s') && !word.endsWith('ss')
    ? [word, word.slice(0, -1)]
    : [word];

/** The words to highlight in results for a query, singular forms included. */
export const highlightTerms = (query: string): string[] =>
  Array.from(new Set(searchTerms(query).flatMap(wordForms)));

const includesWord = (haystack: string, word: string): boolean =>
  wordForms(word).some((form) => haystack.includes(form));

const wholeWord = (haystack: string, word: string): boolean => {
  const forms = wordForms(word);
  return haystack.split(/[^a-z0-9@#.+-]+/).some((part) => forms.includes(part));
};

/**
 * How many of `words` have to match: all of one or two words, and most of
 * three or more (two of three, three of four).
 */
const requiredMatches = (count: number): number =>
  count <= 2 ? count : Math.ceil((count * 2) / 3);

type Fields = { title: string; synonyms: string; description: string; context: string };

/**
 * Scores one entry, or undefined when too few words match. Matching more
 * words always ranks higher; then the whole query as a phrase in the title
 * or synonyms; then where each word matched (title over synonyms over the
 * description over the page name); then a title that starts with, or has as a
 * whole word, the first word.
 */
const scoreEntry = (fields: Fields, words: string[], isSection: boolean): number | undefined => {
  let matched = 0;
  let placement = 0;
  words.forEach((word) => {
    if (includesWord(fields.title, word)) placement += 3;
    else if (includesWord(fields.synonyms, word)) placement += 2;
    else if (includesWord(fields.description, word)) placement += 1;
    else if (includesWord(fields.context, word)) placement += 0.5;
    else return;
    matched += 1;
  });
  if (matched === 0 || matched < requiredMatches(words.length)) return undefined;

  let score = matched * 100 + placement;
  if (words.length > 1) {
    const phrase = words.join(' ');
    if (fields.title.includes(phrase)) score += 40;
    else if (fields.synonyms.split('|').some((synonym) => synonym.includes(phrase))) score += 30;
  }
  const [first] = words;
  if (wordForms(first).some((form) => fields.title.startsWith(form))) score += 1;
  if (wholeWord(fields.title, first)) score += 2;
  // A row is a better answer than the section heading above it.
  if (isSection) score -= 0.5;
  return score;
};

/**
 * Matches case-insensitively, by substring, against each setting's title,
 * synonyms, description and page name. Results are grouped by page; the page
 * with the best result comes first, then the others in page order. A page
 * whose own name or keywords match appears even without a matching setting.
 */
export const searchSettings = <T>(
  pages: SettingsSearchPage<T>[],
  index: SettingsSearchEntry<T>[],
  query: string
): SettingsSearchGroup<T>[] => {
  const words = searchTerms(query);
  if (words.length === 0) return [];

  const groups = pages.flatMap((page, pageOrder) => {
    const pageText = [page.name, ...(page.keywords ?? [])].join(' ').toLowerCase();
    const results: SettingsSearchResult<T>[] = [];
    index
      .filter((entry) => entry.page === page.id)
      .forEach((entry) => {
        const rank = scoreEntry(
          {
            title: entry.title.toLowerCase(),
            synonyms: (entry.synonyms ?? []).join('|').toLowerCase(),
            description: (entry.description ?? '').toLowerCase(),
            context: page.name.toLowerCase(),
          },
          words,
          entry.section !== undefined && entry.section === entry.title
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
      .sort((a, b) => b.rank - a.rank)
      .filter((result) => {
        const key = result.title ?? '';
        if (seen.has(key)) return false;
        seen.add(key);
        return true;
      });
    if (unique.length === 0) {
      const pageRank = scoreEntry(
        { title: page.name.toLowerCase(), synonyms: '', description: pageText, context: '' },
        words,
        false
      );
      if (pageRank !== undefined) {
        unique.push({ page: page.id, pageName: page.name, rank: pageRank - 50 });
      }
    }
    return unique.length > 0
      ? [{ group: { page: page.id, pageName: page.name, results: unique }, pageOrder }]
      : [];
  });

  return groups
    .sort((a, b) => b.group.results[0].rank - a.group.results[0].rank || a.pageOrder - b.pageOrder)
    .map(({ group }) => group);
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
