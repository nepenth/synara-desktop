import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test, { mock } from 'node:test';
import {
  createTypingStatusSender,
  TYPING_IDLE_STOP_MS,
  TYPING_REFRESH_MS,
} from '../useTypingStatusUpdater';

type TypingCall = { roomId: string; typing: boolean; at: number };

const withDesktop = (run: (calls: TypingCall[]) => void) => {
  const calls: TypingCall[] = [];
  const originalWindow = globalThis.window;
  mock.timers.enable({ apis: ['setTimeout', 'setInterval', 'Date'], now: 0 });
  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      invoke: async (command: string, args?: Record<string, unknown>) => {
        assert.equal(command, 'matrix_typing_set');
        calls.push({
          roomId: args?.roomId as string,
          typing: args?.typing as boolean,
          at: Date.now(),
        });
        return undefined;
      },
    },
  };

  try {
    run(calls);
  } finally {
    mock.timers.reset();
    (globalThis as any).window = originalWindow;
  }
};

// Mocked Date.now() reads the end of a tick inside timer callbacks, so step in
// 50 ms slices: every keystroke and timer in these tests lands on one.
const advance = (ms: number) => {
  for (let elapsed = 0; elapsed < ms; elapsed += 50) {
    mock.timers.tick(Math.min(50, ms - elapsed));
  }
};

// Keystrokes every 150 ms, like steady typing.
const typeFor = (update: (typing: boolean) => void, ms: number) => {
  for (let elapsed = 0; elapsed < ms; elapsed += 150) {
    update(true);
    advance(150);
  }
};

const refreshGaps = (calls: TypingCall[]) => {
  const refreshes = calls.filter((call) => call.typing);
  return refreshes.slice(1).map((call, index) => call.at - refreshes[index].at);
};

test('continuous typing refreshes typing=true inside the SDK resend window', () => {
  withDesktop((calls) => {
    const sender = createTypingStatusSender('!room:example.org');
    typeFor(sender.update, 12_000);

    assert.equal(
      calls.some((call) => !call.typing),
      false,
      'typing=false must not be sent while typing continues'
    );
    assert.equal(calls[0]?.at, 0);
    // The SDK resends after 3 s and the server times out at 4 s, so every
    // 3-4 s window needs a call; also stay well clear of one call per keystroke.
    for (const gap of refreshGaps(calls)) {
      assert.equal(gap, TYPING_REFRESH_MS);
    }
    assert.ok(12_000 - calls[calls.length - 1].at <= TYPING_REFRESH_MS);
    assert.ok(calls.length <= 12_000 / TYPING_REFRESH_MS + 1);

    sender.stop();
  });
});

test('a pause inside the SDK resend window keeps typing=true coming', () => {
  withDesktop((calls) => {
    const sender = createTypingStatusSender('!room:example.org');
    // Last keystroke at 3000 ms, then a 1.5 s pause across the 3-4 s window.
    typeFor(sender.update, 3_001);
    advance(1_500 - 150);
    typeFor(sender.update, 3_000);

    assert.equal(
      calls.some((call) => !call.typing),
      false
    );
    const gaps = refreshGaps(calls);
    assert.ok(gaps.length >= 14);
    for (const gap of gaps) {
      assert.ok(gap <= TYPING_REFRESH_MS, `refresh gap ${gap} ms during a pause`);
    }
    assert.ok(calls.some((call) => call.at > 3_000 && call.at < 4_000));

    sender.stop();
  });
});

