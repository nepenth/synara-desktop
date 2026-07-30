import { useMemo, useRef } from 'react';
import { TYPING_TIMEOUT_MS } from '../state/typingMembers';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../utils/desktop';

type TypingStatusUpdater = (typing: boolean) => void;

const sendNativeTyping = (roomId: string, typing: boolean) => {
  if (!isSynaraDesktop()) return;
  void invokeDesktopWithAvailability('matrix_typing_set', { roomId, typing }).catch(() => {
    // Native command records a privacy-safe diagnostic.
  });
};

export const useTypingStatusUpdater = (roomId: string): TypingStatusUpdater => {
  const statusSentTsRef = useRef<number>(0);

  const sendTypingStatus: TypingStatusUpdater = useMemo(() => {
    statusSentTsRef.current = 0;
    return (typing) => {
      if (typing) {
        if (Date.now() - statusSentTsRef.current < TYPING_TIMEOUT_MS) {
          return;
        }

        sendNativeTyping(roomId, true);
        const sentTs = Date.now();
        statusSentTsRef.current = sentTs;

        // Don't believe server will timeout typing status;
        // Clear typing status after timeout if already not;
        setTimeout(() => {
          if (statusSentTsRef.current === sentTs) {
            sendNativeTyping(roomId, false);
            statusSentTsRef.current = 0;
          }
        }, TYPING_TIMEOUT_MS);
        return;
      }

      if (Date.now() - statusSentTsRef.current < TYPING_TIMEOUT_MS) {
        sendNativeTyping(roomId, false);
      }
      statusSentTsRef.current = 0;
    };
  }, [roomId]);

  return sendTypingStatus;
};
