# Desktop settings, dialogs and secondary flows — UX/UI review (code reading, main @ 3231c7bb)

Paths below are relative to `synara/src/app/` unless stated. Line numbers are from the current tree.

## Summary

1. The settings *frame* is already Slack-grade (grouped nav with search-and-reveal, `SettingsPage` → `SettingsSection` → divided rows, critical-toned Danger Zone, proper dialog semantics); the *content* inside it has not caught up — General still leads with experimental Widgets and ends with two header-less diagnostics cards, Appearance crams five colour controls into a right-hand control slot, Devices uses "Current/Others" as section names.
2. Copy is split between two conventions: ~45 Title Case row titles/dialog titles/buttons ("Hide Membership Change", "Setup Device Verification", "Check for Updates") versus newer sentence-case rows ("Link previews", "Shorten long links") — the single most visible inconsistency against the restyled timeline.
3. There is no shared confirmation-dialog pattern: Log out (no X, stacked buttons, Cancel), Leave room (X, no Cancel, no room name, Enter does nothing), Remove avatar (X, one Critical Soft button), Room upgrade (Secondary button for an irreversible action), Diagnostics (`window.confirm`), updater (right-aligned, no header).
4. Verification/cross-signing/identity dialogs block Escape and click-outside even when nothing is running; forms across settings rely on placeholders instead of labels; inline errors use four different treatments (bold red, plain red, `priority="500"` grey, `role=alert` unstyled).
5. iOS counterparts exist for most of this (Settings root, Account, Notifications, Approvals, shared `IDENTITY_RESET_COPY`); terminology drifts between platforms (Devices vs Sessions, "Security & Recovery", "Keywords").

## Findings (ordered by impact)

### SET-1 · P0 · One confirmation-dialog pattern for destructive and irreversible actions
- **Where:** `components/LogoutDialog.tsx:42-100`; `components/leave-room-prompt/LeaveRoomPrompt.tsx:59-105`; `components/leave-space-prompt/LeaveSpacePrompt.tsx:59-105`; `features/settings/account/Profile.tsx:270-305` (Remove Avatar); `features/common-settings/general/RoomUpgrade.tsx:106-154`; `features/common-settings/general/RoomEncryption.tsx:165-193`; `features/desktop-updater/DesktopUpdaterProvider.tsx:144-213`; `features/cross-signing/IdentityReset.tsx:163-175`.
- **Today:** Log out: header without close X, body "You're about to log out. Are you sure?", two full-width stacked buttons (Critical then Cancel). Leave room/space: X in header, "Are you sure you want to leave this room?" (no room name), one Critical button, no Cancel, `type="submit"` on a button with no `<form>` so Enter does nothing (:88). Remove Avatar: one Critical *Soft* button, no Cancel. Room Upgrade: bold red "This action is irreversible!" then a *Secondary* button performs the irreversible act. Enable Encryption: "Are you sure? Once enabled, encryption cannot be disabled!" with a Primary button. Updater: no header, buttons right-aligned, different from every other dialog. Identity reset: horizontal [Critical][Cancel]. Only IdentityReset (:124), redact confirm and store recovery use `role="alertdialog"`.
- **Proposal (Slack-informed):** one `ConfirmDialog` built on `components/modal-dialog`: sentence-case title that names the object ("Leave #design-review?", "Log out of Synara?", "Remove your avatar?"), one body sentence stating the consequence, footer right-aligned `[Cancel] [Leave]` with the destructive button Critical Solid, `role="alertdialog"`, Enter activates the primary (wrap in `<form onSubmit>`), initial focus on Cancel for destructive dialogs, Esc = Cancel, no exclamation marks. Use it for Log out, Leave, Remove avatar, Upgrade, Enable encryption, Clear diagnostics, Reset identity step 1.
- **Effort:** M · **Risk:** low (presentation only; async states preserved) · **iOS:** yes — iOS uses `.signOutConfirmation`/`confirmationDialog`; align titles and button labels.

