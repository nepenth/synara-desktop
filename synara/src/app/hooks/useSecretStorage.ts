import {
  AccountDataEvent,
  SecretStorageDefaultKeyContent,
  SecretStorageKeyContent,
} from '../../types/matrix/accountData';
import { useAccountData } from './useAccountData';
import { isNativeMatrixSession } from '../features/verification/nativeVerification';
import {
  getNativeSecretStorageStatus,
  NATIVE_SECRET_STORAGE_CHANGED,
  NativeSecretStorageStatus,
} from '../features/secret-storage/nativeSecretStorage';
import { useCallback, useEffect, useState } from 'react';

export const getSecretStorageKeyEventType = (key: string): string => `m.secret_storage.key.${key}`;

export const useSecretStorageDefaultKeyId = (): string | undefined => {
  const nativeSession = isNativeMatrixSession();
  const defaultKeyEvent = useAccountData(AccountDataEvent.SecretStorageDefaultKey, !nativeSession);
  const defaultKeyId = defaultKeyEvent?.getContent<SecretStorageDefaultKeyContent>().key;

  return defaultKeyId;
};

export const useSecretStorageKeyContent = (keyId: string): SecretStorageKeyContent | undefined => {
  const nativeSession = isNativeMatrixSession();
  const keyEvent = useAccountData(getSecretStorageKeyEventType(keyId), !nativeSession);
  const secretStorageKey = keyEvent?.getContent<SecretStorageKeyContent>();

  return secretStorageKey;
};

export type NativeSecretStorageHook = {
  status?: NativeSecretStorageStatus;
  loading: boolean;
  error?: string;
  refresh: () => void;
};

export const useNativeSecretStorage = (): NativeSecretStorageHook => {
  const nativeSession = isNativeMatrixSession();
  const [status, setStatus] = useState<NativeSecretStorageStatus>();
  const [loading, setLoading] = useState(nativeSession);
  const [error, setError] = useState<string>();
  const [refreshGeneration, setRefreshGeneration] = useState(0);
  const refresh = useCallback(() => setRefreshGeneration((generation) => generation + 1), []);

  useEffect(() => {
    if (!nativeSession) return undefined;
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
  }, [nativeSession, refreshGeneration]);

  useEffect(() => {
    if (!nativeSession) return undefined;
    window.addEventListener(NATIVE_SECRET_STORAGE_CHANGED, refresh);
    return () => window.removeEventListener(NATIVE_SECRET_STORAGE_CHANGED, refresh);
  }, [nativeSession, refresh]);

  return {
    status: nativeSession ? status : undefined,
    loading: nativeSession ? loading : false,
    error: nativeSession ? error : undefined,
    refresh,
  };
};
