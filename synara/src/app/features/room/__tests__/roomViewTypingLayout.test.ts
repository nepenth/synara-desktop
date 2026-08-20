import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const typingCss = readFileSync('src/app/features/room/RoomViewTyping.css.ts', 'utf8');
const typingView = readFileSync('src/app/features/room/RoomViewTyping.tsx', 'utf8');
const roomView = readFileSync('src/app/features/room/RoomView.tsx', 'utf8');
const presenter = readFileSync('src/app/features/room/NativeTimelinePresenter.tsx', 'utf8');

test('typing indicator sits in layout flow instead of overlaying the last message', () => {
  assert.doesNotMatch(typingCss, /position:\s*['"]absolute['"]/);
  assert.doesNotMatch(typingCss, /bottom:\s*0/);
  assert.match(typingCss, /translateY\(100%\)/);
  assert.match(typingCss, /overflow:\s*['"]hidden['"]/);

  assert.match(typingView, /if \(typingNames\.length === 0\) \{\s*return null;/);
  assert.doesNotMatch(typingView, /position:\s*['"]relative['"]/);

  assert.match(
    roomView,
    /<NativeTimelinePresenter[\s\S]*<RoomViewTyping room=\{room\} \/>[\s\S]*<Box shrink="No"/
  );
  assert.match(roomView, /grow="Yes" direction="Column" style=\{\{ minHeight: 0 \}\}/);
});

test('native timeline scroll still shrinks with the in-flow typing bar', () => {
  assert.match(presenter, /<Box grow="Yes" direction="Column" style=\{\{ minHeight: 0 \}\}>/);
  assert.match(presenter, /style=\{\{ minHeight: 0, position: 'relative' \}\}/);
  assert.match(presenter, /style=\{\{ height: '100%' \}\}/);
});
