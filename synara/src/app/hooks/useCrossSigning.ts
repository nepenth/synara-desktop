import { useEffect, useState } from 'react';
import { AccountDataEvent, SecretAccountData } from '../../types/matrix/accountData';
import { useAccountData } from './useAccountData';
import {
  getNativeCryptoStatus,
  isNativeMatrixSession,
} from '../features/verification/nativeVerification';

export const useCrossSigningActive = (): boolean => {
  const masterEvent = useAccountData(AccountDataEvent.CrossSigningMaster);
  const content = masterEvent?.getContent<SecretAccountData>();
  const nativeSession = isNativeMatrixSession();
  const [nativeActive, setNativeActive] = useState(false);

  useEffect(() => {
    if (!nativeSession) return undefined;
    let disposed = false;
    getNativeCryptoStatus()
      .then((status) => {
        if (!disposed) setNativeActive(status.crossSigningState === 'ready');
      })
      .catch(() => {
        if (!disposed) setNativeActive(false);
      });
    return () => {
      disposed = true;
    };
  }, [nativeSession]);

  return nativeSession ? nativeActive : !!content;
};
