import { useCallback, useEffect, useState } from 'react';
import {
  getNativeSecretStorageStatus,
  NATIVE_SECRET_STORAGE_CHANGED,
  NativeSecretStorageStatus,
} from '../features/secret-storage/nativeSecretStorage';

export type NativeSecretStorageHook = {
  status?: NativeSecretStorageStatus;
  loading: boolean;
  error?: string;
  refresh: () => void;
};

export const useNativeSecretStorage = (): NativeSecretStorageHook => {
  const [status, setStatus] = useState<NativeSecretStorageStatus>();
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();
  const [refreshGeneration, setRefreshGeneration] = useState(0);
  const refresh = useCallback(() => setRefreshGeneration((generation) => generation + 1), []);

  useEffect(() => {
    let disposed = false;
    setLoading(true);
    setError(undefined);
    getNativeSecretStorageStatus()
      .then((nextStatus) => {
        if (!disposed) {
          setStatus(nextStatus);
          setLoading(false);
        }
      })
      .catch(() => {
        if (!disposed) {
          setStatus(undefined);
          setError('Native secret storage status is unavailable.');
          setLoading(false);
        }
      });
    return () => {
      disposed = true;
    };
  }, [refreshGeneration]);

  useEffect(() => {
    window.addEventListener(NATIVE_SECRET_STORAGE_CHANGED, refresh);
    return () => window.removeEventListener(NATIVE_SECRET_STORAGE_CHANGED, refresh);
  }, [refresh]);

  return {
    status,
    loading,
    error,
    refresh,
  };
};
