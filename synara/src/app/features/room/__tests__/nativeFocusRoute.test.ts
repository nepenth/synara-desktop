import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  FOCUS_TARGET_MISSING_MESSAGE,
  FOCUS_TARGET_UNREACHABLE_MESSAGE,
  resolveNativeFocusRouteAction,
} from '../nativeFocusRoute';
import { resolveThreadPane } from '../threadPane';

test('a routed thread reply opens its thread at the reply', () => {
  assert.deepEqual(
    resolveNativeFocusRouteAction('$reply', { kind: 'thread', root_event_id: '$root' }),
    { kind: 'open_thread', rootEventId: '$root', replyEventId: '$reply' }
  );
});

test('a routed room event, or a thread root, stays in the room stream', () => {
  assert.deepEqual(resolveNativeFocusRouteAction('$event', { kind: 'room' }), {
    kind: 'show_room',
  });
  assert.deepEqual(
    resolveNativeFocusRouteAction('$root', { kind: 'thread', root_event_id: '$root' }),
    { kind: 'show_room' }
  );
});

test('a target that cannot be shown says why instead of a generic failure', () => {
  assert.deepEqual(resolveNativeFocusRouteAction('$gone', { kind: 'missing' }), {
    kind: 'notice',
    message: FOCUS_TARGET_MISSING_MESSAGE,
  });
  assert.deepEqual(resolveNativeFocusRouteAction('$far', { kind: 'unreachable' }), {
    kind: 'notice',
    message: FOCUS_TARGET_UNREACHABLE_MESSAGE,
  });
  assert.doesNotMatch(FOCUS_TARGET_MISSING_MESSAGE, /temporarily unavailable/);
});

test('the side pane keeps the routed reply it should scroll to', () => {
  assert.deepEqual(
    resolveThreadPane({
      mode: 'side',
      opened: { rootEventId: '$root', focusEventId: '$reply' },
      desktop: true,
    }),
    { rootEventId: '$root', focusEventId: '$reply' }
  );
});
