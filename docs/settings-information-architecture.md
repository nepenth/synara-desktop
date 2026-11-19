# Settings information architecture

How app Settings is organised on desktop, why, and where a new setting goes.
Room and space settings (`features/common-settings`, `room-settings`,
`space-settings`) are separate and not covered here.

## Principles

1. **Group by what the user is trying to do**, not by how the setting is
   implemented. "Turn on GIFs" belongs with media in chat, not with the
   switches that happen to live in the same React component.
2. **Everyday before advanced.** The groups run App, Chat, Account,
   Advanced. Experimental, developer and maintenance items live on Advanced,
   never among everyday rows.
3. **One home per concept.** Each stored setting has exactly one row. Other
   places (the composer, error messages, the account avatar) link to that row
   instead of repeating the control.
4. **Same names on desktop and iOS** for the pages both have.
5. **Destructive last.** Within a page, the destructive or disruptive section
   is last (Danger zone on Security & recovery, Maintenance on Advanced), and
   Log out sits at the foot of the Settings nav.
6. **Common before rare** inside a section, and sentence case for every title.

## Structure

| Group    | Page                      | Sections                                                                     |
| -------- | ------------------------- | ---------------------------------------------------------------------------- |
| App      | General                   | Updates (desktop app) · Date & time · Keyboard shortcuts                     |
| App      | Appearance                | Theme · Text · Message display · Window (Linux shell)                        |
| App      | Notifications             | System · Agent notifications · Direct messages · Rooms · Mentions · Keywords |
| Chat     | Messages                  | Composer · Timeline · Drafts                                                 |
| Chat     | Media & links             | Media · GIFs · Links                                                         |
| Chat     | Privacy                   | Activity · Blocked users                                                     |
| Account  | Account                   | Profile · Matrix ID · Password · Contact                                     |
| Account  | Security & recovery       | Verification · Recovery · Certificate identities · Danger zone               |
| Account  | Sessions                  | This session · Other sessions · Export & import keys                         |
| Advanced | Advanced                  | Experimental · Calls · Developer · Maintenance                               |
| Advanced | Diagnostics (desktop app) | Diagnostic capture · Stored report · Desktop integration · Privacy           |
| Advanced | About                     | Credits                                                                      |

The full list of rows, with the page and section each lives on, what it
stores, and where it was before the 2026-11 regroup, is pinned in
`synara/src/app/features/settings/__tests__/settingsInventory.ts`. Tests hold
the search index, the page sources and the rendered pages to it, so a row
cannot be dropped or moved without changing that file on purpose.

## Where a new setting goes

Ask what the user is doing when they want it:

- Changing how Synara itself behaves on this device (updates, clock, global
  keys): **General**.
- Changing how Synara looks: **Appearance**.
- Deciding what alerts them: **Notifications**.
- Writing messages or reading the timeline: **Messages**.
- Images, video, GIFs, link previews and embeds, especially anything that
  contacts a third party: **Media & links**.
- What others can see about them, or who can reach them: **Privacy**.
- Their profile and sign-in details: **Account**; encryption and recovery:
  **Security & recovery**; logged-in devices: **Sessions**.
- Anything experimental, for developers, a status readout, or a maintenance
  action: **Advanced**. Support data goes on **Diagnostics**.

Then:

1. Add the row to the page with a plain string `title` (or `t('key',
'Fallback')`), so search can find and reveal it.
2. Add it to `settingsSearchIndex.ts` with the same `page`, `section`, title
   and description, plus synonyms people would type. If the row only renders
   on some builds, give the entry the same `when` condition.
3. Add it to `settingsInventory.ts`.
4. Keep platform conditions on the row itself (desktop-only, Linux-only,
   native-session-only, permission-dependent).

## Opening Settings at a place

- `useOpenAppSettings()` opens Settings over the current screen, optionally
  at a `SettingsLocation` (`{ page, setting }`), and scrolls to and
  highlights that row. `GIF_SEARCH_SETTING` and
  `verificationSettingsLocation()` in `settingsPages.ts` are examples.
- The route `/settings/` opens on the last page viewed in this run of the
  app (General the first time); `/settings/<page>/?setting=<row title>` opens
  a page and row. `synara://settings/<page>/` links do the same.
- On a phone-sized window Settings opens on the page list unless a page is
  asked for.

## Entry points

- **The account avatar** in the bottom bar is the Settings button: it opens
  Settings and closes it when pressed again. The account menu it used to
  open is gone; Log out is in the Settings nav footer. When this session is
  unverified the avatar shows a warning badge and opens Security & recovery
  at Verify this session; when other sessions need review it shows their
  count and opens Sessions. Cmd/Ctrl+, and the Settings place key toggle the
  same way.
- The bottom bar is mounted once for the signed-in client shell
  (`NavBottomBarHost` in `pages/Router.tsx`) and placed into each pane's list
  column by `NavBottomBarSlot`, so the avatar, its badge and its probes keep
  their state across Home, Activity, Approvals, Explore and spaces.
- **The composer**: with a GIF provider configured and GIF search off, the
  "+" menu keeps its GIF item, which opens the Enable GIFs? prompt with Open
  settings; `/gif` with GIF search off says where the switch is and has an
  Open settings action.
- **Copy elsewhere** names the path: Experimental widgets point to Settings →
  Advanced → Experimental, indexed search to the same place.

## Search

Settings search ignores filler words (enable, turn, on, off, support,
setting, how, to, the, my, where…) while a meaningful word remains, matches
a plural to its singular, requires all of one or two words and most of
longer queries, and ranks results: more words matched, then the query as a
phrase, then title over synonyms over description. A single word keeps exact
substring matching. Room and space settings use the same search.

## Names iOS mirrors

iOS should use the same page names for the pages it has: **Messages**,
**Media & links**, **Privacy** and **Advanced** (alongside the existing
General, Appearance, Notifications, Account, Security & recovery and
Sessions). iOS's "GIFs & Link Previews" page becomes Media & links.

## UX review items this resolves

From `docs/ux-audit/review-2/review-desktop-settings.md`:

- **SET-3** (General order and misplaced items): General is now Updates,
  Date & time and Keyboard shortcuts. Calls, Widgets and Reload application
  moved to Advanced; diagnostics content was already on Diagnostics.
- **SET-5** (devices and sessions): "Verify this device" now leads to
  Security & recovery where the control is; reviewing other sessions leads
  to Sessions.
- **SET-19** (account page structure): Blocked users moved to Privacy;
  Account is profile, Matrix ID, password and contact.
- **SET-24** (Developer tools pages): the separate page is gone; its rows
  are under Advanced › Developer, and the never-rendered "Global" row is
  now the Account data row (still only outside native sessions).
- **SET-29** (nav): Settings reopens on the last page viewed. The single-item
  groups in room and space settings are not part of this change.
- **SET-30** (terminology): partly. Section and page names follow this
  document; "device" in the avatar's unverified label is unchanged.
