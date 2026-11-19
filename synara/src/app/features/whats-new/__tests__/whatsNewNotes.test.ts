import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';
import {
  decideWhatsNewAtStartup,
  desktopWhatsNew,
  parseUpdateNotes,
  selectWhatsNew,
  type WhatsNewStartupInput,
} from '../whatsNewNotes';
import { normalizeWhatsNewSeenVersion } from '../../../state/settings';

const NOTES = {
  version: '3.2.0',
  headline: 'Easier settings, cleaner timestamps',
  items: [
    { text: 'Settings are grouped by what you want to do.', platforms: ['desktop'] },
    { text: 'Settings rows on iPhone respond to taps again.', platforms: ['ios'] },
    { text: 'After an update, Synara now shows what changed.', platforms: ['desktop', 'ios'] },
  ],
};

const decide = (input: Partial<WhatsNewStartupInput>) =>
  decideWhatsNewAtStartup({
    seenVersion: '',
    currentVersion: '3.2.0',
    session: 'signed-in',
    hasNotes: true,
    ...input,
  });

test('a fresh install records the version silently', () => {
  assert.equal(decide({ seenVersion: '', session: 'signed-out' }), 'record');
  // Also with no notes for this version.
  assert.equal(decide({ seenVersion: '', session: 'signed-out', hasNotes: false }), 'record');
});

test('the first version with this feature shows its notes to a signed-in user', () => {
  assert.equal(decide({ seenVersion: '', session: 'signed-in' }), 'show');
  assert.equal(decide({ seenVersion: '', session: 'signed-in', hasNotes: false }), 'none');
});

test('a failed session store neither shows nor records', () => {
  assert.equal(decide({ seenVersion: '', session: 'unknown' }), 'none');
});

test('an update shows the current notes once, skipped versions included', () => {
  assert.equal(decide({ seenVersion: '3.1.1' }), 'show');
  assert.equal(decide({ seenVersion: '2.9.0' }), 'show');
  // Signed out after an update: still shown once the shell is up.
  assert.equal(decide({ seenVersion: '3.1.1', session: 'signed-out' }), 'show');
  assert.equal(decide({ seenVersion: '3.1.1', hasNotes: false }), 'none');
});

test('the same version or a downgrade shows nothing', () => {
  assert.equal(decide({ seenVersion: '3.2.0' }), 'none');
  assert.equal(decide({ seenVersion: '3.3.0' }), 'none');
  assert.equal(decide({ seenVersion: '3.2.0', session: 'signed-out' }), 'none');
});

test('versions compare by number, not as text', () => {
  assert.equal(decide({ seenVersion: '3.9.0', currentVersion: '3.10.0' }), 'show');
  assert.equal(decide({ seenVersion: '3.10.0', currentVersion: '3.9.0' }), 'none');
  assert.equal(decide({ seenVersion: '3.2.9', currentVersion: '3.2.10' }), 'show');
});

test('only desktop items are shown, with the release link', () => {
  const notes = desktopWhatsNew(NOTES, '3.2.0');
  assert.deepEqual(notes, {
    version: '3.2.0',
    headline: 'Easier settings, cleaner timestamps',
    items: [
      'Settings are grouped by what you want to do.',
      'After an update, Synara now shows what changed.',
    ],
    releaseUrl: 'https://github.com/nepenth/synara-desktop/releases/tag/v3.2.0',
  });
});

test('notes for another version, of the wrong shape, or with no desktop items show nothing', () => {
  assert.equal(desktopWhatsNew(NOTES, '3.2.1'), undefined);
  assert.equal(desktopWhatsNew({ ...NOTES, headline: '' }, '3.2.0'), undefined);
  assert.equal(desktopWhatsNew({ ...NOTES, items: 'none' }, '3.2.0'), undefined);
  assert.equal(desktopWhatsNew(null, '3.2.0'), undefined);
  assert.equal(desktopWhatsNew([], '3.2.0'), undefined);
  assert.equal(
    desktopWhatsNew({ ...NOTES, items: [{ text: 'iOS only', platforms: ['ios'] }] }, '3.2.0'),
    undefined
  );
  // Malformed items are skipped; more than four are cut to four.
  const many = desktopWhatsNew(
    {
      ...NOTES,
      items: [
        { text: 42, platforms: ['desktop'] },
        { text: 'a', platforms: 'desktop' },
        ...['1', '2', '3', '4', '5'].map((text) => ({ text, platforms: ['desktop'] })),
      ],
    },
    '3.2.0'
  );
  assert.deepEqual(many?.items, ['1', '2', '3', '4']);
});

test('the bundle picks the running version and nothing else', () => {
  const files = {
    '../../../../../release-notes/v3.1.1.json': { ...NOTES, version: '3.1.1' },
    '../../../../../release-notes/v3.2.0.json': NOTES,
    '../../../../../release-notes/v13.2.0.json': { ...NOTES, version: '13.2.0' },
  };
  assert.equal(selectWhatsNew(files, '3.2.0')?.version, '3.2.0');
  assert.equal(selectWhatsNew(files, '3.1.1')?.version, '3.1.1');
  assert.equal(selectWhatsNew(files, '3.3.0'), undefined);
  assert.equal(selectWhatsNew({}, '3.2.0'), undefined);
});

test('every notes file in the repository loads for its own version', () => {
  const dir = join(process.cwd(), '..', 'release-notes');
  const names = readdirSync(dir).filter((name) => name.endsWith('.json'));
  assert.ok(names.length > 0);
  names.forEach((name) => {
    const version = name.slice(1, -'.json'.length);
    const notes = desktopWhatsNew(JSON.parse(readFileSync(join(dir, name), 'utf8')), version);
    assert.ok(notes, name);
    assert.ok(notes.items.length >= 1 && notes.items.length <= 4, name);
  });
  // The app's own version has notes, so the card and About have something to show.
  const { version } = JSON.parse(readFileSync(join(process.cwd(), 'package.json'), 'utf8'));
  assert.ok(names.includes(`v${version}.json`), `release-notes/v${version}.json`);
});

test('update prompt notes split into the headline and a list', () => {
  assert.deepEqual(
    parseUpdateNotes('Easier settings\n• Grouped settings\n• Cleaner timestamps\n'),
    { paragraphs: ['Easier settings'], bullets: ['Grouped settings', 'Cleaner timestamps'] }
  );
  // Free text from older releases stays paragraphs; "- " lists still list.
  assert.deepEqual(parseUpdateNotes('Bug fixes.\r\n\r\n- One\n- Two'), {
    paragraphs: ['Bug fixes.'],
    bullets: ['One', 'Two'],
  });
  assert.deepEqual(parseUpdateNotes(undefined), { paragraphs: [], bullets: [] });
});

test('the stored record keeps only a version', () => {
  assert.equal(normalizeWhatsNewSeenVersion('3.2.0'), '3.2.0');
  assert.equal(normalizeWhatsNewSeenVersion(' 3.10.1 '), '3.10.1');
  assert.equal(normalizeWhatsNewSeenVersion('3.2.0-rc.1'), '3.2.0-rc.1');
  assert.equal(normalizeWhatsNewSeenVersion('3.2'), '');
  assert.equal(normalizeWhatsNewSeenVersion(320), '');
  assert.equal(normalizeWhatsNewSeenVersion(undefined), '');
});
