import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { nativeTimelineShieldPresentation } from '../nativeTimelineShield';
import { nativeTimelineRowShield, type NativeTimelineViewRow } from '../nativeTimelineView';

test('trusted rows and malformed shields render nothing', () => {
  assert.equal(nativeTimelineShieldPresentation(undefined), undefined);
  assert.equal(nativeTimelineShieldPresentation(null), undefined);
  assert.equal(
    nativeTimelineShieldPresentation({ tone: 'blue', code: 'unknown_device' }),
    undefined
  );
  assert.equal(nativeTimelineShieldPresentation({ tone: 'red', code: 'made_up' }), undefined);
  assert.equal(nativeTimelineShieldPresentation({ tone: 'red', code: 'toString' }), undefined);
});

test('red shields flag identity violations and mismatched senders', () => {
  for (const code of ['verification_violation', 'mismatched_sender']) {
    const shield = nativeTimelineShieldPresentation({ tone: 'red', code });
    assert.equal(shield?.tone, 'red');
    assert.equal(shield?.icon, 'shield');
    assert.ok(shield?.label);
  }
});

test('grey shields cover unknown authenticity; plaintext uses its own icon', () => {
  for (const code of [
    'authenticity_not_guaranteed',
    'unknown_device',
    'unsigned_device',
    'unverified_identity',
  ]) {
    assert.equal(nativeTimelineShieldPresentation({ tone: 'grey', code })?.icon, 'shield');
  }
  const clear = nativeTimelineShieldPresentation({ tone: 'grey', code: 'sent_in_clear' });
  assert.equal(clear?.icon, 'unencrypted');
  assert.equal(clear?.label, 'Not encrypted.');
});

test('only message-like rows carry a shield', () => {
  const shield = { tone: 'red', code: 'verification_violation' };
  const message = { kind: 'message', encryptionShield: shield } as unknown as NativeTimelineViewRow;
  const sticker = {
    kind: 'sticker',
    event: { encryptionShield: shield },
  } as unknown as NativeTimelineViewRow;
  const membership = {
    kind: 'membership',
    encryptionShield: shield,
  } as unknown as NativeTimelineViewRow;
  assert.deepEqual(nativeTimelineRowShield(message), shield);
  assert.deepEqual(nativeTimelineRowShield(sticker), shield);
  assert.equal(nativeTimelineRowShield(membership), undefined);
});

test('the presenter renders the shield with an accessible name on the row surface', () => {
  const source = readFileSync('src/app/features/room/NativeTimelinePresenter.tsx', 'utf8');
  assert.match(source, /nativeTimelineShieldPresentation\(nativeTimelineRowShield\(row\)\)/);
  assert.match(source, /data-native-timeline-shield=\{shield\.tone\}/);
  assert.match(source, /aria-label=\{shield\.label\}/);
});
