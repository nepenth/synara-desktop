import { useCallback, useEffect, useState } from 'react';
import { isNativeMatrixSession } from '../features/verification/nativeVerification';
import {
  getNativeBackupStatus,
  NATIVE_BACKUP_CHANGED,
  NativeBackupStatus,
} from '../features/backup/nativeBackup';

export type NativeKeyBackupHook = {
  status?: NativeBackupStatus;
  loading: boolean;
  error?: string;
  refresh: () => void;
};

export const useNativeKeyBackup = (): NativeKeyBackupHook => {
  const nativeSession = isNativeMatrixSession();
  const [status, setStatus] = useState<NativeBackupStatus>();
  const [loading, setLoading] = useState(nativeSession);
  const [error, setError] = useState<string>();
  const [refreshGeneration, setRefreshGeneration] = useState(0);
  const refresh = useCallback(() => setRefreshGeneration((generation) => generation + 1), []);

  useEffect(() => {
    if (!nativeSession) return undefined;
    let disposed = false;
    setLoading(true);
    setError(undefined);
    getNativeBackupStatus()
      .then((nextStatus) => {
        if (!disposed) {
          setStatus(nextStatus);
          setLoading(false);
        }
      })
      .catch(() => {
        if (!disposed) {
          setStatus(undefined);
          setError('Native encryption backup status is unavailable.');
          setLoading(false);
        }
      });
    return () => {
      disposed = true;
    };
  }, [nativeSession, refreshGeneration]);

  useEffect(() => {
    if (!nativeSession) return undefined;
    window.addEventListener(NATIVE_BACKUP_CHANGED, refresh);
    return () => window.removeEventListener(NATIVE_BACKUP_CHANGED, refresh);
  }, [nativeSession, refresh]);

  return {
    status: nativeSession ? status : undefined,
    loading: nativeSession ? loading : false,
    error: nativeSession ? error : undefined,
    refresh,
  };
};
