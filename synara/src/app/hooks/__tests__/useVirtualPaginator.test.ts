import assert from 'node:assert/strict';
import test from 'node:test';
import {
  getRestoredScrollTop,
  getRestoreScrollData,
  getViewportRestoreAnchor,
} from '../useVirtualPaginator';

const makeScrollElement = (
  scrollTop: number,
  offsetHeight = 500,
  offsetTop = 0,
  scrollHeight = 2000
) =>
  ({
    scrollTop,
    offsetHeight,
    offsetTop,
    scrollHeight,
  } as HTMLElement);

const makeItemElement = (offsetTop: number, clientHeight: number) =>
  ({
    offsetTop,
    clientHeight,
  } as HTMLElement);

test('getRestoreScrollData preserves index zero anchors', () => {
  const anchor = makeItemElement(0, 120);
  const restoreData = getRestoreScrollData(24, [0, anchor]);

  assert.deepEqual(restoreData, {
    scrollTop: 24,
    anchorItem: 0,
    anchorOffsetTop: 0,
  });
});

test('getViewportRestoreAnchor selects the first item crossing the viewport top', () => {
  const elements = new Map<number, HTMLElement>([
    [0, makeItemElement(0, 120)],
    [1, makeItemElement(120, 240)],
    [2, makeItemElement(360, 160)],
  ]);

  const [item] = getViewportRestoreAnchor(makeScrollElement(180), { start: 0, end: 3 }, (index) =>
    elements.get(index)
  );

  assert.equal(item, 1);
});

test('getRestoredScrollTop compensates for anchor offset changes', () => {
  const restoreData = {
    scrollTop: 180,
    anchorItem: 1,
    anchorOffsetTop: 120,
  };

  assert.equal(getRestoredScrollTop(restoreData, makeItemElement(170, 240)), 230);
});
