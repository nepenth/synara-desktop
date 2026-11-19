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

## iOS

iOS Settings uses the same groups, page names and section names for the
pages it has. The account summary row at the top of the list opens
Account; Log out is the last row.

| Group    | Page                | Sections                                                                                   |
| -------- | ------------------- | ------------------------------------------------------------------------------------------ |
| (top)    | Account             | Account · Profile · Avatar · Contact · Presence · Status                                   |
| App      | General             | Date & time                                                                                |
| App      | Appearance          | Theme · Text · Message display                                                             |
| App      | Notifications       | Permission · Direct messages · Rooms · Mentions · Keywords · Agent approvals · Previews · Troubleshooting |
| Chat     | Messages            | Drafts                                                                                     |
| Chat     | Media & links       | GIFs · Links                                                                               |
| Chat     | Privacy             | Activity · Blocked users                                                                   |
| Account  | Security & recovery | Verification · Recovery · Danger zone                                                      |
| Account  | Sessions            | Sessions · This session                                                                    |
| Advanced | Advanced            | Calls · Security details                                                                   |
| Advanced | Diagnostics         | Delivery · Local delivery diagnostics                                                      |
| Advanced | About               | Build · Links (What's new, Licenses, Privacy policy, Support)                              |

Notifications › Agent approvals links to the Agent notifications page
(agent accounts and message categories), and Notifications ›
Troubleshooting links to Diagnostics; Support links there too.

Every row, its accessibility identifier and the key it stores is pinned in
`synara-ios/Synara/Features/SettingsInventory.swift`. The root list is built
from it, and `SettingsInventoryTests` holds the page sources and the stored
keys to it, the same way `settingsInventory.ts` works on desktop.

Desktop pages, sections and rows iOS does not have, and why:

- **General › Updates** and **Keyboard shortcuts**: the App Store updates the
  app, and there are no global shortcuts on iOS. **Date format** is not
  offered; iOS follows the system's date format.
- **Appearance**: Light/Dark theme and Follow system theme colors (iOS
  always follows the system appearance), Monochrome mode, Accent color,
  Message text, fonts and Page zoom (iOS uses Dynamic Type), Message layout
  and spacing, Legacy username color, and Window (Linux shell only).
  Appearance keeps a base color and Thread display.
- **Notifications › System** has no Desktop notifications or Notification
  sound switch: iOS Settings owns both, so iOS shows the permission instead.
  Urgent agent approvals, Approve once without unlocking and Show message
  content in notifications are iOS-only (time-sensitive alerts and Lock
  Screen previews).
- **Messages › Composer** and **Timeline** (Enter adds a new line, Markdown
  formatting, Hide membership and profile changes) are not in iOS Settings.
- **Media & links › Media** (Load media automatically) is not in iOS Settings.
- **Account › Matrix ID** and **Password**: the Matrix ID shows in the Account
  section; iOS has no change-password form.
- **Security & recovery › Certificate identities** (X.509, experimental) is
  desktop-only.
- **Sessions › Export & import keys** is desktop-only.
- **Advanced › Experimental**, **Developer** and **Maintenance** (indexed
  search, hidden events, widgets, developer tools, reload) are desktop-only.
  iOS Advanced holds the call-service readout and the implementation
  readouts (session storage, message security) that used to sit among the
  security status rows.
- **Diagnostics** on iOS is the push delivery log, not desktop's capture
  settings.
- **About › Credits** is Licenses, a row of About.

"Sessions › This session" on iOS is the Verify this device button the
Account page used to have; Security & recovery › Verification has the same
action, as before.

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