### SET-2 · P0 · Title Case vs sentence case across settings copy
- **Where (sample of ~45):** row titles `features/settings/general/General.tsx:1046,1067,1076,1082,1119,1132,1145,1232` ("24-Hour Time Format", "ENTER for Newline", "Markdown Formatting", "Hide Typing & Read Receipts", "Hide Membership Change", "Hide Profile Change", "Load Media Automatically", "Show Hidden Events"); `appearance/Appearance.tsx:192,243,391,408,561,600,639,820,824`; `devices/Devices.tsx:74`, `devices/LocalBackup.tsx:125,258,316`, `devices/OtherDevices.tsx:184`; `notifications/Notifications.tsx:127,135,154,259,267`, `SystemNotification.tsx:86,128,159,164`; `account/Password.tsx:41,94`, `ContactInfo.tsx:86,93`, `IgnoredUserList.tsx:143`; `diagnostics/Diagnostics.tsx:181,189,206,223,240,259`; nav labels `Settings.tsx:126` ("Developer Tools"); section titles `General.tsx:1043,889` ("Date & Time", "Software Updates"); dialog titles `JoinAddressPrompt.tsx:83`, `Verification.tsx:259` ("Setup Device Verification"), `nativeIdentityReset.ts:16,42-45` ("Reset Identity", "Verify This Device", "Use Recovery Key", "Open Account Page"); buttons `DesktopUpdaterProvider.tsx:178,188,436`, `NativeDeviceVerification.tsx:236,244,582`, `Verification.tsx:101`, `AddExisting.tsx:385`.
- **Today:** mixed with sentence-case neighbours on the same page: "Link previews", "Shorten long links", "Indexed message search" (`General.tsx:1196-1216`), "Encrypted state events (experimental)" (`DevelopTools.tsx:129`), "Message history for new sessions" (`Devices.tsx:97`), "Agent accounts" (`AgentNotifications.tsx:116`).
- **Proposal:** sentence case everywhere (titles, section headers, dialog titles, buttons, chips); capitals only for proper nouns and product terms (Later, Approvals, Notes, Threads, Spaces, Matrix ID, GIF, X). "Setup" → "Set up". Do it as one mechanical pass per file and update the string-matching tests (`features/settings/__tests__/settingsSurfaceGuard.test.ts:79,118`, e2e specs) in the same change.
- **Effort:** M (mechanical, but many tests reference strings) · **Risk:** low · **iOS:** yes for shared constants (`IDENTITY_RESET_COPY`, `VERIFY_DEVICE_PROMPT_COPY` are mirrored in `synara-ios` `IdentityResetCopy`).

### SET-3 · P1 · General page: order, header-less cards and misplaced diagnostics
- **Where:** `features/settings/general/General.tsx:1246-1259` (order), `:602-684` + `:1254` (DesktopShortcutsSection rendered as a bare `SequenceCard` outside any `SettingsSection`), `:806-882` + `:1257` (DesktopIntegrationSection, same), `:688-748` (Native Session Store tile) duplicated at `developer-tools/DevelopTools.tsx:56-66`; `DevelopTools.tsx:147-150` ("Experimental Widgets — Widgets are configured under General." dead pointer row).
- **Today:** section order is Widgets (experimental) → Calls (read-only status) → Date & Time → Editor → Messages → [bare card] Desktop Shortcuts → Storage → Software Updates → [bare card] Desktop Integration (tray/portal readiness + "Copy diagnostics"). The two bare cards have no section header, use inline `padding: 12` (:624, :811), and the integration card is diagnostics content living on the first page a user sees.
- **Proposal:** General = Updates (desktop only, first), Date & time, Composer (rename "Editor"), Messages, Keyboard shortcuts (proper `SettingsSection`), Storage, Widgets (experimental, last, with a "Beta" badge). Move Desktop integration and Native session store to Diagnostics; delete the DevTools pointer row; move "Calls/MatrixRTC" status to Diagnostics or About. Give Keyboard shortcuts three labelled rows (see SET-8).
- **Effort:** M · **Risk:** low; search index (`settingsSearchIndex.ts`) must follow · **iOS:** partial — iOS has "GIFs & Link Previews" as its own page; no change needed there.

### SET-4 · P1 · Appearance: theme choice is four rows, colour controls are crammed into the control slot
- **Where:** `features/settings/appearance/Appearance.tsx:383-404` (System Theme switch + "Theme — Theme to use when system theme is not enabled" + conditional Light/Dark pickers :191-292); `:413-484` Base Color and `:486-546` Accent Color put a ramp preview, 6 swatches, a native `<input type=color>`, a hex `Input` and Reset into `after=` with `minWidth: toRem(220)`; `:361` hardcoded `rgba(0,0,0,0.18)` swatch outline (invisible on dark surfaces).
- **Proposal:** one segmented row "Appearance: System · Light · Dark" (same quiet-button pattern as Thread display :564-587); show "Light theme"/"Dark theme" pickers only when System is selected. Make Base colour and Accent colour full-width rows: title + one-line description, then a left-aligned swatch row (presets + custom well + hex + Reset) as `children`, preview to the right. Shorten descriptions ("Tints the rail, room list and chat."). Replace the rgba outline with `color.Surface.ContainerLine`. Reconsider "Legacy Username Color" (:824) now that the Slack-style timeline uses bold names — rename "Colour sender names per person" or drop.
- **Effort:** M · **Risk:** low · **iOS:** AppearanceSettingsView exists; match the System/Light/Dark wording.

