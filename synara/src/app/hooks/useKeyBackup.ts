import {
  BackupTrustInfo,
  CryptoApi,
  CryptoEvent,
  CryptoEventHandlerMap,
  KeyBackupInfo,
} from 'matrix-js-sdk/lib/crypto-api';
import { useCallback, useEffect, useState } from 'react';
import { useMatrixClient } from './useMatrixClient';
import { useAlive } from './useAlive';
import { isNativeMatrixSession } from '../features/verification/nativeVerification';
import {
  getNativeBackupStatus,
  NATIVE_BACKUP_CHANGED,
  NativeBackupStatus,
} from '../features/backup/nativeBackup';

export const useKeyBackupStatusChange = (
  onChange: CryptoEventHandlerMap[CryptoEvent.KeyBackupStatus]
) => {
  const mx = useMatrixClient();
  const nativeSession = isNativeMatrixSession();

  useEffect(() => {
    if (nativeSession) return undefined;
    mx.on(CryptoEvent.KeyBackupStatus, onChange);
    return () => {
      mx.removeListener(CryptoEvent.KeyBackupStatus, onChange);
    };
  }, [mx, nativeSession, onChange]);
};

export const useKeyBackupStatus = (crypto: CryptoApi): boolean => {
  const alive = useAlive();
  const nativeSession = isNativeMatrixSession();
  const [status, setStatus] = useState(false);

  useEffect(() => {
    if (nativeSession) return undefined;
    crypto.getActiveSessionBackupVersion().then((v) => {
      if (alive()) {
        setStatus(typeof v === 'string');
      }
    });
    return undefined;
  }, [crypto, alive, nativeSession]);

  useKeyBackupStatusChange(setStatus);

  return status;
};

export const useKeyBackupSessionsRemainingChange = (
  onChange: CryptoEventHandlerMap[CryptoEvent.KeyBackupSessionsRemaining]
) => {
  const mx = useMatrixClient();
  const nativeSession = isNativeMatrixSession();

  useEffect(() => {
    if (nativeSession) return undefined;
    mx.on(CryptoEvent.KeyBackupSessionsRemaining, onChange);
    return () => {
      mx.removeListener(CryptoEvent.KeyBackupSessionsRemaining, onChange);
    };
  }, [mx, nativeSession, onChange]);
};

export const useKeyBackupFailedChange = (
  onChange: CryptoEventHandlerMap[CryptoEvent.KeyBackupFailed]
) => {
  const mx = useMatrixClient();
  const nativeSession = isNativeMatrixSession();

  useEffect(() => {
    if (nativeSession) return undefined;
    mx.on(CryptoEvent.KeyBackupFailed, onChange);
    return () => {
      mx.removeListener(CryptoEvent.KeyBackupFailed, onChange);
    };
  }, [mx, nativeSession, onChange]);
};

export const useKeyBackupDecryptionKeyCached = (
  onChange: CryptoEventHandlerMap[CryptoEvent.KeyBackupDecryptionKeyCached]
) => {
  const mx = useMatrixClient();
  const nativeSession = isNativeMatrixSession();

  useEffect(() => {
    if (nativeSession) return undefined;
    mx.on(CryptoEvent.KeyBackupDecryptionKeyCached, onChange);
    return () => {
      mx.removeListener(CryptoEvent.KeyBackupDecryptionKeyCached, onChange);
    };
  }, [mx, nativeSession, onChange]);
};

export const useKeyBackupSync = (): [number, string | undefined] => {
  const [remaining, setRemaining] = useState(0);
  const [failure, setFailure] = useState<string>();

  useKeyBackupSessionsRemainingChange(
    useCallback((count) => {
      setRemaining(count);
      setFailure(undefined);
    }, [])
  );

  useKeyBackupFailedChange(
    useCallback((f) => {
      if (typeof f === 'string') {
        setFailure(f);
        setRemaining(0);
      }
    }, [])
  );

  return [remaining, failure];
};

export const useKeyBackupInfo = (crypto: CryptoApi): KeyBackupInfo | undefined | null => {
  const alive = useAlive();
  const nativeSession = isNativeMatrixSession();
  const [info, setInfo] = useState<KeyBackupInfo | null>();

  const fetchInfo = useCallback(() => {
    if (nativeSession) return;
    crypto.getKeyBackupInfo().then((i) => {
      if (alive()) {
        setInfo(i);
      }
    });
  }, [crypto, alive, nativeSession]);

  useEffect(() => {
    fetchInfo();
  }, [fetchInfo]);

  useKeyBackupStatusChange(fetchInfo);

  useKeyBackupSessionsRemainingChange(
    useCallback(
      (remainingCount) => {
        if (remainingCount === 0) {
          fetchInfo();
        }
      },
      [fetchInfo]
    )
  );

  return info;
};

export const useKeyBackupTrust = (
  crypto: CryptoApi,
  backupInfo: KeyBackupInfo
): BackupTrustInfo | undefined => {
  const alive = useAlive();
  const nativeSession = isNativeMatrixSession();
  const [trust, setTrust] = useState<BackupTrustInfo>();

  const fetchTrust = useCallback(() => {
    if (nativeSession) return;
    crypto.isKeyBackupTrusted(backupInfo).then((t) => {
      if (alive()) {
        setTrust(t);
      }
    });
  }, [crypto, alive, backupInfo, nativeSession]);

  useEffect(() => {
    fetchTrust();
  }, [fetchTrust]);

  useKeyBackupStatusChange(fetchTrust);

  useKeyBackupDecryptionKeyCached(fetchTrust);

  return trust;
};

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
