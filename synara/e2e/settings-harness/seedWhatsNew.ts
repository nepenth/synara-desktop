// Imported before the app, whose settings atoms read storage when they load.
//
// The harness starts as a signed-in session with no "What's new" record, which
// is an update from before the feature, so every page would show the card.
// Unless the URL has `?whatsNew`, mark this version's notes as seen; specs
// about the card add `?whatsNew` and keep whatever the app records.
import packageInfo from '../../package.json';

if (!new URLSearchParams(location.search).has('whatsNew')) {
  let stored: Record<string, unknown>;
  try {
    stored = JSON.parse(localStorage.getItem('platformSettings') ?? '{}') ?? {};
  } catch {
    stored = {};
  }
  if (!stored.whatsNewSeenVersion) {
    localStorage.setItem(
      'platformSettings',
      JSON.stringify({ ...stored, whatsNewSeenVersion: packageInfo.version })
    );
  }
}
