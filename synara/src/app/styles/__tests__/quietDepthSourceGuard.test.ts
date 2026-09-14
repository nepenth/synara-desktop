import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const source = (path: string) => readFileSync(path, 'utf8');

test('quiet depth system preserves accessibility preferences and keeps text flat', () => {
  const depth = source('src/app/styles/Depth.css.ts');

  assert.match(depth, /prefers-reduced-transparency: reduce/);
  assert.match(depth, /prefers-contrast: more/);
  assert.match(depth, /prefers-reduced-motion: reduce/);
  assert.match(depth, /quietSurfaceFold/);
  assert.match(depth, /export const restingInnerEdge/);
  assert.match(depth, /export const floatingSurface/);
  assert.match(depth, /export const criticalSurface/);
  assert.match(depth, /export const avatarSurface/);
  assert.match(depth, /export const avatarMedia/);
  assert.match(depth, /export const quietInteractiveSurface/);
  assert.match(depth, /export const quietActionButton/);
  assert.match(depth, /:not\(:disabled\):not\(\[aria-disabled=true\]\)/);
  assert.match(depth, /&:disabled, &\[aria-disabled=true\]/);
  assert.doesNotMatch(depth, /textShadow|text-shadow/);
});

test('desktop hierarchy uses semantic depth while keeping text itself flat', () => {
  const nav = source('src/app/components/nav/styles.css.ts');
  const editor = source('src/app/components/editor/Editor.css.ts');
  const timeline = source('src/app/features/room/nativeTimelineHtml.css.ts');
  const legacyMessage = source('src/app/features/room/message/styles.css.ts');
  const roomNav = source('src/app/features/room-nav/styles.css.ts');

  assert.match(nav, /&\[aria-selected=true\]/);
  assert.match(nav, /raisedShadow/);
  assert.match(editor, /raisedShadow/);
  assert.match(editor, /EditorFloatingOptions/);
  assert.doesNotMatch(editor, /floatingShadow|quietSurfaceFold|linear-gradient/);
  assert.match(editor, /backgroundImage: 'none'/);
  assert.match(timeline, /MessageActionRail = style\(\[\s*floatingSurface/);
  assert.match(timeline, /MessageBody = style\(\{\s*background: 'transparent'/);
  assert.match(timeline, /MessageActionSurface}:hover/);
  assert.match(timeline, /boxShadow: 'none'/);
  assert.match(timeline, /boxShadow: raisedShadow/);
  assert.match(timeline, /border-color 140ms ease-out/);
  assert.match(timeline, /synara-depth-contrast-edge/);
  assert.match(timeline, /TimelineAvatar = style\(\[avatarSurface\]\)/);
  assert.match(timeline, /ReplySurface/);
  assert.match(legacyMessage, /MessageOptionsBar = style\(\[\s*DefaultReset,\s*floatingSurface/);
  assert.match(legacyMessage, /MessageBase = style\(\{/);
  assert.match(legacyMessage, /boxShadow: 'none'/);
  assert.match(legacyMessage, /synara-depth-contrast-edge/);
  assert.match(roomNav, /RoomSurface = style/);
  assert.match(roomNav, /boxShadow: 'none'/);
  assert.doesNotMatch(roomNav, /restingInnerEdge/);
  assert.match(roomNav, /boxShadow: raisedShadow/);
});

test('identity, composer popouts, and critical approvals use their intended depth levels', () => {
  const userAvatar = source('src/app/components/user-avatar/UserAvatar.css.ts');
  const roomAvatar = source('src/app/components/room-avatar/RoomAvatar.css.ts');
  const approval = source('src/app/components/agent-approval/AgentApprovalCard.css.ts');
  const roomInput = source('src/app/features/room/RoomInput.tsx');
  const gifPicker = source('src/app/features/room/gif/GifPicker.tsx');
  const emojiBoard = source('src/app/components/emoji-board/components/styles.css.ts');

  assert.match(userAvatar, /avatarMedia/);
  assert.match(roomAvatar, /avatarMedia/);
  assert.match(approval, /criticalSurface/);
  assert.match(roomInput, /depthCss\.floatingSurface/);
  assert.match(gifPicker, /depthCss\.floatingSurface/);
  assert.match(emojiBoard, /floatingSurface/);
});

test('desktop controls and personal notes share quiet interactive depth', () => {
  const composer = source('src/app/features/room/RoomComposer.css.ts');
  const input = source('src/app/features/room/RoomInput.tsx');
  const header = source('src/app/features/room/RoomViewHeader.tsx');
  const home = source('src/app/pages/client/home/Home.tsx');
  const notes = source('src/app/features/room/room-notes/RoomNotesPanel.tsx');
  const notesCss = source('src/app/features/room/room-notes/RoomNotesPanel.css.ts');
  const sidebar = source('src/app/components/sidebar/SidebarItem.tsx');
  const editorToolbar = source('src/app/components/editor/Toolbar.tsx');

  assert.match(composer, /height: toRem\(50\)/);
  assert.doesNotMatch(composer, /EditorFloatingOptions/);
  assert.match(input, /className=\{css\.ComposerAction\}/);
  assert.match(header, /className=\{depthCss\.quietInteractiveSurface\}/);
  assert.match(home, /className=\{depthCss\.quietInteractiveSurface\}/);
  assert.match(notes, /className=\{css\.KindSwitch\}/);
  assert.match(notes, /aria-pressed=\{kind === 'note'\}/);
  assert.match(notes, /aria-pressed=\{kind === 'todo'\}/);
  assert.match(notesCss, /quietInteractiveSurface/);
  assert.match(notesCss, /boxShadow: raisedShadow/);
  assert.match(notesCss, /prefers-contrast: more/);
  assert.match(sidebar, /interactive && depthCss\.quietInteractiveSurface/);
  assert.match(sidebar, /aria-current=\{interactive && active \? 'page' : undefined\}/);
  assert.match(editorToolbar, /import \* as depthCss from '\.\.\/\.\.\/styles\/Depth\.css'/);
  // The formatting toolbar matches the composer exactly through the shared
  // quiet action recipe (transparent rest, tint + edge on hover, stronger
  // tint when pressed), which lives in Depth.css.ts next to
  // quietInteractiveSurface.
  assert.match(editorToolbar, /className=\{depthCss\.quietActionButton\}/);
  assert.match(editorToolbar, /fill="None"/);
  assert.doesNotMatch(editorToolbar, /<IconButton[^>]*variant="SurfaceVariant"/);
});

test('room menus, members, and message search share quiet interactive depth', () => {
  const header = source('src/app/features/room/RoomViewHeader.tsx');
  const members = source('src/app/features/room/MembersDrawer.tsx');
  const memberFilter = source('src/app/components/MembershipFilterMenu.tsx');
  const memberSort = source('src/app/components/MemberSortMenu.tsx');
  const sidePanel = source('src/app/features/room/RoomSidePanel.tsx');
  const search = source('src/app/features/message-search/MessageSearch.tsx');
  const searchInput = source('src/app/features/message-search/SearchInput.tsx');
  const searchFilters = source('src/app/features/message-search/SearchFilters.tsx');
  const searchResults = source('src/app/features/message-search/SearchResultGroup.tsx');

  // Overflow menu: floating container with quiet options.
  assert.match(header, /<Menu[\s>]/);
  assert.match(header, /ref=\{ref\}/);
  assert.match(header, /depthCss\.floatingSurface/);
  assert.match(memberFilter, /<Menu/);
  assert.match(memberFilter, /depthCss\.floatingSurface/);
  assert.match(memberSort, /<Menu/);
  assert.match(memberSort, /depthCss\.floatingSurface/);
  // Members drawer controls.
  assert.match(members, /import \* as depthCss from '\.\.\/\.\.\/styles\/Depth\.css'/);
  assert.match(members, /className=\{depthCss\.quietInteractiveSurface\}/);
  // Search section controls.
  assert.match(sidePanel, /className=\{depthCss\.quietInteractiveSurface\}/);
  assert.match(search, /className=\{depthCss\.quietInteractiveSurface\}/);
  assert.match(searchInput, /className=\{depthCss\.quietInteractiveSurface\}/);
  assert.match(searchFilters, /className=\{depthCss\.quietInteractiveSurface\}/);
  assert.match(searchFilters, /<Menu className=\{depthCss\.floatingSurface\}/);
  assert.match(searchResults, /className=\{depthCss\.quietInteractiveSurface\}/);
  // No menu option renders without quiet depth: each of these surfaces
  // carries at least one interactive class per MenuItem it renders.
  for (const [name, component] of Object.entries({
    header,
    memberFilter,
    memberSort,
    members,
    searchFilters,
  })) {
    const items = component.match(/<MenuItem[\s/>]/g) ?? [];
    const depth = component.match(/quietInteractiveSurface/g) ?? [];
    assert.ok(items.length > 0, `${name} must render menu options`);
    assert.ok(
      depth.length >= items.length,
      `${name} has ${items.length} menu options but only ${depth.length} quiet-depth classes`
    );
  }
});

const mixRgb = (
  fg: [number, number, number],
  bg: [number, number, number],
  amount: number
): [number, number, number] => [
  Math.round(fg[0] * amount + bg[0] * (1 - amount)),
  Math.round(fg[1] * amount + bg[1] * (1 - amount)),
  Math.round(fg[2] * amount + bg[2] * (1 - amount)),
];

const relativeLuminance = (rgb: [number, number, number]): number => {
  const linear = rgb.map((channel) => {
    const sample = channel / 255;
    return sample <= 0.03928 ? sample / 12.92 : ((sample + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2];
};

const contrastRatio = (a: [number, number, number], b: [number, number, number]): number => {
  const first = relativeLuminance(a);
  const second = relativeLuminance(b);
  const lighter = Math.max(first, second);
  const darker = Math.min(first, second);
  return (lighter + 0.05) / (darker + 0.05);
};

test('quiet action 7%/12% tints fail 3:1 so pressed keeps an inset ring after hover', () => {
  const pairs: Array<[[number, number, number], [number, number, number]]> = [
    [
      [0, 0, 0],
      [242, 242, 242],
    ],
    [
      [255, 255, 255],
      [26, 26, 26],
    ],
  ];
  for (const [foreground, background] of pairs) {
    const hover = mixRgb(foreground, background, 0.07);
    const pressed = mixRgb(foreground, background, 0.12);
    assert.ok(contrastRatio(hover, background) < 3, 'hover tint vs rest is below 3:1');
    assert.ok(contrastRatio(pressed, background) < 3, 'pressed tint vs rest is below 3:1');
    assert.ok(contrastRatio(pressed, hover) < 3, 'pressed vs hover tint is below 3:1');
  }

  const depth = source('src/app/styles/Depth.css.ts');
  const hoverShadow = depth.indexOf('const quietActionHoverShadow');
  const pressedShadow = depth.indexOf('const quietActionPressedShadow');
  const hoverRule = depth.indexOf('&&:not(:disabled):not([aria-disabled=true]):hover');
  const pressedRule = depth.indexOf(
    '&&:not(:disabled):not([aria-disabled=true])[aria-pressed=true]'
  );
  assert.ok(pressedShadow > hoverShadow);
  assert.ok(pressedRule > hoverRule);
  assert.match(depth, /quietActionPressedShadow = `inset 0 1px 0/);
  assert.match(
    depth,
    /inset 0 0 0 \$\{config\.borderWidth\.B300\} color-mix\(in srgb, currentColor/
  );
  assert.match(depth, /&&:disabled, &&\[aria-disabled=true\]/);
});

test('quiet-depth restyle covers new settings, explore, notes, approvals, and support surfaces', () => {
  const settingsCss = source('src/app/features/settings/styles.css.ts');
  const appearance = source('src/app/features/settings/appearance/Appearance.tsx');
  const deviceTile = source('src/app/features/settings/devices/DeviceTile.tsx');
  const devices = source('src/app/features/settings/devices/Devices.tsx');
  const explore = source('src/app/pages/client/explore/Explore.tsx');
  const server = source('src/app/pages/client/explore/Server.tsx');
  const exploreCss = source('src/app/pages/client/explore/style.css.ts');
  const notes = source('src/app/features/room/room-notes/RoomNotesPanel.tsx');
  const notesCss = source('src/app/features/room/room-notes/RoomNotesPanel.css.ts');
  const approvals = source('src/app/features/approvals/Approvals.tsx');
  const editor = source('src/app/features/room/message/MessageEditor.tsx');
  const toolbar = source('src/app/components/editor/Toolbar.tsx');
  const welcome = source('src/app/pages/client/WelcomePage.tsx');

  assert.match(settingsCss, /export const SettingsThemeSwatch/);
  assert.match(settingsCss, /&:focus-visible/);
  assert.doesNotMatch(appearance, /rgba\(255, 255, 255, 0\.18\)/);
  assert.match(appearance, /className=\{SettingsThemeSwatch\}/);
  assert.match(appearance, /aria-haspopup="menu"/);
  assert.match(deviceTile, /wrap="Wrap"/);
  assert.match(deviceTile, /aria-expanded=\{details\}/);
  assert.match(deviceTile, /userSelect: 'all'/);
  assert.match(devices, /SettingsQuietControl/);
  assert.match(explore, /aria-haspopup="dialog"/);
  assert.match(explore, /initialFocus: \(\) => serverInputRef\.current/);
  assert.match(server, /wrap="Wrap"/);
  assert.match(server, /role="alert"/);
  assert.match(server, /aria-haspopup="dialog"/);
  assert.match(exploreCss, /overflowWrap: 'anywhere'/);
  assert.match(exploreCss, /color\.Critical\.Main/);
  assert.doesNotMatch(exploreCss, /ContainerColor\(\{ variant: 'Critical' \}\)/);
  assert.match(notes, /role="group"/);
  assert.match(notes, /aria-label="Item kind"/);
  assert.match(notesCss, /export const PanelHeader/);
  assert.match(notesCss, /flexShrink: 0/);
  assert.match(approvals, /role="list"/);
  assert.match(approvals, /role="listitem"/);
  assert.match(approvals, /<time dateTime=/);
  assert.match(approvals, /dir="auto"/);
  assert.match(editor, /aria-controls=\{toolbar \? 'message-formatting-toolbar' : undefined\}/);
  assert.match(editor, /aria-haspopup="dialog"/);
  assert.match(toolbar, /filled=\{isMarkActive\(editor, format\)\}/);
  assert.match(welcome, /openExternalUrlFromClick\(evt, SYNARA_SUPPORT_URL\)/);
});