### SET-5 · P1 · Devices page: naming, section names and terminology
- **Where:** `features/settings/Settings.tsx:96-98` (nav "Devices"); `features/settings/devices/Devices.tsx:65-66,171` (page "Devices", sections "Security", "Current"); `devices/OtherDevices.tsx:175` ("Others"); `Devices.tsx:74-75` ("Device Verification — To verify device identity and grant access to encrypted messages." — a fragment); `:100-104` static "Key backup" Badge sitting in the control slot; `DeviceTile.tsx:211` error says "this session", `:215` "Device names are visible to public."; `OtherDevices.tsx:263` "Log out of selected devices."
- **Today vs iOS:** iOS root has "Security & Recovery" and Account → "Sessions" with the description "Log out a session to revoke it on the homeserver. This device uses Log Out instead." (`synara-ios/Synara/Features/SettingsView.swift:143,543-545`).
- **Proposal:** split into two nav items mirroring iOS: "Security & recovery" (verification, key backup, recovery, X.509, reset identity in its own critical "Danger zone" section) and "Sessions" ("This session", "Other sessions", "Export & import keys"). One noun: *session*. Description: "Verify this session so other devices trust it and it can read your encrypted history." Replace the static Badge with description text. "Session names are visible to other users."
- **Effort:** M · **Risk:** medium (search index, e2e `device-verification-prompt.spec.ts`) · **iOS:** yes, this is the alignment target.

### SET-6 · P1 · Notifications page structure and controls
- **Where:** `features/settings/notifications/Notifications.tsx:126` (wrapper `gap="700"` breaks the page rhythm), `:127-203` section "All Messages" with rows "1-to-1 Chats", "1-to-1 Chats (Encrypted)", "Rooms", "Rooms (Encrypted)"; `:207-257` Mentions rows use On/Off *buttons* (`aria-pressed`) instead of `Switch`; `:259-268` "Keyword Messages"/"Select Keyword"; `SystemNotification.tsx:160` "Play sound when new message arrive."; `:127-144` "Notification Permission" tile renders its button as a child, not `after`; `AgentNotifications.tsx:206-232` explicit "Save to account"/"Refresh"/"Unsaved changes." model while every other setting autosaves.
- **iOS:** sections "1-to-1 Chats", "Rooms", "Mentions" (toggles), "Keywords" (`SettingsView.swift:1262-1310`).
- **Proposal:** sections "Direct messages" and "Rooms" each with two rows (Unencrypted / Encrypted) or one row + an "Encrypted" sub-row; "Mentions" with `Switch` controls; "Keywords" (input + removable chips in one card); fix copy ("Play a sound when a message arrives."). Agent notifications: autosave each change with the existing `role=status` line; drop Refresh (already refreshed on focus/subscription :54-75).
- **Effort:** M · **Risk:** low · **iOS:** yes — adopt iOS section names.

### SET-7 · P1 · Form fields use placeholders as labels
- **Where:** `features/settings/account/Password.tsx:96-131` (three password fields, placeholder only); `general/General.tsx:625-663` (three shortcut inputs, `aria-label` only, placeholder "Off"); `account/ContactInfo.tsx:114-150`; `General.tsx:963-976` (agent widget name/URL); `notifications/Notifications.tsx:285-292`; `AgentNotifications.tsx:162-168`.
- **Today:** labels disappear as soon as the user types; the login form does it right (`pages/auth/login/PasswordLoginForm.tsx:278-280` visible `L400` labels).
- **Proposal:** visible `Text as="label" size="L400" priority="300"` above every input (as in auth); placeholders only for format hints ("@user:server"). For shortcuts: label "Show Synara", "Open Later", "Open Notifications" with "Off" as the empty-state hint.
- **Effort:** S · **Risk:** none · **iOS:** n/a (Form rows carry labels).

### SET-8 · P1 · Inline error and status text has four treatments
- **Where:** grey `priority="500"` for real errors `General.tsx:665,741`; `role="alert"` with no colour `Password.tsx:175`; plain text `ContactInfo.tsx:169`, `General.tsx:878-879`; red bold `InviteUserPrompt.tsx:363-370`, `JoinAddressPrompt.tsx:117-119`, `CreateChat.tsx:99-104`, `pages/auth/FiledError.tsx:4-12`; red regular `LeaveRoomPrompt.tsx:79`, `Verification.tsx:305`; two local components already exist: `IgnoredUserList.tsx` `InlineError` and `Notifications.tsx:67-74` `SectionError`.
- **Proposal:** one `InlineMessage` component (`tone: 'critical' | 'success' | 'info'`, icon, T200, `role=alert|status`) exported from `components/`, used by all settings forms and dialogs; never bold, never `priority=500` for errors.
- **Effort:** S · **Risk:** none · **iOS:** n/a.

