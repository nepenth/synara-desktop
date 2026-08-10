import { useEffect, useState } from 'react';
import { getRecentEmojis } from '../plugins/recent-emoji';
import type { MatrixClientReading, MatrixEventReading } from '../utils/room';
import { AccountDataEvent } from '../../types/matrix/accountData';
import { IEmoji } from '../plugins/emoji';

export const useRecentEmoji = (mx: MatrixClientReading, limit?: number): IEmoji[] => {
  const [recentEmoji, setRecentEmoji] = useState(() => getRecentEmojis(mx, limit));

  useEffect(() => {
    const handleAccountData = (event: MatrixEventReading) => {
      if (event.getType() !== AccountDataEvent.ElementRecentEmoji) return;
      setRecentEmoji(getRecentEmojis(mx, limit));
    };

    (
      mx as unknown as {
        on(event: string, listener: (event: MatrixEventReading) => void): void;
        removeListener(event: string, listener: (event: MatrixEventReading) => void): void;
      }
    ).on('AccountData', handleAccountData);
    return () => {
      (
        mx as unknown as {
          on(event: string, listener: (event: MatrixEventReading) => void): void;
          removeListener(event: string, listener: (event: MatrixEventReading) => void): void;
        }
      ).removeListener('AccountData', handleAccountData);
    };
  }, [mx, limit]);

  return recentEmoji;
};
