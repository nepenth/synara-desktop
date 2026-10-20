import { isKeyHotkey } from 'is-hotkey';
import { KeyboardEventHandler } from 'react';

export interface KeyboardEventLike {
  key: string;
  which: number;
  keyCode?: number;
  altKey: boolean;
  ctrlKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
  /** DOM KeyboardEvent only; React events carry it on `nativeEvent`. */
  isComposing?: boolean;
  nativeEvent?: { isComposing?: boolean };
  preventDefault(): void;
}

/**
 * True for a key the input method handles (keyCode 229 is the "IME is
 * processing" code). Tab moves between conversion candidates there.
 */
export const isImeKeyEvent = (evt: KeyboardEventLike): boolean =>
  evt.isComposing === true || evt.nativeEvent?.isComposing === true || evt.keyCode === 229;

export const onTabPress = (evt: KeyboardEventLike, callback: () => void) => {
  if (isImeKeyEvent(evt)) return;
  if (isKeyHotkey('tab', evt)) {
    evt.preventDefault();
    callback();
  }
};

export const preventScrollWithArrowKey: KeyboardEventHandler = (evt) => {
  if (isKeyHotkey(['arrowup', 'arrowright', 'arrowdown', 'arrowleft'], evt)) {
    evt.preventDefault();
  }
};

export const onEnterOrSpace =
  <T>(callback: (evt: T) => void) =>
  (evt: KeyboardEventLike) => {
    if (isKeyHotkey('enter', evt) || isKeyHotkey('space', evt)) {
      evt.preventDefault();
      callback(evt as T);
    }
  };

export const stopPropagation = (evt: KeyboardEvent): boolean => {
  const ae = document.activeElement;
  const editableActiveElement = ae
    ? ae.nodeName.toLowerCase() === 'input' ||
      ae.nodeName.toLowerCase() === 'textarea' ||
      ae.getAttribute('contenteditable') === 'true'
    : false;

  if (editableActiveElement) return false;

  evt.stopPropagation();
  return true;
};

/**
 * The index a list key moves to: ArrowDown/ArrowUp step (stopping at the ends),
 * Home/End jump. Undefined for other keys or an empty list.
 */
export const listNavigationIndex = (
  key: string,
  current: number,
  count: number
): number | undefined => {
  if (count === 0) return undefined;
  switch (key) {
    case 'ArrowDown':
      return Math.min(current + 1, count - 1);
    case 'ArrowUp':
      return Math.max(current - 1, 0);
    case 'Home':
      return 0;
    case 'End':
      return count - 1;
    default:
      return undefined;
  }
};
