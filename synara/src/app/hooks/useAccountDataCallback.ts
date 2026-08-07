import type { MatrixClientReading } from '../utils/room';
import { useEffect } from 'react';

export const useAccountDataCallback = (
  mx: MatrixClientReading,
  onAccountData: (event: any) => void,
  enabled = true
) => {
  useEffect(() => {
    if (!enabled) return undefined;
    (
      mx as unknown as {
        on(event: string, listener: (event: any) => void): void;
        removeListener(event: string, listener: (event: any) => void): void;
      }
    ).on('AccountData', onAccountData);
    return () => {
      (
        mx as unknown as {
          on(event: string, listener: (event: any) => void): void;
          removeListener(event: string, listener: (event: any) => void): void;
        }
      ).removeListener('AccountData', onAccountData);
    };
  }, [mx, onAccountData, enabled]);
};
