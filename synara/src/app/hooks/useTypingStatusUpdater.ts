import { useEffect, useMemo } from 'react';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../utils/desktop';

type TypingStatusUpdater = (typing: boolean) => void;

// Timing contract with matrix-sdk 0.19 `Room::typing_notice` (Core
// `matrix_typing_set`): typing=true goes out with a 4 s server timeout, and a
// repeated true is only re-sent once 3 s have passed since the last send. So
// while typing is active we must reach the SDK inside that 3-4 s window, or
// other members see the indicator lapse. A 500 ms refresh timer gives two
// chances in each window, also through pauses between keystrokes, at no more
// than two IPC calls a second. The SDK decides which refreshes reach the server.
export const TYPING_REFRESH_MS = 500;

// typing=false after this long without a keystroke.
export const TYPING_IDLE_STOP_MS = 5000;

const sendNativeTyping = (roomId: string, typing: boolean) => {
  if (!isSynaraDesktop()) return;
  void invokeDesktopWithAvailability('matrix_typing_set', { roomId, typing }).catch(() => {
    // Native command records a privacy-safe diagnostic.
  });
};

export type TypingStatusSender = {
  update: TypingStatusUpdater;
  /** Sends typing=false if typing is active and clears both timers. */
  stop: () => void;
};

export const createTypingStatusSender = (
  roomId: string,
  send: (roomId: string, typing: boolean) => void = sendNativeTyping
): TypingStatusSender => {
  let refreshTimer: ReturnType<typeof setInterval> | undefined;
  let idleTimer: ReturnType<typeof setTimeout> | undefined;

  const stop = () => {
    clearTimeout(idleTimer);
    idleTimer = undefined;
    if (refreshTimer === undefined) return;
    clearInterval(refreshTimer);
    refreshTimer = undefined;
    send(roomId, false);
  };

  const update: TypingStatusUpdater = (typing) => {
    if (!typing) {
      stop();
      return;
    }

    clearTimeout(idleTimer);
    idleTimer = setTimeout(stop, TYPING_IDLE_STOP_MS);
    if (refreshTimer !== undefined) return;

    send(roomId, true);
    refreshTimer = setInterval(() => send(roomId, true), TYPING_REFRESH_MS);
  };

  return { update, stop };
};

export const useTypingStatusUpdater = (roomId: string): TypingStatusUpdater => {
  const sender = useMemo(() => createTypingStatusSender(roomId), [roomId]);

  // Stops the previous room on room change and this room on unmount.
  useEffect(() => sender.stop, [sender]);

  return sender.update;
};