### SET-9 · P1 · Verification / cross-signing / identity dialogs block Escape and click-outside
- **Where:** `features/settings/devices/Verification.tsx:186-191`; `features/cross-signing/DeviceVerificationPrompt.tsx:71-77`; `features/cross-signing/IdentityReset.tsx:117-122`; `features/verification/NativeDeviceVerification.tsx:363-368` (`escapeDeactivates: false`, `clickOutsideDeactivates: false`); `components/UIAFlowOverlay.tsx:32-46` handles Esc manually (fine).
- **Today:** each dialog has a visible X that cancels, but Esc is dead even when idle; copy: "Setup Device Verification" (:259), "Okay" (`NativeDeviceVerification.tsx:440`) vs "Done"/"Close" elsewhere, "They Match"/"Do Not Match" stacked full-width (:230-246), start button is `variant="Warning"` (:580) so the primary action is amber; `NativeStartVerification` right-aligns itself (`alignItems="End"` :579) inside cards that are otherwise left-aligned.
- **Proposal:** Esc behaves like the X (cancel/dismiss) except while `working`; "Set up device verification"; SAS buttons side by side `[They don't match] [They match]` with match as Primary; "Done"; start verification as Primary; keep alignment to the card.
- **Effort:** S–M · **Risk:** low (cancel path already exists) · **iOS:** yes — shared copy constants.

### SET-10 · P1 · Approvals page: duplicate headings and bespoke controls
- **Where:** `features/approvals/Approvals.tsx:355-356` (PageHeader "Approvals") + `:94-99` (in-content h1 "Agent approvals" + intro); `:102-130` bespoke `<button className={css.Filter}>` and `<input type="search" className={css.Search}>` (`Approvals.css.ts:23-55`) rather than folds `Input`/the settings quiet-button pattern; `:222-258` card header stacks sender name, raw MXID and room on three lines, then `AgentApprovalCard` adds its own H5 title and a "Critical" chip (`components/agent-approval/AgentApprovalCard.tsx:179-187`) — two headers per card; `:304-308` "!approve session" hint on every pending card; `:98,204,316` "Hermes" hardcoded.
- **Proposal:** one page title (header "Approvals", drop the inner h1; keep the intro as `PageIntro`-style text); folds `Input` with search icon; filters as the Thread-display quiet segmented buttons; card = one header row (avatar · sender · room · time-left pill right) + command block + footer actions; the red card edge already carries risk so drop the "Critical" chip or move it into the time-left pill; put the session-approve hint behind a "More ways to approve" disclosure; say "agent" not "Hermes" (room/sender already name the agent).
- **Effort:** M · **Risk:** low (fixture `e2e/approvals-harness/main.tsx` renders `ApprovalsView`, so visual QA is cheap) · **iOS:** yes — `ApprovalsView.swift:33-34,43` uses the same filter labels and "Open message"; keep those.

### SET-11 · P1 · Approval actions: semantics and order
- **Where:** `components/agent-approval/AgentApprovalCard.tsx:54-76,262-291` (Approve once Solid · Approve always Primary Soft · Deny Critical Soft); `:210-259` confirm-always box styled with `Critical.Container` + `Critical.Main` border for a *positive* action.
- **Proposal:** order `[Deny — quiet Critical outline] … spacer … [Approve always — Secondary] [Approve once — Primary Solid]`; confirm-always box in Warning tone ("This trusts the pattern permanently."), confirm button Warning/Primary not Critical; keep the focus handling (:107-111) which is good.
- **Effort:** S · **Risk:** low · **iOS:** yes — approval card on iOS should mirror order and tone.

### SET-12 · P1 · Update prompt and Updates tile
- **Where:** `features/desktop-updater/DesktopUpdaterProvider.tsx:144-213` (no header/X, `maxWidth: 420`, right-aligned buttons — unlike every other dialog), `:178,188,436` ("Install and Restart", "Open Release Page", "Check for Updates"), `:101-102,436` three-dot "Preparing update..." vs ellipsis "Saving…" (`General.tsx:679`), `:83-98` tile description "Current version: v2.7.6." + "Last checked <full toLocaleString>".
- **Proposal:** use the SET-1 dialog shell (title "Synara 2.7.7 is available", release notes body, `[Later] [Install and restart]`); sentence case; `…`; tile description "Version 2.7.6 · Checked 5 min ago"; move the Updates section to the top of General (or to About beside the version — iOS keeps version on About).
- **Effort:** S · **Risk:** none · **iOS:** n/a.

