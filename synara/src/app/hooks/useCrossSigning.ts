import { useCallback, useEffect, useState } from 'react';
import { AccountDataEvent, SecretAccountData } from '../../types/matrix/accountData';
import { useAccountData } from './useAccountData';
import { isNativeMatrixSession } from '../features/verification/nativeVerification';
import {
  getNativeCrossSigningStatus,
  isNativeCrossSigningPublished,
  NATIVE_CROSS_SIGNING_CHANGED,
  NativeCrossSigningStatus,
} from '../features/cross-signing/nativeCrossSigning';

export type CrossSigningHook = {
  active: boolean;
  nativeStatus?: NativeCrossSigningStatus;
  loading: boolean;
  error?: string;
  refresh: () => void;
};

export const useCrossSigning = (): CrossSigningHook => {
  const masterEvent = useAccountData(AccountDataEvent.CrossSigningMaster);
  const content = masterEvent?.getContent<SecretAccountData>();
  const nativeSession = isNativeMatrixSession();
  const [nativeStatus, setNativeStatus] = useState<NativeCrossSigningStatus>();
  const [loading, setLoading] = useState(nativeSession);
  const [error, setError] = useState<string>();
  const [refreshGeneration, setRefreshGeneration] = useState(0);
  const refresh = useCallback(() => setRefreshGeneration((generation) => generation + 1), []);

  useEffect(() => {
    if (!nativeSession) return undefined;
    let disposed = false;
    setLoading(true);
    setError(undefined);
    getNativeCrossSigningStatus()
      .then((status) => {
        if (!disposed) {
          setNativeStatus(status);
          setLoading(false);
        }
      })
      .catch(() => {
        if (!disposed) {
          setNativeStatus(undefined);
          setError('Native cross-signing status is unavailable.');
          setLoading(false);
        }
      });
    return () => {
      disposed = true;
    };
  }, [nativeSession, refreshGeneration]);

  useEffect(() => {
    if (!nativeSession) return undefined;
    window.addEventListener(NATIVE_CROSS_SIGNING_CHANGED, refresh);
    return () => window.removeEventListener(NATIVE_CROSS_SIGNING_CHANGED, refresh);
  }, [nativeSession, refresh]);

  return {
    active: nativeSession
      ? !!nativeStatus && isNativeCrossSigningPublished(nativeStatus)
      : !!content,
    nativeStatus: nativeSession ? nativeStatus : undefined,
    loading: nativeSession ? loading : false,
    error: nativeSession ? error : undefined,
    refresh,
  };
};

export const useCrossSigningActive = (): boolean => useCrossSigning().active;
