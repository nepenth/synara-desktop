import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  NativeTimelineRedactConfirmation,
  REDACT_CONFIRM_COPY,
  type NativeTimelineRedactTarget,
} from '../nativeTimelineRedact';

const presenter = readFileSync('src/app/features/room/NativeTimelinePresenter.tsx', 'utf8');
const dialog = readFileSync('src/app/features/room/NativeTimelineRedactConfirm.tsx', 'utf8');

const target: NativeTimelineRedactTarget = {
  sessionGeneration: 1,
  roomId: '!room:example.test',
  eventId: '$event',
};

test('redaction runs only after Remove, once, with the requested message', () => {
  const confirmation = new NativeTimelineRedactConfirmation();
  const redacted: NativeTimelineRedactTarget[] = [];
  const redact = (value: NativeTimelineRedactTarget) => redacted.push(value);

  // Nothing was asked: neither answer redacts.
  assert.equal(confirmation.settle('remove', redact), false);
  confirmation.request(target);
  assert.deepEqual(redacted, []);

  assert.equal(confirmation.settle('remove', redact), true);
  assert.deepEqual(redacted, [target]);
  // The dialog's unmount (onDeactivate) settles again; it must not redact twice.
  assert.equal(confirmation.settle('cancel', redact), false);
  assert.equal(confirmation.settle('remove', redact), false);
  assert.deepEqual(redacted, [target]);
});

test('Cancel, Escape and an outside click redact nothing', () => {
  const confirmation = new NativeTimelineRedactConfirmation();
  let calls = 0;
  confirmation.request(target);
  assert.equal(
    confirmation.settle('cancel', () => (calls += 1)),
    true
  );
  // A late Remove from the closing dialog is ignored.
  assert.equal(
    confirmation.settle('remove', () => (calls += 1)),
    false
  );
  assert.equal(calls, 0);
});

test('the row menu asks before redacting and keeps the guard and error copy', () => {
  const actions = presenter.slice(
    presenter.indexOf('const NativeTimelineRowActions = ({'),
    presenter.indexOf('const NativeTimelineUnsentChrome = ({')
  );
  // The menu item only asks; no redaction runs from the menu itself.
  assert.match(actions, /key="redact"[\s\S]*?onRequestRedact\(\);\s+closeAfterOneShotAction\(\);/);
  assert.doesNotMatch(actions, /redactWithNativeTimelineAction/);
  assert.match(actions, /if \(capabilities\.redact\) \{/);

  const surface = presenter.slice(presenter.indexOf('const NativeTimelineRowActionSurface = ({'));
  assert.equal(presenter.match(/redactWithNativeTimelineAction\(/g)?.length, 1);
  assert.match(
    surface,
    /redactConfirmation\.settle\(decision, \(target\) =>\s+runNativeRowAction\(\s+\(\) => redactWithNativeTimelineAction\(/
  );
  assert.match(surface, /TIMELINE_ACTION_FAILED\.redact,/);
  assert.match(surface, /nativeTimelineActionFlightKey\(\s+target\.sessionGeneration,/);
  assert.match(surface, /onRequestRedact=\{requestRedact\}/);
  // Focus goes back to the menu button after the dialog unmounts, in a layout
  // effect React sees, while the rail is held for that commit.
  assert.match(surface, /ref=\{menuButtonRef\}/);
  assert.match(
    surface,
    /hovered \|\| focusWithin \|\| menuOpen \|\| emojiBoardOpen \|\| redactConfirm !== undefined/
  );
  assert.match(
    surface,
    /useLayoutEffect\(\(\) => \{\s+if \(redactConfirm !== 'closing'\) return;\s+\(menuButtonRef\.current \?\? surfaceRef\.current\)\?\.focus\(/
  );
  assert.match(surface, /\{redactConfirm === 'open' && \(/);
});

test('the confirmation is an alertdialog named by its question with Cancel as default', () => {
  assert.equal(REDACT_CONFIRM_COPY.question, "Remove this message? This can't be undone.");
  assert.equal(REDACT_CONFIRM_COPY.remove, 'Remove');
  assert.match(dialog, /import \{ Dialog \} from '\.\.\/\.\.\/components\/modal-dialog';/);
  assert.match(dialog, /role="alertdialog"\s+aria-labelledby=\{questionId\}/);
  assert.match(dialog, /<Text id=\{questionId\}/);
  assert.match(dialog, /onDeactivate: onCancel,\s+clickOutsideDeactivates: true,/);
  assert.match(dialog, /escapeDeactivates: stopPropagation/);
  assert.match(dialog, /setReturnFocus: false/);
  assert.match(dialog, /<Button variant="Critical" onClick=\{onRemove\}>/);
  assert.match(dialog, /onClick=\{onCancel\} autoFocus>/);
  // No reason field yet.
  assert.doesNotMatch(dialog, /<textarea|<Input|reason/i);
});