### SET-13 · P1 · Diagnostics uses a native `window.confirm` and an ad-hoc section
- **Where:** `features/settings/diagnostics/Diagnostics.tsx:158` (`window.confirm('Clear all diagnostic records…')`); `:323-343` "Privacy" rendered with `Header`/`Text L400` instead of `SettingsSection`.
- **Proposal:** SET-1 `ConfirmDialog` ("Clear diagnostic records?" · "Reports on this device are deleted. Nothing is sent." · `[Cancel] [Clear]`); `SettingsSection title="Privacy"`.
- **Effort:** S · **Risk:** none · **iOS:** n/a.

### SET-14 · P2 · Dialog chrome varies dialog to dialog
- **Where:** headers with `variant="Surface"` + bottom border (`LogoutDialog.tsx:43-50`, `LeaveRoomPrompt.tsx:60-67`, `Verification.tsx:250-257`) vs no border (`InviteUserPrompt.tsx:254-257`, `JoinAddressPrompt.tsx:75-81`); header padding `padding: S200, paddingLeft: S400` (`CreateRoomModal.tsx:55-58`, `AddExisting.tsx:210-213`) vs `0 S200 0 S400`; body padding `S400` vs `paddingTop: 0` (`JoinAddressPrompt.tsx:92`); close buttons sometimes `SettingsQuietControl` (`Profile.tsx:283-291`) sometimes default.
- **Proposal:** `DialogHeader` / `DialogBody` / `DialogFooter` primitives next to `components/modal-dialog`, used by all 15 dialogs. Falls out of SET-1.
- **Effort:** S (with SET-1) · **Risk:** none · **iOS:** n/a.

### SET-15 · P2 · Enter does not confirm in several dialogs
- **Where:** `LeaveRoomPrompt.tsx:87-104` and `LeaveSpacePrompt.tsx:87-104` (`type="submit"` without a form), `LogoutDialog.tsx:86-96`, `Profile.tsx:296-305` (Remove avatar), `RoomUpgrade.tsx:147-154`; `initialFocus: false` means focus sits on the dialog surface.
- **Proposal:** wrap confirm dialogs in `<form onSubmit>` so Enter fires the primary; destructive dialogs take initial focus on Cancel. Covered by SET-1 but called out because it's a keyboard-first regression today.
- **Effort:** S · **Risk:** none · **iOS:** n/a.

### SET-16 · P2 · Join-by-address prompt copy
- **Where:** `components/join-address-prompt/JoinAddressPrompt.tsx:83,97-104,118,123`.
- **Today:** "Join with Address"; "Enter public address to join the community. Addresses looks like:"; error "**Invalid Address**"; button "Open".
- **Proposal:** "Join a room by address" · "Enter a room address or matrix.to link." · examples as muted monospace · error "That doesn't look like a room address or matrix.to link." · button "Join".
- **Effort:** S · **Risk:** none · **iOS:** check join-by-address sheet copy.

### SET-17 · P2 · Create room / space / chat / add-existing forms
- **Where:** `features/create-room/CreateRoom.tsx:217,239,266-267,300-301,323-324`; `features/create-space/CreateSpace.tsx:164,212-213,228-229`; `features/create-chat/CreateChat.tsx:102`; titles `CreateRoomModal.tsx:62` ("New Chat Room"/"New Voice Room"), `CreateSpaceModal.tsx:61` ("New Space"), `AddExisting.tsx:216` ("Add Existing"), `AddExisting.tsx:254-259` ("No Match Found", "You do not have any Rooms to display yet.").
- **Today:** "Topic (Optional)", chip "Advanced Options", "Knock to Join — Anyone can send request to join this room.", "Allow Federation — Users from other servers can join.", "End-to-End Encryption — Once this feature is enabled, it can't be disabled after the room is created.", "Please enter a valid User ID."
- **Proposal:** titles "New room" / "New voice room" / "New space" / "Add existing rooms"; "Topic (optional)"; "Advanced options"; "Let people knock to join — Anyone can ask to join; members approve."; "Allow other servers — People on other homeservers can join (federation)."; "End-to-end encryption — Can't be turned off later."; "Enter a full user ID like @name:server."; empty state "No rooms match "x"" / "You haven't joined any rooms yet."
- **Effort:** S · **Risk:** none · **iOS:** yes if iOS has a create-room sheet; mirror labels.

