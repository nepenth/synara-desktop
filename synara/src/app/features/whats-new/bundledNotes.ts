import { APP_VERSION } from '../../utils/appLinks';
import { selectWhatsNew, type WhatsNewNotes } from './whatsNewNotes';

/*
 * The release notes files at the repository root, read when the app is
 * built: Vite parses each one as JSON, so a malformed file fails the build.
 * Each is a few hundred bytes. Only the running version's desktop notes are
 * used; with no file for this version there is nothing to show.
 *
 * Kept apart from whatsNewNotes.ts: `import.meta.glob` exists only under Vite, and
 * the unit tests run the pure logic under Node.
 */
const files = import.meta.glob<unknown>('../../../../../release-notes/v*.json', {
  eager: true,
  import: 'default',
});

/** This version's "What's new", or undefined when the release has no notes. */
export const BUNDLED_WHATS_NEW: WhatsNewNotes | undefined = selectWhatsNew(files, APP_VERSION);
