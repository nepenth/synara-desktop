import assert from 'node:assert/strict';
import { readdirSync, readFileSync, statSync } from 'node:fs';
// POSIX joins keep paths comparable with the allowlists below on Windows too.
import { join } from 'node:path/posix';
import test from 'node:test';
import { listNavigationIndex } from '../../utils/keyboard';

const source = (path: string) => readFileSync(path, 'utf8');

const tsxFiles = (dir: string): string[] =>
  readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === '__tests__' ? [] : tsxFiles(path);
    return path.endsWith('.tsx') ? [path] : [];
  });

const APP_FILES = tsxFiles('src/app').map((path) => ({ path, text: source(path) }));

const foldsImport = (text: string): string[] =>
  (text.match(/import \{([^}]*)\} from 'folds';/)?.[1] ?? '').split(',').map((name) => name.trim());

test('overlay dialogs use the Synara Modal/Dialog with dialog semantics', () => {
  // Non-modal folds Dialogs: splash-screen error cards, UIA stage cards inside the
  // UIAFlowOverlay dialog, and the composer (B9).
  const allowed = [
    'src/app/pages/ConfigConfig.tsx',
    'src/app/pages/client/ClientRoot.tsx',
    'src/app/components/uia-stages/',
    'src/app/components/modal-dialog/',
    'src/app/features/room/RoomInput.tsx',
  ];
  const offenders = APP_FILES.filter(
    ({ path, text }) =>
      !allowed.some((prefix) => path.startsWith(prefix)) &&
      foldsImport(text).some((name) => name === 'Modal' || name === 'Dialog')
  ).map(({ path }) => path);
  assert.deepEqual(offenders, []);

  const wrapper = source('src/app/components/modal-dialog/ModalDialog.tsx');
  assert.match(wrapper, /role = 'dialog'/);
  assert.match(wrapper, /aria-modal="true"/);
  assert.match(wrapper, /holdAppInert\(dialog\)/);
  assert.match(wrapper, /setAttribute\('aria-labelledby'/);
  assert.match(wrapper, /dialog\.focus\(\{ preventScroll: true \}\)/);
  const inert = source('src/app/components/modal-dialog/appInert.ts');
  assert.match(inert, /getElementById\('root'\)/);
  assert.match(inert, /setAttribute\('inert', ''\)/);
  assert.match(inert, /removeAttribute\('inert'\)/);
  assert.match(source('src/app/components/UIAFlowOverlay.tsx'), /<ModalDialogBox/);
  // The trap returns focus to the opener before the dialog unmounts, so it
  // must lift the inert first or the opener cannot take focus.
  assert.match(
    source('src/app/components/FocusTrap.tsx'),
    /onDeactivate: \(params: DeactivateParams\) => \{\s*if \(containerRef\.current\) releaseAppInertWithin\(containerRef\.current\);\s*focusTrapOptions\?\.onDeactivate\?\.\(params\);/
  );
  assert.match(
    source('src/app/components/settings-layout/SettingsLayout.tsx'),
    /aria-label="Settings"/
  );
});

test('settings switches are named after their row title', () => {
  const tile = source('src/app/components/setting-tile/SettingTile.tsx');
  assert.match(tile, /after\.type === Switch/);
  assert.match(tile, /'aria-labelledby': titleId/);
  assert.match(tile, /id=\{titleId\}/);
});

test('the store-recovery dialog traps focus, is an alertdialog and closes on Escape', () => {
  const login = source('src/app/pages/auth/login/PasswordLoginForm.tsx');
  const dialog = login.slice(login.indexOf('<Overlay open={storeRecoveryOpen}'));
  assert.match(dialog, /^[\s\S]*?<FocusTrap[\s\S]*?onDeactivate: closeStoreRecovery/);
  assert.match(dialog, /escapeDeactivates: \(evt\) => \{/);
  assert.match(dialog, /role="alertdialog"/);
  assert.match(dialog, /aria-labelledby=\{storeRecoveryTitleId\}/);
});

test('menus return focus when they close', () => {
  // The composer autocomplete keeps focus in the editor; Search returns focus
  // only on Escape; the composer belongs to B9.
  const allowed = [
    'src/app/components/editor/autocomplete/AutocompleteMenu.tsx',
    'src/app/features/room/RoomInput.tsx',
  ];
  const offenders = APP_FILES.filter(
    ({ path, text }) => !allowed.includes(path) && /returnFocusOnDeactivate: false/.test(text)
  ).map(({ path }) => path);
  assert.deepEqual(offenders, []);

  const row = source('src/app/features/room-nav/RoomNavItem.tsx');
  assert.match(row, /setReturnFocus: returnFocusOrFallback\(\(\) => linkRef\.current\)/);
  assert.match(row, /optionsVisible = hover \|\| !!menuAnchor \|\| returningFocus/);
  assert.match(row, /onPostDeactivate: \(\) => setReturningFocus\(false\)/);
  assert.match(row, /aria-expanded=\{!!menuAnchor\}/);
  assert.match(source('src/app/components/FocusTrap.tsx'), /export const returnFocusOrFallback/);
});

test('room rows: secondary colour for read names, arrow keys, current page', () => {
  const row = source('src/app/features/room-nav/RoomNavItem.tsx');
  assert.doesNotMatch(row, /priority=\{unread/);
  assert.match(row, /css\.RoomNameRead/);
  assert.match(row, /aria-current=\{selected \? 'page' : undefined\}/);
  assert.match(row, /listNavigationIndex\(evt\.key/);
  assert.match(
    source('src/app/features/room-nav/styles.css.ts'),
    /RoomNameRead = style\(\{\s*color: 'var\(--synara-content-secondary\)'/
  );

  assert.equal(listNavigationIndex('ArrowDown', 0, 3), 1);
  assert.equal(listNavigationIndex('ArrowDown', 2, 3), 2);
  assert.equal(listNavigationIndex('ArrowUp', 0, 3), 0);
  assert.equal(listNavigationIndex('Home', 2, 3), 0);
  assert.equal(listNavigationIndex('End', 0, 3), 2);
  assert.equal(listNavigationIndex('Enter', 0, 3), undefined);
  assert.equal(listNavigationIndex('ArrowDown', -1, 0), undefined);
});

test('the Cmd+K switcher is a combobox with a listbox of options', () => {
  const search = source('src/app/features/search/Search.tsx');
  assert.match(search, /role="combobox"/);
  assert.match(search, /aria-activedescendant=/);
  assert.match(search, /aria-controls=\{listboxId\}/);
  assert.match(search, /role="listbox"/);
  assert.equal(search.match(/role="option"/g)?.length, 2);
  assert.equal(search.match(/aria-selected=\{listFocus\.index === index\}/g)?.length, 2);
  assert.doesNotMatch(search, /aria-pressed=\{listFocus/);
  assert.match(search, /setReturnFocus: \(previous\) => \(choseResultRef\.current \? false/);
});

test('focus indicators: room list, text fields, Following bar, resize handle', () => {
  const nav = source('src/app/components/nav/styles.css.ts');
  assert.doesNotMatch(nav, /solid \$\{ContainerLine\}/);
  assert.equal(nav.match(/B600\} solid \$\{color\.Primary\.Main\}/g)?.length, 2);

  const rings = source('src/app/styles/FocusRing.css.ts');
  assert.match(rings, /div:has\(> input\.\$\{Disabled\}/);
  assert.match(rings, /textarea\.\$\{Disabled\}:focus-visible/);
  assert.match(
    rings,
    /outline: `\$\{config\.borderWidth\.B500\} solid \$\{color\.Primary\.Main\}`/
  );
  assert.match(source('src/app/pages/App.tsx'), /import '\.\.\/styles\/FocusRing\.css';/);

  assert.match(
    source('src/app/features/room/RoomViewFollowing.css.ts'),
    /'&:focus-visible': \{\s*outline:/
  );
  assert.match(
    source('src/app/features/room/ThreadSidePanel.css.ts'),
    /'&:focus-visible': \{\s*backgroundColor: color\.Primary\.Main/
  );
});

test('global CSS: high-contrast boosts reach body, forced colours keep focus and selection', () => {
  const css = source('src/index.css');
  const contrastBlock = css.slice(css.indexOf('@media (prefers-contrast: more)'));
  assert.match(contrastBlock, /^@media \(prefers-contrast: more\) \{[\s\S]*?:root,\s*body \{/);
  const forced = css.slice(css.indexOf('@media (forced-colors: active)'));
  assert.match(forced, /:focus-visible \{\s*outline: 2px solid Highlight !important;/);
  assert.match(forced, /\[aria-selected='true'\]/);
});

test('timestamps, composer and links do not rely on opacity or colour alone', () => {
  const time = source('src/app/components/message/Time.tsx');
  assert.doesNotMatch(time, /priority="300"/);
  assert.match(time, /color: 'var\(--synara-content-secondary\)'/);

  const editor = source('src/app/components/editor/Editor.tsx');
  assert.match(editor, /aria-label=\{ariaLabel \?\? placeholder\}/);
  assert.match(editor, /style=\{\{ \.\.\.attributes\.style, opacity: 1 \}\}/);
  assert.match(
    source('src/app/components/editor/Editor.css.ts'),
    /color: 'var\(--synara-content-secondary\)',\s*opacity: 1/
  );

  const html = source('src/app/features/room/nativeTimelineHtml.css.ts');
  assert.match(
    html,
    /globalStyle\(`\$\{MessageBody\} a:not\(\.\$\{UnfurlLink\}\):not\(\[data-mention-pill\]\)`/
  );
  // Mention pills skip the underline because a tinted fill marks them.
  assert.match(
    source('src/app/styles/CustomHtml.css.ts'),
    /export const Mention = recipe\(\{[\s\S]*?backgroundColor: `color-mix/
  );
  assert.match(html, /SpoilerButton = style\(\{[\s\S]*?color: 'var\(--synara-content-primary\)'/);
});

test('page zoom reaches 200%', () => {
  const appearance = source('src/app/features/settings/appearance/Appearance.tsx');
  assert.match(appearance, /Math\.min\(newZoom, 200\)/);
  assert.match(appearance, /max="200"/);
});
