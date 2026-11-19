/**
 * Source scan used by the settings-search completeness tests: the literal
 * titles (and descriptions) a settings page renders on `<SettingTile>` and
 * `<SettingsSection>`, and on `<InfoCard>` where a test asks for it.
 *
 * A literal string is taken as is; `t('key', 'Fallback')` yields its
 * fallback; any other expression yields every string literal inside it (for
 * example both sides of a Room/Space ternary). Expressions with no string
 * literal (titles computed from data) and JSX or template values are skipped.
 */
export type ScannedComponent = 'SettingTile' | 'SettingsSection' | 'InfoCard';

export type ScannedTitle = {
  /** Candidate titles; the index must contain at least one of them. */
  candidates: string[];
  /** Candidate descriptions, when the description is a literal or `t()` fallback. */
  descriptions: string[];
  component: ScannedComponent;
};

const DEFAULT_COMPONENTS: ScannedComponent[] = ['SettingTile', 'SettingsSection'];

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

/** The values of attribute `name` on the tag whose attributes start at `start`. */
const attributeValues = (
  source: string,
  start: number,
  attributesLength: number,
  name: string
): string[] => {
  const attributes = source.slice(start);
  const at = attributes.search(new RegExp(`\\b${name}=`));
  // Attribute must belong to this tag, not a later element.
  if (at < 0 || at > attributesLength) return [];
  const valueAt = start + at + name.length + 1;
  let values: string[] = [];
  if (source[valueAt] === '"') {
    const end = source.indexOf('"', valueAt + 1);
    values = [source.slice(valueAt + 1, end)];
  } else if (source[valueAt] === '{') {
    const expression = readBraced(source, valueAt) ?? '';
    // Values built from JSX or template data are not fixed copy.
    if (!/[<`]/.test(expression)) values = literals(expression);
  }
  return values.map((value) => value.trim()).filter(Boolean);
};

export const scanSettingTitles = (
  source: string,
  components: ScannedComponent[] = DEFAULT_COMPONENTS
): ScannedTitle[] => {
  const tag = new RegExp(`<(${components.join('|')})\\b([\\s\\S]*?)(\\/?>)`, 'g');
  const titles: ScannedTitle[] = [];
  for (const match of source.matchAll(tag)) {
    const start = (match.index ?? 0) + match[0].indexOf(match[2]);
    const candidates = attributeValues(source, start, match[2].length, 'title');
    if (candidates.length === 0) continue;
    titles.push({
      candidates,
      descriptions: attributeValues(source, start, match[2].length, 'description'),
      component: match[1] as ScannedComponent,
    });
  }
  return titles;
};