test('idle typing sends typing=false once, 5 s after the last keystroke', () => {
  withDesktop((calls) => {
    const sender = createTypingStatusSender('!room:example.org');
    typeFor(sender.update, 2_000);
    // Last keystroke at 2100 ms.
    sender.update(true);

    advance(TYPING_IDLE_STOP_MS - 1);
    assert.equal(
      calls.some((call) => !call.typing),
      false,
      'typing=false waits for the idle delay'
    );
    advance(1);
    assert.deepEqual(calls[calls.length - 1], {
      roomId: '!room:example.org',
      typing: false,
      at: 2_100 + TYPING_IDLE_STOP_MS,
    });

    const settled = calls.length;
    advance(TYPING_IDLE_STOP_MS * 3);
    sender.stop();
    assert.equal(calls.length, settled, 'no refresh or idle timer survives the idle stop');
    assert.equal(calls.filter((call) => !call.typing).length, 1);
  });
});

test('an explicit stop sends typing=false and clears both timers', () => {
  withDesktop((calls) => {
    const sender = createTypingStatusSender('!room:example.org');
    sender.update(true);
    advance(1_000);
    sender.update(false);

    assert.deepEqual(
      calls.map(({ typing, at }) => ({ typing, at })),
      [
        { typing: true, at: 0 },
        { typing: true, at: 500 },
        { typing: true, at: 1_000 },
        { typing: false, at: 1_000 },
      ]
    );
    advance(TYPING_IDLE_STOP_MS * 2);
    assert.equal(calls.length, 4, 'no refresh or second typing=false after stop');

    sender.update(false);
    assert.equal(calls.length, 4, 'stopping while idle sends nothing');
  });
});

test('a room change stops the old room and unmount clears its timers', () => {
  withDesktop((calls) => {
    const first = createTypingStatusSender('!first:example.org');
    first.update(true);
    advance(TYPING_REFRESH_MS - 1);

    // The hook's effect cleanup runs `stop` for the previous room.
    first.stop();
    const second = createTypingStatusSender('!second:example.org');
    second.update(true);
    // Unmount runs the same cleanup for the current room.
    second.stop();

    assert.deepEqual(
      calls.map(({ roomId, typing }) => ({ roomId, typing })),
      [
        { roomId: '!first:example.org', typing: true },
        { roomId: '!first:example.org', typing: false },
        { roomId: '!second:example.org', typing: true },
        { roomId: '!second:example.org', typing: false },
      ]
    );
    advance(TYPING_IDLE_STOP_MS * 2);
    assert.equal(calls.length, 4, 'no refresh or idle timer survives stop');
  });
});

test('a stale update after stop ends with typing=false after the idle delay', () => {
  withDesktop((calls) => {
    const sender = createTypingStatusSender('!room:example.org');
    sender.update(true);
    sender.stop();
    const stopped = calls.length;

    sender.update(true);
    advance(TYPING_IDLE_STOP_MS);
    const stale = calls.slice(stopped);
    assert.equal(stale[0]?.typing, true);
    assert.deepEqual(stale[stale.length - 1], {
      roomId: '!room:example.org',
      typing: false,
      at: TYPING_IDLE_STOP_MS,
    });

    const settled = calls.length;
    advance(TYPING_IDLE_STOP_MS * 3);
    assert.equal(calls.length, settled, 'the stale update leaves no timer behind');
  });
});

test('the hook stops typing on room change and unmount', () => {
  const source = readFileSync('src/app/hooks/useTypingStatusUpdater.ts', 'utf8');
  assert.match(source, /useMemo\(\(\) => createTypingStatusSender\(roomId\), \[roomId\]\)/);
  assert.match(source, /useEffect\(\(\) => sender\.stop, \[sender\]\)/);
});

test('the composer passes typing=false on send and on a cleared editor', () => {
  const source = readFileSync('src/app/features/room/RoomInput.tsx', 'utf8');
  assert.match(source, /const sendTypingStatus = useTypingStatusUpdater\(roomId\)/);
  assert.match(source, /sendTypingStatus\(!isEmptyEditor\(editor\)\)/);
  assert.ok(
    (
      source.match(
        /resetEditor\(editor\);\s*resetEditorHistory\(editor\);\s*sendTypingStatus\(false\)/g
      ) ?? []
    ).length >= 2
  );
});
