import { useSetAtom } from 'jotai';
import { useCallback } from 'react';
import { backupRestoreProgressAtom } from '../state/backupRestore';
import { useMatrixClient } from './useMatrixClient';
import { useKeyBackupDecryptionKeyCached } from './useKeyBackup';
import { isNativeMatrixSession } from '../features/verification/nativeVerification';

export const useRestoreBackupOnVerification = () => {
  const setRestoreProgress = useSetAtom(backupRestoreProgressAtom);

  const mx = useMatrixClient();

  useKeyBackupDecryptionKeyCached(
    useCallback(() => {
      if (isNativeMatrixSession()) return;
      const crypto = mx.getCrypto();
      if (!crypto) return;

      crypto.restoreKeyBackup({
        progressCallback(progress) {
          setRestoreProgress(progress);
        },
      });
    }, [mx, setRestoreProgress])
  );
};
