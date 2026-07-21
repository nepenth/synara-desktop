import assert from 'node:assert/strict';
import test from 'node:test';
import { clearDesktopDiagnostics, getDesktopDiagnosticEntries } from '../desktopDiagnostics';
import {
  clearFoundationDiagnosticTokens,
  recordFoundationDiagnostic,
} from '../foundationDiagnostics';

const resetDiagnostics = () => {
  clearDesktopDiagnostics();
  clearFoundationDiagnosticTokens();
};

test('foundation diagnostics tokenize identifiers and reject message-shaped fields', () => {
  resetDiagnostics();
  recordFoundationDiagnostic('timeline', 'room-timeline.open', {
    roomId: '!private-room:example.org',
    eventId: '$private-event',
    fields: {
      openMode: 'unread-window',
      linkedEventCount: 120,
      messageBody: 'do not record this message',
      reason: 'contains message text',
    },
  });

  const [entry] = getDesktopDiagnosticEntries();
  assert.ok(entry.startsWith('[synara:foundation] '));
  assert.equal(entry.includes('!private-room:example.org'), false);
  assert.equal(entry.includes('$private-event'), false);
  assert.equal(entry.includes('do not record this message'), false);
  assert.equal(entry.includes('contains message text'), false);

  const parsed = JSON.parse(entry.slice('[synara:foundation] '.length));
  assert.equal(parsed.room, 'room-1');
  assert.equal(parsed.eventToken, 'event-1');
  assert.equal(parsed.fields.openMode, 'unread-window');
  assert.equal(parsed.fields.reason, '[redacted]');
  assert.equal(parsed.fields.messageBody, undefined);
});

test('foundation diagnostic storage and structured entries remain bounded', () => {
  resetDiagnostics();
  for (let index = 0; index < 75; index += 1) {
    recordFoundationDiagnostic('activity', 'room-activity.updated', {
      roomId: `!room-${index}:example.org`,
      eventId: `$event-${index}`,
      fields: { revision: index, hasConcreteHead: true },
    });
  }

  const entries = getDesktopDiagnosticEntries();
  assert.equal(entries.length, 50);
  entries.forEach((entry) => {
    assert.ok(entry.length <= 220);
    assert.doesNotThrow(() => JSON.parse(entry.slice('[synara:foundation] '.length)));
  });
  assert.equal(
    entries.some((entry) => entry.includes('!room-')),
    false
  );
  assert.equal(
    entries.some((entry) => entry.includes('$event-')),
    false
  );
});

test('foundation diagnostics cannot fail client operations when a native logger throws', () => {
  resetDiagnostics();
  const originalWindow = globalThis.window;
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: {
      __SYNARA_DESKTOP__: {
        invoke: () => {
          throw new Error('logger unavailable');
        },
      },
    },
  });
  try {
    assert.doesNotThrow(() =>
      recordFoundationDiagnostic('read', 'marker.commit-success', {
        roomId: '!room:example.org',
        eventId: '$event',
      })
    );
  } finally {
    if (originalWindow === undefined) {
      Reflect.deleteProperty(globalThis, 'window');
    } else {
      Object.defineProperty(globalThis, 'window', { configurable: true, value: originalWindow });
    }
  }
});
