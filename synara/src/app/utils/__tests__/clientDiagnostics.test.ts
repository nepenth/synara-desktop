import assert from 'node:assert/strict';
import test from 'node:test';
import { defaultDesktopPlatformSettings } from '../../state/settings';
import {
  buildClientDiagnosticPayload,
  recordClientDiagnostic,
  refreshDesktopDiagnosticsConfig,
  resetClientDiagnosticsForTests,
} from '../clientDiagnostics';

const enabledSettings = {
  ...defaultDesktopPlatformSettings,
  desktopDiagnosticsEnabled: true,
  desktopDiagnosticsPerformance: true,
  desktopDiagnosticsSession: true,
  desktopDiagnosticsRoomState: true,
};

test('client diagnostics are disabled by default', () => {
  const calls: unknown[] = [];
  const originalWindow = globalThis.window;
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: {
      __SYNARA_DESKTOP__: {
        invoke: (_command: string, args: unknown) => {
          calls.push(args);
          return Promise.resolve();
        },
      },
    },
  });
  try {
    refreshDesktopDiagnosticsConfig(defaultDesktopPlatformSettings);
    recordClientDiagnostic('session', 'bootstrap.complete', { success: true });
    assert.equal(calls.length, 0);
  } finally {
    if (originalWindow === undefined) Reflect.deleteProperty(globalThis, 'window');
    else Object.defineProperty(globalThis, 'window', { configurable: true, value: originalWindow });
    resetClientDiagnosticsForTests();
  }
});

test('client diagnostics strictly filter fields and tokenize Matrix identifiers', () => {
  const calls: Array<{ command: string; args: Record<string, unknown> }> = [];
  const originalWindow = globalThis.window;
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: {
      __SYNARA_DESKTOP__: {
        invoke: (command: string, args: Record<string, unknown>) => {
          calls.push({ command, args });
          return Promise.resolve();
        },
      },
    },
  });
  try {
    refreshDesktopDiagnosticsConfig(enabledSettings);
    recordClientDiagnostic(
      'room',
      'room-timeline.open',
      {
        openMode: 'unread-window',
        sequence: 999,
        traceSequence: 7,
        durationMs: 12.345,
        messageBody: 'private message body',
        accessToken: 'secret-token',
        reason: 'https://matrix.example/private',
      },
      { roomId: '!private:example.org', eventId: '$private-event' }
    );

    assert.equal(calls.length, 1);
    assert.equal(calls[0].command, 'desktop_record_diagnostic');
    const payload = calls[0].args as {
      category: string;
      event: string;
      fields: Record<string, unknown>;
    };
    assert.equal(payload.category, 'room');
    assert.equal(payload.event, 'room-timeline.open');
    assert.equal(payload.fields.openMode, 'unread-window');
    assert.equal(payload.fields.durationMs, 12.35);
    assert.notEqual(payload.fields.sequence, 999);
    assert.equal(payload.fields.traceSequence, 7);
    assert.equal(payload.fields.roomToken, 'room-1');
    assert.equal(payload.fields.eventToken, 'event-1');
    assert.equal(payload.fields.messageBody, undefined);
    assert.equal(payload.fields.accessToken, undefined);
    assert.equal(payload.fields.reason, undefined);
    const serialized = JSON.stringify(payload);
    assert.equal(serialized.includes('private message body'), false);
    assert.equal(serialized.includes('secret-token'), false);
    assert.equal(serialized.includes('!private:example.org'), false);
    assert.equal(serialized.includes('$private-event'), false);
    assert.equal(serialized.includes('matrix.example'), false);
  } finally {
    if (originalWindow === undefined) Reflect.deleteProperty(globalThis, 'window');
    else Object.defineProperty(globalThis, 'window', { configurable: true, value: originalWindow });
    resetClientDiagnosticsForTests();
  }
});

test('client diagnostic payloads reject unsafe event names and range content', () => {
  assert.equal(buildClientDiagnosticPayload('session', 'https://private.example', {}), undefined);

  const payload = buildClientDiagnosticPayload('performance', 'runtime.sample', {
    virtualRange: { startIndex: 10, endIndex: 18, label: 'secret' },
    unknownField: 'secret',
  });
  assert.deepEqual(payload?.fields.virtualRange, { startIndex: 10, endIndex: 18 });
  assert.equal(payload?.fields.unknownField, undefined);
});
