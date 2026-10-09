/**
 * Source scan used by the settings-search completeness tests: the literal
 * titles a settings page renders on `<SettingTile>` and `<SettingsSection>`.
 *
 * A literal string title is taken as is; `t('key', 'Fallback')` yields its
 * fallback; any other expression yields every string literal inside it (for
 * example both sides of a Room/Space ternary). Expressions with no string
 * literal (titles computed from data) are skipped.
 */
export type ScannedTitle = {
  /** Candidate titles; the index must contain at least one of them. */
  candidates: string[];
  component: 'SettingTile' | 'SettingsSection';
};

const TAG = /<(SettingTile|SettingsSection)\b([\s\S]*?)(\/?>)/g;

const readBraced = (source: string, open: number): string | undefined => {
  let depth = 0;
  for (let at = open; at < source.length; at += 1) {
    const char = source[at];
    if (char === '{') depth += 1;
    if (char === '}') {
      depth -= 1;
      if (depth === 0) return source.slice(open + 1, at);
    }
  }
  return undefined;
};

const literals = (expression: string): string[] => {
  const translated = Array.from(
    expression.matchAll(/\bt\(\s*(['"`])[^'"`]*\1\s*,\s*(['"`])((?:(?!\2).)*)\2/g)
  ).map((match) => match[3]);
  if (translated.length > 0) return translated;
  return Array.from(expression.matchAll(/(['"])((?:(?!\1).)+)\1/g)).map((match) => match[2]);
};

export const scanSettingTitles = (source: string): ScannedTitle[] => {
  const titles: ScannedTitle[] = [];
  for (const tag of source.matchAll(TAG)) {
    const start = (tag.index ?? 0) + tag[0].indexOf(tag[2]);
    const attributes = source.slice(start);
    const titleAt = attributes.search(/\btitle=/);
    // Attribute must belong to this tag, not a later element.
    if (titleAt < 0 || titleAt > tag[2].length) continue;
    const valueAt = start + titleAt + 'title='.length;
    let candidates: string[] = [];
    if (source[valueAt] === '"') {
      const end = source.indexOf('"', valueAt + 1);
      candidates = [source.slice(valueAt + 1, end)];
    } else if (source[valueAt] === '{') {
      const expression = readBraced(source, valueAt) ?? '';
      // Titles built from JSX or template data are not fixed settings.
      if (!/[<`]/.test(expression)) candidates = literals(expression);
    }
    candidates = candidates.map((candidate) => candidate.trim()).filter(Boolean);
    if (candidates.length > 0) {
      titles.push({ candidates, component: tag[1] as ScannedTitle['component'] });
    }
  }
  return titles;
};
