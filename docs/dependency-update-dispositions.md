# Dependency update dispositions and renderer retirement

This records dependency work incorporated into the Rust consolidation branch.
GitHub's open PR list was verified on 2026-10-01 UTC (2026-09-30 local time).
The four open PRs were [#1159](https://github.com/nepenth/synara-desktop/pull/1159),
[#1161](https://github.com/nepenth/synara-desktop/pull/1161),
[#1162](https://github.com/nepenth/synara-desktop/pull/1162), and
[#1163](https://github.com/nepenth/synara-desktop/pull/1163), all dependency updates.
They remain open; incorporation into this branch does not mean GitHub merge or
closure. Once the consolidated branch passes its remaining gates and lands,
these original PRs can be closed as superseded with a link to the consolidated
change.

## Rust PR outcomes

The root Cargo workspace and lockfile own desktop, Core, NSE, and bindgen.
Desktop's independent lockfile is retired. Builds use selected package
production features and `--locked`; unified workspace test features do not
prove a shipping Apple or NSE graph.

| PR | Crate | Previous | PR proposal and resolved branch version | Manifest policy and outcome |
| --- | --- | --- | --- | --- |
| #1159 | `tauri` | 2.11.5 | 2.11.6 | Integrated; desktop requires `~2.11.6`. npm API/CLI use the matching 2.11 family. |
| #1159 | `tauri-plugin-updater` | 2.11.0 | 2.12.0 | Integrated; desktop requires `~2.12.0`, matching npm updater 2.12.0. |
| #1159 | `tauri-plugin-single-instance` | 2.4.4 | 2.4.5 | Integrated; compatible major requirement with a 2.4.5 floor. |
| #1159 | `zbus` | 5.16.0 | 5.19.0 | Integrated; compatible major requirement with a 5.19.0 floor. |
| #1162 | `uniffi` | 0.28.3 | 0.32.2 | Integrated; exact `=0.32.2` shared by Core/NSE runtime, build dependencies, and project bindgen. |

Matrix SDK and its sibling crates remain exactly 0.19.1. Generated async UniFFI
exports retain Tokio bridges through the shared signature-aware helper.
Generator fixtures and generated Swift typechecks passed; regenerating and
linking shipping archives is a separate pending gate. See the
[Rust dependency security dispositions](rust-dependency-security.md) for version
policy, patched XML/RNG entries, and production feature-graph checks.

## npm PR #1161: all three root updates

| Package | Previous | PR proposal | Branch outcome | Disposition |
| --- | --- | --- | --- | --- |
| `@tauri-apps/api` | `2.11.0` | `2.11.1` | `2.11.1` | integrated |
| `@tauri-apps/plugin-updater` | `2.11.0` | `2.12.0` | `2.12.0` | integrated |
| `@tauri-apps/cli` | `2.11.2` | `2.11.5` | `2.11.5` | integrated |

## npm PR #1163: all 69 renderer/tooling updates

“Integrated” means the proposal is in the manifest and lockfile. Compatible
replacements and retired packages are deliberate outcomes explained below.

| Package | Previous | PR proposal | Branch outcome | Disposition |
| --- | --- | --- | --- | --- |
| `@atlaskit/pragmatic-drag-and-drop` | `1.1.6` | `4.0.0` | `4.0.0` | integrated |
| `@atlaskit/pragmatic-drag-and-drop-auto-scroll` | `1.3.0` | `3.2.1` | `3.2.1` | integrated |
| `@atlaskit/pragmatic-drag-and-drop-hitbox` | `1.0.3` | `3.0.0` | `3.0.0` | integrated |
| `@fontsource/inter` | `4.5.14` | `5.3.0` | `@fontsource-variable/inter@5.3.0` | compatible replacement |
| `@tanstack/react-query` | `5.24.1` | `5.104.0` | `5.104.0` | integrated |
| `@tanstack/react-query-devtools` | `5.24.1` | `5.104.0` | `5.104.0` | integrated |
| `@tanstack/react-virtual` | `3.14.7` | `3.14.13` | `3.14.13` | integrated |
| `@tauri-apps/api` | `2.11.0` | `2.11.1` | `2.11.1` | integrated |
| `@tauri-apps/plugin-updater` | `2.11.0` | `2.12.0` | `2.12.0` | integrated |
| `@vanilla-extract/css` | `1.20.1` | `1.21.2` | `1.21.2` | integrated |
| `blurhash` | `2.0.4` | `2.0.5` | `2.0.5` | integrated |
| `chroma-js` | `3.1.2` | `3.2.0` | `3.2.0` | integrated |
| `classnames` | `2.3.2` | `2.5.1` | `2.5.1` | integrated |
| `dayjs` | `1.11.10` | `1.11.23` | `1.11.23` | integrated |
| `domhandler` | `5.0.3` | `6.0.1` | `6.0.1` | integrated |
| `emojibase` | `15.3.1` | `17.0.0` | `17.0.0` | integrated |
| `emojibase-data` | `15.3.2` | `17.0.0` | `17.0.0` | integrated |
| `focus-trap-react` | `10.0.2` | `12.0.3` | `12.0.3` | integrated |
| `folds` | `2.6.2` | `2.7.2` | `2.7.2` | integrated |
| `html-dom-parser` | `4.0.0` | `8.0.2` | `8.0.2` | integrated |
| `html-react-parser` | `4.2.0` | `6.1.8` | `6.1.8` | integrated |
| `i18next` | `23.12.2` | `26.4.2` | `26.4.2` | integrated |
| `i18next-browser-languagedetector` | `8.0.0` | `8.2.1` | `8.2.1` | integrated |
| `i18next-http-backend` | `4.0.0` | `4.0.2` | `4.0.2` | integrated |
| `immer` | `9.0.16` | `11.1.18` | `11.1.18` | integrated |
| `jotai` | `2.6.0` | `3.0.0` | `3.0.0` | integrated |
| `linkify-react` | `4.3.2` | `4.3.3` | `4.3.3` | integrated |
| `linkifyjs` | `4.3.2` | `4.3.3` | `4.3.3` | integrated |
| `pdfjs-dist` | `6.2.108` | `6.3.289` | `6.3.289` | integrated |
| `react` | `19.2.6` | `19.3.0` | `19.3.0` | integrated |
| `react-aria` | `3.29.1` | `3.52.1` | `3.52.1` | integrated |
| `react-blurhash` | `0.2.0` | `0.3.0` | `0.3.0` | integrated |
| `react-colorful` | `5.6.1` | `5.8.1` | `5.8.1` | integrated |
| `react-dom` | `19.2.6` | `19.3.0` | `19.3.0` | integrated |
| `react-error-boundary` | `4.0.13` | `6.1.6` | `6.1.6` | integrated |
| `react-google-recaptcha` | `2.1.0` | `3.1.0` | `3.1.0` | integrated |
| `react-i18next` | `15.0.0` | `17.0.15` | `17.0.15` | integrated |
| `react-range` | `1.8.14` | `1.10.0` | `1.10.0` | integrated |
| `react-router-dom` | `7.18.2` | `7.18.4` | `7.18.4` | integrated |
| `slate` | `0.123.0` | `0.126.2` | `0.126.2` | integrated |
| `slate-dom` | `0.123.0` | `0.126.0` | `0.126.0` | integrated |
| `slate-react` | `0.123.0` | `0.127.1` | `0.127.1` | integrated |
| `ua-parser-js` | `1.0.35` | `2.0.10` | `2.0.10` | integrated |
| `@babel/core` | `7.29.7` | `8.0.6` | Removed | retired |
| `@element-hq/element-call-embedded` | `0.22.0` | `0.26.0` | Removed | retired |
| `@eslint/js` | `9.39.4` | `10.0.1` | `9.39.5` | compatible replacement |
| `@playwright/test` | `1.61.1` | `1.63.0` | `1.63.0` | integrated |
| `@rollup/plugin-inject` | `5.0.3` | `5.0.5` | Removed | retired |
| `@types/chroma-js` | `3.1.1` | `3.1.2` | `3.1.2` | integrated |
| `@types/file-saver` | `2.0.5` | `2.0.7` | `2.0.7` | integrated |
| `@types/node` | `24.13.3` | `26.6.3` | `24.13.3` | compatible replacement |
| `@types/prismjs` | `1.26.0` | `1.26.6` | `1.26.6` | integrated |
| `@types/react` | `19.2.14` | `19.3.0` | `19.3.0` | integrated |
| `@types/react-dom` | `19.2.3` | `19.3.0` | `19.3.0` | integrated |
| `@types/react-google-recaptcha` | `2.1.8` | `2.1.9` | `2.1.9` | integrated |
| `@types/sanitize-html` | `2.9.0` | `2.16.1` | `2.16.1` | integrated |
| `@types/ua-parser-js` | `0.7.36` | `0.7.39` | Removed | retired |
| `@typescript-eslint/eslint-plugin` | `8.59.4` | `8.70.1` | `8.70.1` | integrated |
| `@typescript-eslint/parser` | `8.59.4` | `8.70.1` | `8.70.1` | integrated |
| `@vanilla-extract/vite-plugin` | `5.2.5` | `5.2.6` | `5.2.6` | integrated |
| `@vitejs/plugin-react` | `5.2.0` | `6.1.1` | `6.1.1` | integrated |
| `eslint` | `9.39.4` | `10.11.0` | `9.39.5` | compatible replacement |
| `globals` | `16.5.0` | `17.12.0` | `17.12.0` | integrated |
| `lint-staged` | `16.3.2` | `17.6.0` | `17.6.0` | integrated |
| `prettier` | `2.8.1` | `3.9.9` | `2.8.8` | compatible replacement |
| `typescript` | `5.7.3` | `7.0.2` | `npm:@typescript/typescript6@6.0.2` | compatible replacement |
| `vite` | `7.3.6` | `8.3.1` | `8.3.1` | integrated |
| `vite-plugin-static-copy` | `3.4.0` | `4.1.1` | `4.1.1` | integrated |
| `vite-plugin-top-level-await` | `1.4.4` | `1.6.0` | Removed | retired |

## Compatibility choices and follow-up conditions

- **TypeScript:** native TypeScript 7.0.2 is installed as
  `@typescript/native: npm:typescript@7.0.2`, and `tsc` runs the native compiler.
  The official `typescript: npm:@typescript/typescript6@6.0.2` alias supports
  compiler-API inventory tooling and ESLint. This follows Microsoft's
  [side-by-side migration guidance](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/).
  The compatibility package exports `tsc6`, while the native package exports
  `tsc`; installed bin links target those respective packages without a name
  collision. Retire the API alias once its callers support the native compiler API.
- **ESLint:** `eslint` and `@eslint/js` use 9.39.5 because installed React/import
  plugins declare support through ESLint 9. Upgrade to 10 when peers support it;
  clean installs must keep working without force or legacy peer flags.
- **Node types:** 24.13.3 matches pinned Node 24.13.1 rather than promising Node
  26 APIs. Both nonpublished application/tooling packages are private and no
  longer advertise the nonexistent `main: index.js`.
- **Prettier:** 2.8.8 preserves existing formatting with explicit ES5 commas.
  Proposed 3.9.9 caused 692 unrelated rewrites, or 115 even with ES5 commas.
  A separate formatter migration should establish a new policy. Twenty existing
  differences were corrected so the real full lint gate passes.
- **Fonts:** Fontsource 5 moved variable fonts to
  `@fontsource-variable/inter@5.3.0`; imports preserve variable-font rendering.
- **Jotai:** Jotai 3 removed `atomFamily` from its utilities. Maintained
  `jotai-family@1.1.0` serves upload and room-draft atom families.
- **HTML parser:** structural `isTag`/`isText` checks replace instance comparisons
  because parser packages can instantiate different exported classes. Child
  containers are flattened for React conversion. The actual production parser
  browser regression covers empty `pre`, empty code, and populated code blocks.
- **Other adaptations:** Immer uses named `produce`; search results use explicit
  result types; the millify adapter uses its named export under Vite 8; UA-parser
  2's `macOS` name preserves shortcuts and device labels.
- **Slate:** the proposed `slate` 0.126.2, `slate-dom` 0.126.0, and `slate-react`
  0.127.1 satisfy their declared peers (`slate >=0.121.0` and
  `slate-dom >=0.119.1`); retained `slate-history` 0.113.1 accepts
  `slate >=0.65.3`. Versions need not have identical minor numbers. A browser
  regression uses the production `RoomComposer` and `useEditor`, including
  `withReact`/`withHistory`, to insert text, apply bold, split paragraphs,
  undo, and redo. Its native user agent remains aligned with browser platform
  information so macOS/Linux keyboard mappings are coherent.

## Retired infrastructure and retained renderer roles

Rust owns Matrix authentication, encrypted stores, synchronization, recovery,
and notification decisions. React/TypeScript still owns views, routing, input,
presentation, accessibility, PDF display, and the typed native bridge.
JavaScript repository/build/test scripts remain Node tooling. This consolidation
does not require migrating UI presentation code into Rust.

The duplicate JavaScript `useForceUpdate` is removed and `colorMXID` is typed.
Product tsconfig has `allowJs: false` and no JavaScript source files; TypeScript
and Vite resolve the same surviving TS hook. Its browser case verifies numeric
updates, and fixed Matrix-ID palette samples preserve prior color behavior.

Legacy application IndexedDB detection/deletion, obsolete migration identity
tracking, and related tests are retired. Logout can dispose of narrow old
local-storage bootstrap keys without inspecting or deleting IndexedDB.
Renderer cleanup/reload follows the exact native `{ status: 'logged_out' }`
acknowledgment; native failure preserves renderer state for retry. Logout retains
the encrypted native store, and archival recovery is a separate native operation.
After confirmed native logout, every renderer cleanup is attempted and reload
still occurs if browser storage or a renderer listener throws; native remains
the session authority. Existing cleanup already removes all account navigation
keys by prefix. Fresh login markers are consumed only after the initialized
facade's native identity, homeserver, and canonical generation match. Missing,
unavailable, logged-out, mismatched, and failed native refreshes preserve them;
the marker writer currently has no production caller.
Former “Clear Cache” controls accurately say “Reload Application” and only
refresh renderer state. Renderer stop/cache failures cannot prevent that recovery
reload, and no native stop or wipe command is added.

PWA generation, unused Element Call and roughly 41 MB of embedded assets,
Buffer injection/polyfills, and the top-level-await plugin are retired. Startup
unregisters only known old Synara worker scripts at expected origin/scopes and
remains usable if worker APIs fail. An already controlled page may remain
controlled until navigation; this unregistering neither deletes IndexedDB nor
broadly removes other applications' workers. Vite targets ES2022 explicitly.

PDF.js remains at patched `6.3.289`; the production API and verbatim worker use
its matched upstream `legacy` distribution. A bounded `core-js` `3.50.0`
initializer supplies `Promise.withResolvers` in both independent realms and
`ArrayBuffer.transferToFixedLength` in the worker before PDF.js evaluates.
These standard builtin shims serve active PDF code. The Vite worker entry loads
the packaged upstream worker after initialization; build checks verify its
exact installed bytes and the wrapper's shim provenance.

The current [PDF.js browser support table](https://github.com/mozilla/pdf.js/wiki/Frequently-Asked-Questions#which-browsersenvironments-are-supported)
starts legacy Safari support at 18. The
[upstream compatibility report](https://github.com/mozilla/pdf.js/issues/20899)
confirms newer builtin requirements affect both page and worker realms. The
[core-js standard modules](https://github.com/zloirock/core-js#ecmascript-promise)
provide maintained implementations rather than project-written polyfills.
Retaining the patched dependency also covers the
[July 2026 PDF.js scripting advisory](https://github.com/mozilla/pdf.js/security/advisories/GHSA-hq66-cqwq-w95j),
whose upstream fix is `6.2.108`. Current Chromium and WebKit tests model missing
newer builtins before both production imports, require a real worker and red PDF
pixel, and reject fake-worker fallback. This corrects the tested builtin gap;
it does not certify every PDF feature on physical macOS 13/WebKit 16.

Unused direct `dateformat`, `vite-node`, and `@types/ua-parser-js` are removed.
The proposed Babel 8 direct update is retired: Vite React 6 uses Oxc and no
project Babel configuration or direct consumer exists. Legitimate transitive
Babel use by other plugins remains. Obsolete overrides for vite-node, Workbox,
fast-uri, uuid, brace-expansion 2, and old minimatch versions are removed.
Installed brace-expansion/js-yaml are patched; nanoid/PostCSS overrides still
serve the installed graph.

## Recorded validation and limits

Validated with pinned Node 24.13.1:

| Gate | Recorded outcome |
| --- | --- |
| Ordinary root `npm ci` and `npm --prefix synara ci` | Passed without peer bypass flags. |
| Full root/frontend `npm audit --json` | Zero findings at every severity, including development dependencies. |
| Full TypeScript and modernization typechecks | Passed with native TypeScript 7. |
| Full renderer lint | ESLint and Prettier passed. |
| Normal `npm run test:modernization`, including runtime build pretest | 1,199 passed, zero failures or skips, including the accepted notification delivery/action adapters. |
| Timeline Chromium browser command | 7 passed. |
| Native-timeline Chromium browser command | 72 passed. |
| Normal `test:browser:desktop-polish:ci` command | 6 room-list, 34 approvals, and 5 runtime-maturity Chromium cases passed. |
| Normal release `test:browser:desktop-polish` command from a cold Vite cache | 12 room-list, 68 approvals, and 10 runtime-maturity Chromium/WebKit cases passed. |
| Normal build output guard and `check:runtime-assets` | Matching installed PDF-worker bytes, config/locales at actual URLs, retired assets absent. |

Both CI `desktop-polish:ci` and release `desktop-polish` npm entrypoints include
runtime-maturity. It runs the typed update hook, dialog focus trap, production
HTML parser, Slate composer, and real PDF worker, checks a rendered pixel, and rejects fake-worker
fallback. The runtime harness uses unrestricted port 4191 for WebKit, normal
Safari keyboard navigation for its focus trap, and explicitly preoptimizes/warms
the active PDF/shim/Prism modules so cold dependency discovery cannot reload a
page during the assertions. The final release command passed from a cold Vite
cache with both actual browser engines; runtime-maturity uses their native user
agents. This local WebKit run is distinct from physical minimum-system and installed native proof.
Sanitizer/PostCSS externalization
warnings remain visible. These harnesses do not prove authenticated live Matrix
sessions or installed native behavior.

Locked Cargo metadata/version queries, NSE production feature-graph checks,
bridge generator fixtures, and generated Swift typechecks passed. Full native
workspace/desktop/Core/NSE compilation, tests and Clippy are being completed.
Shipping Apple archive/XCFramework regeneration, app/extension compile/link,
NSE symbol isolation, packaging, and installed behavior proof remain pending.
Source scans and generated Swift typechecks do not substitute for these gates.

Cargo audit reports zero vulnerability-class findings and zero yanked entries,
but **nine informational warnings remain: two unsoundness advisories and seven
unmaintained-crate warnings**. They cover `lru`, `glib`, `derivative`,
`proc-macro-error`, and five UNIC crates. Bounded reachability evidence, owners,
and exit conditions are in
[Rust dependency security dispositions](rust-dependency-security.md#remaining-informational-warnings).
Warnings remain visible without blanket ignores. This is not an all-clear
security claim: upstream GTK/WebKit, Tantivy, and Tauri/URLPattern migrations
remain maintenance work.
