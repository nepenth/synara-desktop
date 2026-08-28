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
  assert.match(editor, /floatingShadow/);
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
  assert.match(roomNav, /boxShadow: restingInnerEdge/);
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
