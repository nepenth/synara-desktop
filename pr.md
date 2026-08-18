## V-CALL deep cut — fully zero-Matrix-JS tree

> Historical pull-request description retained as migration evidence. It is not
> current branch, merge, or release guidance.

Operator-authorized (option B). Removes the embedded **(V-)CALL** feature and the last
remaining Matrix JS dependency.

### Removed
- **`plugins/call`**, **`features/call`**, **`features/call-status`** (CallEmbedProvider,
  useCallEmbed/useCall/useCallSpeakers, callEmbed/callPreferences/callChat atoms,
  pages/CallStatusRenderer, RoomNavItem call buttons/session/members, Room CallView/chat view)
- **`matrix-widget-api`** from `dependencies` + lockfile
- **native `matrix/widgets` module** (registry, product_commands, error) + `dto/widget.rs` + `WidgetId`
- **call-widget-media live Synapse proof** + its CI job + aggregate wiring
  (native proof family is now **6**: reactions, attachments, polls, rich-messages, threads, receipts)
- modernisation-test registrations for the deleted call tests

### Preserved — general media (not call)
- `matrix_media_config` / `matrix_media_download` native owners relocated into the **media module**;
  `matrix_call_media_config` renamed -> `matrix_media_config` (capability ACL rename included).
  These back `getMediaConfig`/`downloadMedia` used by ServerConfigsLoader, MediaConfigLoader,
  ImageViewer, and matrix/media.

### Strengthened
- `check-v-burn-complete.mjs` **now 20 checks**: asserts `matrix-widget-api` absent from
  deps/devDeps/lockfile **and** no import/require coupling — the tree is fully zero-Matrix-JS.
- Docs (SCOREBOARD / PROGRESS / FACADE-contract / v-burn taxonomy) record the removal and keep
  the V-BURN reached markers.

### Verification
- `cargo test --locked`: **827 passed / 0 failed**
- `cargo check` + `cargo fmt --check`: clean
- `tsc --noEmit`, `eslint`, `prettier --check`: clean
- `npm run check:matrix-rust-guardrails` (P1.6 + SDK guardrails + program-status + governance + V-BURN 20/20): **green**

Does **not** merge to `main`; bridge (#39) remains operator-gated as before.
