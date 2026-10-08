import {
  Box,
  Button,
  config,
  Dialog,
  Icon,
  IconButton,
  Icons,
  Menu,
  MenuItem,
  PopOut,
  RectCords,
  Spinner,
  Text,
} from 'folds';
/** The installed native session, as `initClient` returns it. */
type ClientMatrix = Awaited<ReturnType<typeof initClient>>;
import FocusTrap from 'focus-trap-react';
import { listen } from '@tauri-apps/api/event';
import { recordSessionExpiry } from '../../utils/sessionExpiry';
import React, {
  MouseEventHandler,
  ReactNode,
  useCallback,
  useEffect,
  useRef,
  useState,
} from 'react';
import {
  reloadApplication,
  initClient,
  performLogout,
  startClient,
} from '../../../client/initMatrix';
import {
  canRetryCryptoStoreContinuityFailure,
  CryptoStoreContinuityError,
} from '../../../client/cryptoStoreContinuity';
import { SplashScreen } from '../../components/splash-screen';
import { ServerConfigsLoader } from '../../components/ServerConfigsLoader';
import { CapabilitiesProvider } from '../../hooks/useCapabilities';
import { MediaConfigProvider } from '../../hooks/useMediaConfig';
import { SpecVersions } from './SpecVersions';
import { AsyncStatus, useAsyncCallback } from '../../hooks/useAsyncCallback';
import { useSyncState } from '../../hooks/useSyncState';
import { stopPropagation } from '../../utils/keyboard';
import { SyncStatus } from './SyncStatus';
import { AuthMetadataProvider } from '../../hooks/useAuthMetadata';
import { getActiveSession, getSessionBootstrapResult } from '../../state/sessionBootstrap';
import { AutoDiscovery } from './AutoDiscovery';
import {
  consumeHiddenDurationMs,
  shouldRecoverSyncOnWake,
  SYNC_WAKE_RECOVER_COOLDOWN_MS,
  type SyncWakeReason,
} from '../../utils/syncLifecycle';
import {
  formatSyncSplashStatus,
  logSyncStateTransition,
  selectSyncSplashView,
  SYNC_PREPARED_TIMEOUT_MS,
} from '../../utils/syncSplashRecovery';
import { recordClientDiagnostic } from '../../utils/clientDiagnostics';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import { getSettings } from '../../state/settings';

import { nativeSession } from '../../native/nativeSession';
function ClientRootLoading({ status }: { status: string }) {
  return (
    <SplashScreen>
      <Box direction="Column" grow="Yes" alignItems="Center" justifyContent="Center" gap="400">
        <Spinner variant="Secondary" size="600" />
        <Box direction="Column" alignItems="Center" gap="100">
          <Text>Heating up</Text>
          <Text size="T300" priority="400">
            {status}
          </Text>
        </Box>
      </Box>
    </SplashScreen>
  );
}

function ClientRootOptions({
  loadedSession,
  logout,
  logoutError,
  loggingOut,
}: {
  loadedSession?: ClientMatrix;
  logout: () => Promise<void>;
  logoutError?: string;
  loggingOut: boolean;
}) {
  const [menuAnchor, setMenuAnchor] = useState<RectCords>();

  const handleToggle: MouseEventHandler<HTMLButtonElement> = (evt) => {
    const cords = evt.currentTarget.getBoundingClientRect();
    setMenuAnchor((currentState) => {
      if (currentState) return undefined;
      return cords;
    });
  };

  return (
    <IconButton
      style={{
        position: 'absolute',
        top: config.space.S100,
        right: config.space.S100,
      }}
      variant="Background"
      fill="None"
      onClick={handleToggle}
    >
      <Icon size="200" src={Icons.VerticalDots} />
      <PopOut
        anchor={menuAnchor}
        position="Bottom"
        align="End"
        offset={6}
        content={
          <FocusTrap
            focusTrapOptions={{
              initialFocus: false,
              returnFocusOnDeactivate: false,
              onDeactivate: () => setMenuAnchor(undefined),
              clickOutsideDeactivates: true,
              isKeyForward: (evt: KeyboardEvent) => evt.key === 'ArrowDown',
              isKeyBackward: (evt: KeyboardEvent) => evt.key === 'ArrowUp',
              escapeDeactivates: stopPropagation,
            }}
          >
            <Menu>
              <Box direction="Column" gap="100" style={{ padding: config.space.S100 }}>
                {loadedSession && (
                  <MenuItem onClick={() => reloadApplication()} size="300" radii="300">
                    <Text as="span" size="T300" truncate>
                      Reload Application
                    </Text>
                  </MenuItem>
                )}
                <MenuItem
                  onClick={() => void logout()}
                  disabled={loggingOut}
                  size="300"
                  radii="300"
                  variant="Critical"
                  fill="None"
                >
                  <Text as="span" size="T300" truncate>
                    Logout
                  </Text>
                </MenuItem>
                {logoutError && (
                  <Text role="alert" size="T300">
                    {logoutError}
                  </Text>
                )}
              </Box>
            </Menu>
          </FocusTrap>
        }
      />
    </IconButton>
  );
}

