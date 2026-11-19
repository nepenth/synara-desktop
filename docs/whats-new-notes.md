# What's new notes

Every release has one short notes file, `release-notes/vX.Y.Z.json`, that the
apps show to people after they update. It is not the changelog: the changelog
and `docs/releases/vX.Y.Z.md` are for contributors; this is for the person who
just restarted Synara and wants to know what is different.

## Where the notes appear

- **Desktop, after an update.** A small card in the corner of the main window,
  "What's new in X.Y.Z", with the headline, the desktop items and a "Full
  release notes" link to the GitHub release. It shows once, after the signed-in
  window is ready, and not on a fresh install. Settings → About shows the same
  notes at any time. Only the notes for the version that is running are shown;
  someone who skipped versions sees the newest notes only.
- **Desktop, before an update (macOS and Windows).** The release workflow writes
  the headline and the desktop items into the updater metadata (`latest.json`
  `notes`), so the "Synara X.Y.Z is available" prompt lists them. Linux package
  updates (apt, pacman, paru) do not go through this prompt; Linux users see the
  card after the update instead.
- **iOS.** The iOS app reads the same files and shows its own items.

The desktop app bundles the file for its own version when it is built
(`synara/src/app/features/whats-new/bundledNotes.ts`), so the card works offline
and in Linux packages. A malformed file fails the build.

## Format

```json
{
  "version": "3.2.0",
  "headline": "Easier settings, cleaner timestamps",
  "items": [
    { "text": "Click your profile picture to open Settings; click it again to close.", "platforms": ["desktop"] },
    { "text": "Settings rows on iPhone respond to taps again.", "platforms": ["ios"] },
    { "text": "After an update, Synara now shows what changed.", "platforms": ["desktop", "ios"] }
  ]
}
```

`npm run check:whats-new` (`scripts/check-whats-new.mjs`) enforces the limits
below in CI and in the release workflow, and requires a file for the version in
`package.json`. `npm run bump:version` stops if the new version has no file, so
write the notes before bumping.

| Rule      | Limit                                                                                   |
| --------- | --------------------------------------------------------------------------------------- |
| version   | Equals the file name (`v3.2.0.json` holds `"3.2.0"`).                                   |
| headline  | 1–60 characters, one line of plain text.                                                |
| item text | 1–140 characters, one line of plain text: no Markdown, no HTML, no line breaks.         |
| platforms | A non-empty list of `desktop` and `ios`.                                                |
| items     | At least 1 and at most 4 per platform. An item for both platforms counts for each one.  |
| keys      | Only `version`, `headline`, `items`, and `text` and `platforms` in an item.             |

## Writing them

- **Audience:** someone who uses Synara every day and does not read the
  changelog. Write about what they will notice, not how it was built.
- **Headline:** a few words that sum up the release, like a TestFlight note's
  first line. No version number; the card's title already has it.
- **Items:** one plain sentence each, starting with the thing that changed
  ("Settings are grouped…", "Picking an emoji no longer…"). Say what is better
  now. Leave out issue numbers, internal names, and anything people cannot see.
- **Choose:** at most four per platform. Pick the changes people will notice
  first; the "Full release notes" link covers the rest.
- **Fixes count:** "Picking an emoji no longer takes the composer screen down"
  is a good item when that crash is what people hit.
