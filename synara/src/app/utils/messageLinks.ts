import { find } from 'linkifyjs';

/**
 * Endings that are real top-level domains but, written bare, almost always
 * name a file: `README.md`, `setup.py`, `main.rs`, `build.sh`, `notes.zip`.
 * A bare match with one of these is not a link; the same name with a scheme
 * (`https://example.md`) or `www.` still is.
 */
const FILE_LIKE_BARE_TLDS: ReadonlySet<string> = new Set(['md', 'py', 'rs', 'sh', 'zip', 'mov']);

/** A character that glues a match to the word before it. */
const JOINED_BEFORE = /[\p{L}\p{N}_\-./\\~]/u;

const SCHEME_OR_WWW = /^(?:[a-z][a-z0-9+.-]*:|www\.)/i;

export type MessageLink = {
  start: number;
  end: number;
  /** As written in the text. */
  value: string;
  /** Openable address (`http://` added to bare domains, `mailto:` to emails). */
  href: string;
};

/**
 * Whether linkify's match is a link a reader meant. linkify finds
 * `SUBMISSION.md` inside `/Users/me/README_SUBMISSION.md` (an underscore is
 * not allowed in a domain, so the match starts after it) and treats `.md`
 * as a top-level domain, so a file path became a link to submission.md.
 */
const isIntendedLink = (text: string, start: number, value: string, type: string): boolean => {
  if (type !== 'url') return true;
  // Written with a scheme (`...https://example.org`), it is meant as a link.
  if (/^[a-z][a-z0-9+.-]*:/i.test(value)) return true;
  const before = start > 0 ? text[start - 1] : '';
  // A bare name glued to a longer word, path or file name is not an address.
  if (before && JOINED_BEFORE.test(before)) return false;
  if (SCHEME_OR_WWW.test(value)) return true;
  const host = value.split(/[/?#:]/, 1)[0] ?? '';
  const tld = host.slice(host.lastIndexOf('.') + 1).toLowerCase();
  return !FILE_LIKE_BARE_TLDS.has(tld);
};

/** The web and mail links in plain message text, in order. */
export const findMessageLinks = (text: string): MessageLink[] => {
  if (!text.trim()) return [];
  return find(text)
    .filter((match) => match.type === 'url' || match.type === 'email')
    .filter((match) => isIntendedLink(text, match.start, match.value, match.type))
    .map((match) => ({ start: match.start, end: match.end, value: match.value, href: match.href }));
};

export type MessageTextSegment = { text: string; href?: string };

/** Text split into plain runs and links, for renderers. */
export const segmentMessageLinks = (text: string): MessageTextSegment[] => {
  const segments: MessageTextSegment[] = [];
  let cursor = 0;
  findMessageLinks(text).forEach((link) => {
    if (link.start > cursor) segments.push({ text: text.slice(cursor, link.start) });
    segments.push({ text: text.slice(link.start, link.end), href: link.href });
    cursor = link.end;
  });
  if (cursor < text.length) segments.push({ text: text.slice(cursor) });
  return segments;
};