const useLogoutListener = (session: ClientMatrix | undefined, logout: () => Promise<void>) => {
  useEffect(
    () =>
      session?.subscribe('loggedOut', () => {
        void logout();
      }),
    [session, logout]
  );
};

const useSyncResumeRetry = (loadedSession?: ClientMatrix) => {
  useEffect(() => {
    if (!loadedSession) return undefined;

    let retryTimer: number | undefined;
    let hiddenAtMs: number | null = document.visibilityState === 'hidden' ? Date.now() : null;
    let lastRecoverAtMs = 0;

    const retrySyncIfNeeded = (reason: SyncWakeReason, persisted?: boolean) => {
      retryTimer = undefined;
      if (document.visibilityState === 'hidden') return;
      const now = Date.now();
      const consumed = consumeHiddenDurationMs(hiddenAtMs, now);
      hiddenAtMs = consumed.hiddenAtMs;
      if (now - lastRecoverAtMs < SYNC_WAKE_RECOVER_COOLDOWN_MS) return;
      const state = nativeSession().getSyncState();
      if (
        !shouldRecoverSyncOnWake({
          reason,
          syncState: state,
          hiddenDurationMs: consumed.hiddenDurationMs,
          persisted,
        })
      ) {
        return;
      }
      lastRecoverAtMs = now;
      recordClientDiagnostic('session', 'sync.resume-retry', {
        source: 'resume',
        reason,
        syncState: String(state ?? 'null'),
        documentVisible: document.visibilityState === 'visible',
        online: navigator.onLine,
      });
      void nativeSession()
        .retrySyncNow()
        .catch(() => {
          // The native status poll owns connection state after a failed wake.
        });
    };

    const scheduleRetry = (reason: SyncWakeReason, persisted?: boolean) => {
      if (retryTimer !== undefined) {
        window.clearTimeout(retryTimer);
      }
      retryTimer = window.setTimeout(() => retrySyncIfNeeded(reason, persisted), 0);
    };

    const onVisibilityChange = () => {
      if (document.visibilityState === 'hidden') {
        hiddenAtMs = Date.now();
        return;
      }
      scheduleRetry('visibilitychange');
    };

    const onFocus = () => scheduleRetry('focus');
    const onOnline = () => scheduleRetry('online');
    const onPageShow = (event: PageTransitionEvent) => {
      scheduleRetry('pageshow', event.persisted);
    };

    document.addEventListener('visibilitychange', onVisibilityChange);
    window.addEventListener('focus', onFocus);
    window.addEventListener('online', onOnline);
    window.addEventListener('pageshow', onPageShow);

    return () => {
      if (retryTimer !== undefined) {
        window.clearTimeout(retryTimer);
      }
      document.removeEventListener('visibilitychange', onVisibilityChange);
      window.removeEventListener('focus', onFocus);
      window.removeEventListener('online', onOnline);
      window.removeEventListener('pageshow', onPageShow);
    };
  }, [loadedSession]);
};