### SET-18 · P2 · Floating selection bars use saturated Success/Critical containers
- **Where:** `features/add-existing/AddExisting.tsx:334-389` (`Menu variant="Success"`, bold "Apply when ready. (3 Selected)", buttons "Reset"/"Apply Changes"); `features/common-settings/emojis-stickers/RoomPacks.tsx:328-361` (`Menu variant="Critical"`); `features/settings/devices/OtherDevices.tsx:243-324` (`Menu variant="Critical"`, bold "Log out of selected devices. (2 selected)", inline password field).
- **Proposal:** one `SelectionBar` on a neutral Surface with `raisedShadow`: "3 selected" left, `[Cancel] [Add to space]` right; destructive variants colour only the primary button. Sentence case ("3 selected", "Log out 2 sessions").
- **Effort:** S–M · **Risk:** low · **iOS:** n/a.

### SET-19 · P2 · Account page structure
- **Where:** `features/settings/account/Account.tsx:14-20` (Profile, Matrix ID, Password, Contact Information, Blocked Users); `MatrixId.tsx:14-31` (a whole section for one copy row); `IgnoredUserList.tsx:132-135` (header is a bare `Text L400`, not `SettingsSection`), `:143-144` row titled "Select User"; `ContactInfo.tsx:86,93` ("Contact Information" / "Email Address — Email address attached to your account.").
- **iOS:** Account sections Profile (with description), Avatar, Contact, Blocked Users ("Blocked users cannot message or invite you."), Sessions (`SettingsView.swift:353-439`).
- **Proposal:** fold Matrix ID into Profile as a read-only row with Copy; `SettingsSection title="Blocked users" description="Blocked users can't message or invite you."` with row "Block a user" (input + Add) and chips below; "Contact" / "Email".
- **Effort:** S · **Risk:** none · **iOS:** yes (adopt iOS copy).

### SET-20 · P2 · Change-password form repeats itself and hides state
- **Where:** `features/settings/account/Password.tsx:87-94,143-155,168-178`.
- **Today:** section "Password" → row "Change Password — Confirm with your current password." → button "Change Password"; success "Password changed." unstyled; errors unstyled (see SET-8); "Open Account Page" for OIDC.
- **Proposal:** section "Password"; labelled fields; button "Update password"; `InlineMessage success`; OIDC row "Your homeserver manages your password." with "Open account page".
- **Effort:** S · **Risk:** none · **iOS:** n/a (iOS has no password change?) — verify.

### SET-21 · P2 · Room/Space Danger Zone content
- **Where:** `features/room-settings/general/General.tsx:44-50`, `features/space-settings/general/General.tsx:39-45` (good `tone="critical"`), `features/common-settings/general/RoomUpgrade.tsx:217-258` (one row with up to three buttons "Old Room"/"Open New Room"/"Upgrade"), `:115-117,148-154` (bold red "This action is irreversible!", Secondary button performs the upgrade).
- **Proposal:** add "Leave room" as the first Danger Zone row (Slack keeps Leave at the bottom of channel details) wired to `LeaveRoomPrompt`; upgrade dialog via SET-1 (Critical primary, Cancel, consequence sentence instead of exclamation); split "Old room"/"Open new room" into a muted status row.
- **Effort:** S · **Risk:** low · **iOS:** yes — iOS room settings should have the same Leave placement.

### SET-22 · P2 · Members page search chip colour noise
- **Where:** `features/common-settings/members/Members.tsx:170-171,190-204` (title "12 Members", result chip `variant Success|Critical` "3 Results"/"No Results" inside the input).
- **Proposal:** "Members" title with count in the description ("12 people…"); result count as muted text, clear button quiet.
- **Effort:** S · **Risk:** none · **iOS:** n/a.

### SET-23 · P2 · Token discipline: inline paddings and hardcoded colours
- **Where:** `General.tsx:624,740,811` (`padding: 12`), `IdentityReset.tsx:156` (`paddingLeft: 20`), `Appearance.tsx:361` (`rgba(0,0,0,0.18)`), `AgentApprovalCard.tsx:78-91` inline style block (tokens, but inline), `Approvals.css.ts:50` search uses `Background.Container` inside a Surface page.
- **Proposal:** `config.space.*` only; move inline styles to `.css.ts`; swatch outline `color.Surface.ContainerLine` (dark parity).
- **Effort:** S · **Risk:** none · **iOS:** n/a.

