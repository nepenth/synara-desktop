import { compareVersions } from '../../utils/desktopUpdater';
import { SYNARA_RELEASES_URL } from '../../utils/appLinks';

/**
 * "What's new" after an update: one notes file per release,
 * `release-notes/v<version>.json` at the repository root (format and limits:
 * docs/whats-new-notes.md, checked by scripts/check-whats-new.mjs). The app
 * bundles the file for its own version and shows its desktop items once.
 */

export type WhatsNewPlatform = 'desktop' | 'ios';

/** What the card and Settings › About show: the desktop part of one release's notes. */
export type WhatsNewNotes = {
  version: string;
  headline: string;
  items: string[];
  releaseUrl: string;
};

/** The checker allows at most this many items per platform; the card never shows more. */
export const MAX_WHATS_NEW_ITEMS = 4;

export const whatsNewReleaseUrl = (version: string): string =>
  `${SYNARA_RELEASES_URL}/tag/v${encodeURIComponent(version)}`;

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

/**
 * The desktop notes in a parsed notes file for `version`, or undefined when
 * the file is for another version, is not in the expected shape, or has no
 * desktop items. The release check rejects such files; this keeps a bad one
 * from drawing a broken card.
 */
export const desktopWhatsNew = (file: unknown, version: string): WhatsNewNotes | undefined => {
  if (!isRecord(file) || file.version !== version) return undefined;
  const headline = typeof file.headline === 'string' ? file.headline.trim() : '';
  if (!headline || !Array.isArray(file.items)) return undefined;
  const items = file.items
    .filter(
      (item): item is { text: string; platforms: unknown[] } =>
        isRecord(item) &&
        typeof item.text === 'string' &&
        item.text.trim() !== '' &&
        Array.isArray(item.platforms) &&
        item.platforms.includes('desktop')
    )
    .map((item) => item.text.trim())
    .slice(0, MAX_WHATS_NEW_ITEMS);
  if (items.length === 0) return undefined;
  return { version, headline, items, releaseUrl: whatsNewReleaseUrl(version) };
};

/**
 * The running version's notes among bundled notes files, keyed by path
 * (`…/release-notes/v3.2.0.json`), or undefined when there is no file for it.
 */
export const selectWhatsNew = (
  files: Record<string, unknown>,
  version: string
): WhatsNewNotes | undefined => {
  const suffix = `/v${version}.json`;
  const key = Object.keys(files).find((path) => path.endsWith(suffix));
  return key === undefined ? undefined : desktopWhatsNew(files[key], version);
};

/** Whether a session was restored at startup; `unknown` when the session store failed. */
export type WhatsNewSessionState = 'signed-in' | 'signed-out' | 'unknown';

export type WhatsNewStartupInput = {
  /** The last version whose notes were shown or skipped on this device; '' for none. */
  seenVersion: string;
  currentVersion: string;
  session: WhatsNewSessionState;
  hasNotes: boolean;
};

/**
 * - `record`: a fresh install; note the current version without showing anything.
 * - `show`: show the current version's notes once the main window is ready.
 * - `none`: nothing to do.
 */
export type WhatsNewStartupDecision = 'record' | 'show' | 'none';

/**
 * What to do with "What's new" when the app starts.
 *
 * There is no record on a fresh install, and also on the first launch of
 * the first version with this feature. A restored session tells them apart:
 * someone signed in has used an earlier version, so they see the notes; with
 * no session it is a first run, and the version is recorded silently. When
 * the session store failed, nothing is recorded, so the question is asked
 * again on the next start. Skipped versions show only the current notes, and
 * a downgrade or the same version shows nothing.
 */
export const decideWhatsNewAtStartup = ({
  seenVersion,
  currentVersion,
  session,
  hasNotes,
}: WhatsNewStartupInput): WhatsNewStartupDecision => {
  if (!seenVersion) {
    if (session === 'signed-out') return 'record';
    if (session === 'unknown') return 'none';
    return hasNotes ? 'show' : 'none';
  }
  if (compareVersions(seenVersion, currentVersion) >= 0) return 'none';
  return hasNotes ? 'show' : 'none';
};

/**
 * The update prompt's notes (`latest.json` `notes`, written by
 * scripts/generate-release-updater-metadata.mjs): a headline line, then "• "
 * lines. Older releases had free text, which comes back as paragraphs.
 */
export const parseUpdateNotes = (
  body: string | undefined
): { paragraphs: string[]; bullets: string[] } => {
  const paragraphs: string[] = [];
  const bullets: string[] = [];
  (body ?? '')
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .forEach((line) => {
      const bullet = /^(?:•|-|\*)\s+(.*)$/.exec(line);
      if (bullet) bullets.push(bullet[1]);
      else paragraphs.push(line);
    });
  return { paragraphs, bullets };
};
