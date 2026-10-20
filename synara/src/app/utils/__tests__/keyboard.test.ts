import assert from 'node:assert/strict';
import test from 'node:test';
import { isImeKeyEvent, onTabPress, type KeyboardEventLike } from '../keyboard';

const tab = (
  extra: Partial<KeyboardEventLike> = {}
): KeyboardEventLike & { prevented: boolean } => {
  const evt = {
    key: 'Tab',
    which: 9,
    keyCode: 9,
    altKey: false,
    ctrlKey: false,
    metaKey: false,
    shiftKey: false,
    prevented: false,
    preventDefault() {
      evt.prevented = true;
    },
    ...extra,
  };
  return evt;
};

test('Tab picks an autocomplete suggestion when no input method is composing', () => {
  let picked = 0;
  const evt = tab();
  onTabPress(evt, () => {
    picked += 1;
  });
  assert.equal(picked, 1);
  assert.equal(evt.prevented, true);
});

test('Tab is left to the input method while it composes', () => {
  for (const evt of [
    tab({ isComposing: true }),
    tab({ nativeEvent: { isComposing: true } }),
    tab({ keyCode: 229 }),
  ]) {
    assert.equal(isImeKeyEvent(evt), true);
    let picked = 0;
    onTabPress(evt, () => {
      picked += 1;
    });
    assert.equal(picked, 0);
    assert.equal(evt.prevented, false, 'the IME needs Tab to move between candidates');
  }
});