type ClientRootProps = {
  children: ReactNode;
};
export function ClientRoot({ children }: ClientRootProps) {
  const [loading, setLoading] = useState(true);
  const [syncTimedOut, setSyncTimedOut] = useState(false);
  const [syncState, setSyncState] = useState<string | null>(null);
  const syncStateRef = useRef<string | null>(syncState);
  syncStateRef.current = syncState;
  const syncRetryInFlightRef = useRef(false);
  const [syncRetryPending, setSyncRetryPending] = useState(false);
  const [syncRecoveryError, setSyncRecoveryError] = useState<string>();
  const { baseUrl, userId } = getActiveSession() ?? {};

  const [loadState, loadMatrix] = useAsyncCallback<ClientMatrix, Error, []>(
    useCallback(() => {
      const session = getActiveSession();
      if (!session) {
        throw new Error('No session Found!');
      }
      return (async () => {
        if (isSynaraDesktop() && getSessionBootstrapResult().source === 'native') {
          const restored = await invokeDesktopWithAvailability('matrix_restore_session', {
            indexedMessageSearch: getSettings().indexedMessageSearch,
          });
          if (!restored.available || !restored.value) {
            throw new Error('Native Matrix session is unavailable.');
          }
        }
        const client = await initClient(session);
        return client;
      })();
    }, [])
  );
  const loadedSession = loadState.status === AsyncStatus.Success ? loadState.data : undefined;
  const [logoutError, setLogoutError] = useState<string>();
  const [loggingOut, setLoggingOut] = useState(false);
  const [sessionSaveFailed, setSessionSaveFailed] = useState(false);
  const logout = useCallback(async () => {
    if (loggingOut) return;
    setLoggingOut(true);
    setLogoutError(undefined);
    try {
      await performLogout(loadedSession);
    } catch {
      setLogoutError('Local sign out did not complete. Retry to finish local cleanup.');
    } finally {
      setLoggingOut(false);
    }
  }, [loadedSession, loggingOut]);

  const [startState, startMatrix] = useAsyncCallback<void, Error, [ClientMatrix]>(
    useCallback((m) => startClient(m), [])
  );

  useEffect(() => {
    if (!isSynaraDesktop()) return undefined;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen<number>('matrix-session-expired', ({ payload }) => {
      if (Number.isSafeInteger(payload) && payload === nativeSession().getSessionGeneration()) {
        recordSessionExpiry();
      }
    })
      .then((stop) => {
        if (disposed) stop();
        else unlisten = stop;
      })
      .catch(() => {});
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [loadedSession]);

  useEffect(() => {
    if (!isSynaraDesktop()) return undefined;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen<{ sessionGeneration: number; saved: boolean }>(
      'matrix-session-persistence',
      ({ payload }) => {
        if (
          payload?.sessionGeneration === nativeSession().getSessionGeneration() &&
          typeof payload?.saved === 'boolean'
        ) {
          setSessionSaveFailed(!payload.saved);
        }
      }
    )
      .then((stop) => {
        if (disposed) stop();
        else unlisten = stop;
      })
      .catch(() => {});
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [loadedSession]);

  useLogoutListener(loadedSession, logout);
  useSyncResumeRetry(loadedSession);

  // One ClientRoot owns the facade's readiness poll. Other consumers only
  // subscribe to sync events, avoiding duplicate timers per mounted widget.
  useEffect(() => {
    if (!loadedSession) return undefined;
    return nativeSession().watchSync();
  }, [loadedSession]);

  useEffect(() => {
    if (loadState.status === AsyncStatus.Idle) {
      void loadMatrix().catch(() => {
        // useAsyncCallback exposes restore errors without losing the session.
      });
    }
  }, [loadState, loadMatrix]);

  const retryLoadMatrix = useCallback(() => {
    void loadMatrix().catch(() => {
      // The error splash owns a rejected restore, including a manual retry.
    });
  }, [loadMatrix]);

  useEffect(() => {
    if (loadedSession && !nativeSession().isSyncRunning()) {
      void startMatrix(loadedSession).catch(() => {
        // useAsyncCallback exposes the startup error in the splash screen.
      });
    }
  }, [loadedSession, startMatrix]);

  useSyncState(
    loadedSession,
    useCallback((state, previous) => {
      setSyncState(state);
      logSyncStateTransition(state, previous);
      recordClientDiagnostic('session', 'sync.transition', {
        syncState: String(state ?? 'null'),
        previousSyncState: String(previous ?? 'null'),
      });
      if (state === 'PREPARED') {
        setLoading(false);
        setSyncTimedOut(false);
      }
    }, [])
  );

  useEffect(() => {
    if (!loadedSession) setSyncState(null);
  }, [loadedSession]);

  useEffect(() => {
    if (!loading || !loadedSession) {
      setSyncTimedOut(false);
      return undefined;
    }

    const timer = window.setTimeout(() => {
      setSyncTimedOut(true);
      recordClientDiagnostic('session', 'sync.prepared-timeout', {
        timedOut: true,
        syncState: String(syncStateRef.current ?? 'null'),
        documentVisible: document.visibilityState === 'visible',
        online: navigator.onLine,
      });
    }, SYNC_PREPARED_TIMEOUT_MS);

    return () => {
      window.clearTimeout(timer);
    };
  }, [loading, loadedSession]);

  const handleSyncRecoveryRetry = useCallback(async () => {
    if (!loadedSession || syncRetryInFlightRef.current) return;
    syncRetryInFlightRef.current = true;
    setSyncRetryPending(true);
    setSyncRecoveryError(undefined);
    recordClientDiagnostic('session', 'sync.recovery-requested', {
      source: 'user',
      syncState: String(syncState ?? 'null'),
    });

    try {
      // startClient only hydrates renderer reads. Every explicit sync Retry
      // must reach the native owner, including offline/failed/stopped states.
      await nativeSession().retrySyncNow();
      if (startState.status === AsyncStatus.Error) await startMatrix(loadedSession);
      // Only observed PREPARED readiness completes recovery. A successful
      // command can still leave the SDK starting or offline.
    } catch {
      setSyncRecoveryError('Could not restart sync. You can retry or reload the application.');
    } finally {
      syncRetryInFlightRef.current = false;
      setSyncRetryPending(false);
    }
  }, [loadedSession, startMatrix, startState.status, syncState]);

  const splashStatus = formatSyncSplashStatus(
    syncState as Parameters<typeof formatSyncSplashStatus>[0],
    Boolean(loadedSession)
  );
  const splashView = selectSyncSplashView({
    hasError: loadState.status === AsyncStatus.Error || startState.status === AsyncStatus.Error,
    hasClient: Boolean(loadedSession),
    loading,
    syncTimedOut,
  });
  const continuityError =
    (loadState.status === AsyncStatus.Error &&
      loadState.error instanceof CryptoStoreContinuityError &&
      loadState.error) ||
    (startState.status === AsyncStatus.Error &&
      startState.error instanceof CryptoStoreContinuityError &&
      startState.error) ||
    undefined;

  return (
    <AutoDiscovery userId={userId!} baseUrl={baseUrl!}>
      <SpecVersions baseUrl={baseUrl!}>
        {loadedSession && <SyncStatus session={loadedSession} />}
        {sessionSaveFailed && (
          <Text role="alert">
            Your refreshed session could not be saved. Check free disk space and unlock your system
            keychain. Keep Synara open until this warning clears.
          </Text>
        )}
        {loading && (
          <ClientRootOptions
            loadedSession={loadedSession}
            logout={logout}
            logoutError={logoutError}
            loggingOut={loggingOut}
          />
        )}
        {splashView === 'error' && (
          <SplashScreen>
            <Box
              direction="Column"
              grow="Yes"
              alignItems="Center"
              justifyContent="Center"
              gap="400"
            >
              <Dialog>
                <Box direction="Column" gap="400" style={{ padding: config.space.S400 }}>
                  {loadState.status === AsyncStatus.Error && (
                    <Text>{`Failed to load. ${loadState.error.message}`}</Text>
                  )}
                  {startState.status === AsyncStatus.Error && (
                    <Text>{`Failed to start. ${startState.error.message}`}</Text>
                  )}
                  {logoutError && <Text role="alert">{logoutError}</Text>}
                  {continuityError ? (
                    <>
                      <Text size="T300" priority="400">
                        Synara stopped before changing any encryption keys. Your local crypto store
                        is still intact.
                      </Text>
                      <Text size="T300" priority="400">
                        Before signing out, confirm that another verified client can decrypt your
                        history or that you have tested your recovery key/key backup. Signing out
                        ends this session; local encryption data stays on this device.
                      </Text>
                      {canRetryCryptoStoreContinuityFailure(continuityError) && (
                        <Button variant="Primary" onClick={retryLoadMatrix}>
                          <Text as="span" size="B400">
                            Retry Safety Check
                          </Text>
                        </Button>
                      )}
                      <Button
                        variant="Critical"
                        onClick={() => void logout()}
                        disabled={loggingOut}
                      >
                        <Text as="span" size="B400">
                          Sign Out and Delete Local Encryption Data
                        </Text>
                      </Button>
                    </>
                  ) : (
                    <>
                      {syncRecoveryError && <Text role="alert">{syncRecoveryError}</Text>}
                      <Button
                        variant="Critical"
                        disabled={syncRetryPending}
                        onClick={
                          loadedSession ? () => void handleSyncRecoveryRetry() : retryLoadMatrix
                        }
                      >
                        <Text as="span" size="B400">
                          Retry
                        </Text>
                      </Button>
                    </>
                  )}
                </Box>
              </Dialog>
            </Box>
          </SplashScreen>
        )}
        {splashView === 'recovery' && (
          <SplashScreen>
            <Box
              direction="Column"
              grow="Yes"
              alignItems="Center"
              justifyContent="Center"
              gap="400"
            >
              <Dialog>
                <Box direction="Column" gap="400" style={{ padding: config.space.S400 }}>
                  <Text>Sync is taking longer than expected.</Text>
                  {logoutError && <Text role="alert">{logoutError}</Text>}
                  {syncRecoveryError && <Text role="alert">{syncRecoveryError}</Text>}
                  <Text size="T300" priority="400">
                    {splashStatus}
                  </Text>
                  <Text size="T300" priority="400">
                    You can retry, reload the application, or sign out.
                  </Text>
                  <Button
                    variant="Primary"
                    disabled={syncRetryPending}
                    onClick={() => void handleSyncRecoveryRetry()}
                  >
                    <Text as="span" size="B400">
                      Retry
                    </Text>
                  </Button>
                  {loadedSession && (
                    <Button variant="Secondary" onClick={() => void reloadApplication()}>
                      <Text as="span" size="B400">
                        Reload Application
                      </Text>
                    </Button>
                  )}
                  <Button variant="Critical" onClick={() => void logout()} disabled={loggingOut}>
                    <Text as="span" size="B400">
                      Logout
                    </Text>
                  </Button>
                </Box>
              </Dialog>
            </Box>
          </SplashScreen>
        )}
        {splashView === 'loading' && <ClientRootLoading status={splashStatus} />}
        {splashView === 'client' && loadedSession && (
          <ServerConfigsLoader>
            {(serverConfigs) => (
              <CapabilitiesProvider value={serverConfigs.capabilities ?? {}}>
                <MediaConfigProvider value={serverConfigs.mediaConfig ?? {}}>
                  <AuthMetadataProvider value={serverConfigs.authMetadata}>
                    {children}
                  </AuthMetadataProvider>
                </MediaConfigProvider>
              </CapabilitiesProvider>
            )}
          </ServerConfigsLoader>
        )}
      </SpecVersions>
    </AutoDiscovery>
  );
}