### SET-24 · P2 · Developer Tools pages
- **Where:** `features/settings/developer-tools/DevelopTools.tsx:124-127` ("Enable Developer Tools" with no description), `:147-151` dead Widgets pointer, `:168-180` "Account Data → Global — Account-data browsing is not available in this native session." (a row that only says it doesn't work); `features/common-settings/developer-tools/DevelopTools.tsx:95-96` ("Room ID — Copy room ID to clipboard. ("!abc:server")").
- **Proposal:** description "Show event sources, state editors and raw account data."; hide unavailable rows; Room ID row shows the ID as monospace value with Copy, no parentheses.
- **Effort:** S · **Risk:** none · **iOS:** n/a.

### SET-25 · P2 · Error pages and config error
- **Where:** `components/app-error/AppErrorFallback.tsx:52-54` (raw `error.message` in Critical red under "Something went wrong"); `pages/ConfigConfig.tsx:1,10,28-47` (raw folds `Dialog`, "Heating up", "Failed to load client configuration file.", both Retry and Continue `variant="Critical"`).
- **Proposal:** friendly body ("Synara hit a problem loading this screen."), technical message behind a "Show details" disclosure in monospace; `[Try again — Primary] [Go home — Secondary]` already right. Config error: use `components/modal-dialog`, Primary Retry + quiet "Continue anyway", loading copy "Loading…".
- **Effort:** S · **Risk:** none · **iOS:** n/a.

### SET-26 · P2 · Auth copy and heading consistency
- **Where:** `pages/auth/login/PasswordLoginForm.tsx:296-302,324-337` ("Invalid Username or Password.", "Failed to login. Your login request has been rate-limited by server, Please try after some time.", "Failed to find your Matrix ID server."); `pages/auth/register/Register.tsx:46,57` (H2 "Register" in loading/error states vs H3 "Create your account" :77), `:88,93` ("You have been rate-limited! Please try after some time.", "Invalid Request! Failed to get any registration options."); `pages/auth/register/PasswordRegisterForm.tsx:481-493`; `pages/auth/ServerPicker.tsx:118` ("Homeserver List"); `pages/auth/login/Login.tsx:52`; file `pages/auth/FiledError.tsx` (typo of FieldError).
- **Proposal:** calm sentence-case errors ("That username or password isn't right.", "Too many attempts. Wait a few minutes and try again.", "Couldn't find the server for that Matrix ID."); same heading in all three Register states; "Homeservers"; rename file to `FieldError.tsx`. The layout itself (`AuthLayout.tsx`, `styles.css.ts`) is already in the target direction.
- **Effort:** S · **Risk:** none · **iOS:** yes — login error copy should match.

### SET-27 · P2 · UIA stage dialogs bypass the modal-dialog wrapper and misuse Critical
- **Where:** `components/uia-stages/TermsStage.tsx:2,18-33,61` (raw folds `Dialog`, title is the raw `errorCode`, Retry and Cancel both Critical); `EmailStage.tsx:2,41-78,146` (Cancel `variant="Critical"` outlined, "Verification Request Sent", "Send Verification Email"); `RegistrationTokenStage.tsx:52-61`.
- **Proposal:** `components/modal-dialog` `Dialog` (gets `aria-modal`, inert, naming); human titles ("Couldn't accept the terms"); Primary action + quiet Secondary Cancel; sentence case.
- **Effort:** S · **Risk:** none · **iOS:** n/a.

### SET-28 · P2 · Advisory InfoCards use the Critical variant and generic titles
- **Where:** `components/LogoutDialog.tsx:59-70` (`InfoCard variant="Critical" title="Alert"` / "Unverified Device"); `features/settings/devices/Verification.tsx:85-88` ("Unverified" Critical), `:118-121` (Warning for the same state on other devices); `Devices.tsx:109-112` (Critical for a load failure — fine).
- **Proposal:** Warning for advisories ("Back up your keys before logging out"), Critical only for failures/destructive outcomes; titles say what to do.
- **Effort:** S · **Risk:** none · **iOS:** yes for logout warning copy.

### SET-29 · P2 · Settings nav: single-item groups and no last-page memory
- **Where:** `features/room-settings/RoomSettings.tsx:105-106` ("Content" group with only "Custom Emoji", "Advanced" with only Developer Tools); `features/space-settings/SpaceSettings.tsx` same shape; `features/settings/Settings.tsx:153-156` always opens General.
- **Proposal:** flatten single-item groups into the first group; remember the last open page per settings surface for the session (Slack reopens where you left).
- **Effort:** S · **Risk:** none · **iOS:** n/a.

### SET-30 · P2 · Terminology drift
- **Where:** device vs session (`DeviceTile.tsx:211,215`, `OtherDevices.tsx:263`, iOS "Sessions"); "Log Out" (`Settings.tsx:217`, `LogoutDialog.tsx:52,95`) vs "Sign in" (`PasswordLoginForm.tsx:385`) vs "Sign out of my other devices" (`Password.tsx:133`); "Blocked Users" UI vs `IgnoredUserList`/"Unblock"; "1-to-1 Chats" (`Notifications.tsx:135`) vs direct messages; "Hermes" (`Approvals.tsx:98,204,316`) vs agent; "Okay"/"Done"/"Close".
- **Proposal:** glossary in `docs/ux-audit/`: session; Log in / Log out (matches iOS "Log Out"); Blocked users; Direct messages; agent; Done for success, Cancel/Close otherwise. Apply in the SET-2 pass.
- **Effort:** S · **Risk:** none · **iOS:** yes.

## Already good — leave alone

- `components/settings-layout/SettingsLayout.tsx`: grouped nav with uppercase group labels, search across rows with `revealSetting` scroll-and-highlight (`:113-134`), arrow/Home/End nav (`:137-154`), Esc clears search before closing (`:210-216`); `SettingsPage` centred 720px column with intro; `SettingsSection` with `tone="critical"` and hairline-divided rows (`style.css.ts:168-215`).
- `components/setting-tile/SettingTile.tsx:24-34` names folds `Switch` controls after the row title; `SettingTile.css.ts` wraps the control below copy only at ≤720px.
- `components/modal-dialog/ModalDialog.tsx`: `role`, `aria-modal`, title discovery, focus on open, app made inert.
- Room/Space "Danger Zone" with description "Changes here cannot be undone." (`room-settings/general/General.tsx:44-48`).
- Appearance page intro "These settings apply to this device only." (`Appearance.tsx:846`); Thread display and Message text as quiet segmented buttons with check glyphs.
- Approvals: live-region announcement after a decision and focus return to the heading (`Approvals.tsx:85-89`), layered empty/loading/partial states (`:147-207`); `AgentApprovalCard` confirm-always focus management (`:107-111`).
- Login: visible labels, fixed-height server status line, `alertdialog` with typed confirmation for store recovery (`PasswordLoginForm.tsx:403-478`), brand column and card (`pages/auth/styles.css.ts`).
- Identity reset: honest bullet warnings, `alertdialog` on the warning step, recovery-may-be-off follow-up (`IdentityReset.tsx:105-112,124`).
- Logout dialog last-device warning (`LogoutDialog.tsx:72-78`) and retry copy.
- Notifications: per-section write errors that never look saved (`Notifications.tsx:107-117`); Ignored users: load failure never rendered as an empty list (`IgnoredUserList.tsx:104-106`).
- Diagnostics privacy copy (`Diagnostics.tsx:333-341`); `PasswordInput` show/hide with labels.
- Cmd+, opens Settings (`features/desktop-keyboard/appShortcuts.ts:60`).

## Prior-audit items that persist

From `docs/ux-audit/desktop-cross-client-audit.md` §9 (P2 settings):
- "Card soup": **mostly fixed** — `SectionCardRows` flattens per-row `SequenceCard`s onto one section card (`settings-layout/style.css.ts:187-215`), but every row is still authored as its own `SequenceCard` (e.g. `General.tsx:1117-1238`, ten cards for ten switches), so any row rendered outside a `SettingsSection` regresses to a floating card (SET-3).
- "Responsive setting row": **done** (`SettingTile.css.ts`).
- "Destructive actions in a clearly separated final section": **done** for Room/Space (Danger Zone), **not** for app settings — Log out is a red nav-footer button (`Settings.tsx:207-218`), Reset identity sits mid-card in Security (`Devices.tsx:141-147`), Reload application sits in Storage (`General.tsx:756-770`) (SET-5).
- "Keep diagnostics/developer tools visibly advanced": Diagnostics page **exists**, but General still hosts Desktop Integration diagnostics and the Native Session Store tile (SET-3).
- "Shorten technical appearance copy and preview the effect beside the control": **persists** — Base colour description is three sentences and the controls are in the slot (SET-4).
- "Message Layout read-only or hidden on native": **done** (`Appearance.tsx:814`).

From `docs/ux-audit/grok-4.6-high-review.md`:
- P1-5 "macOS Settings idiom: Command-, expected pane navigation, last-pane restoration": Cmd+, **done**; last-pane restoration **not** (`Settings.tsx:153-156`) (SET-29).
- "Confirmations use platform roles": **partial** — `alertdialog` on identity reset, redact confirm and store recovery only (SET-1).
- Agent approval "actions explicit; state not color-only": **done** (status text + labels).
- Settings row acceptance "labels, descriptions, controls, warning text… one predictable alignment grid": **persists** for Appearance colour rows and the bare General cards (SET-3, SET-4).
