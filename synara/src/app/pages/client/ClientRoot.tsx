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
/** The live client's type, derived like useMatrixClient (js-sdk-free here). */
type ClientMatrix = Awaited<ReturnType<typeof initClient>>;
import FocusTrap from 'focus-trap-react';
import React, {
  MouseEventHandler,
  ReactNode,
  useCallback,
  useEffect,
  useRef,
  useState,
} from 'react';
import {
  clearCacheAndReload,
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
import { MatrixClientProvider } from '../../hooks/useMatrixClient';
import { SpecVersions } from './SpecVersions';
import { AsyncStatus, useAsyncCallback } from '../../hooks/useAsyncCallback';
import { useSyncState } from '../../hooks/useSyncState';
import { stopPropagation } from '../../utils/keyboard';
import { SyncStatus } from './SyncStatus';
import { AuthMetadataProvider } from '../../hooks/useAuthMetadata';
import { getActiveSession, getSessionBootstrapResult } from '../../state/sessionBootstrap';
import { AutoDiscovery } from './AutoDiscovery';
import { shouldRetrySyncOnResume } from '../../utils/syncLifecycle';
import {
  formatSyncSplashStatus,
  logSyncStateTransition,
  selectSyncSplashView,
  SYNC_PREPARED_TIMEOUT_MS,
} from '../../utils/syncSplashRecovery';
import { recordClientDiagnostic } from '../../utils/clientDiagnostics';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';

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

function ClientRootOptions({ mx }: { mx?: ClientMatrix }) {
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
                {mx && (
                  <MenuItem onClick={() => clearCacheAndReload(mx)} size="300" radii="300">
                    <Text as="span" size="T300" truncate>
                      Clear Cache and Reload
                    </Text>
                  </MenuItem>
                )}
                <MenuItem
                  onClick={() => performLogout(mx)}
                  size="300"
                  radii="300"
                  variant="Critical"
                  fill="None"
                >
                  <Text as="span" size="T300" truncate>
                    Logout
                  </Text>
                </MenuItem>
              </Box>
            </Menu>
          </FocusTrap>
        }
      />
    </IconButton>
  );
}

const useLogoutListener = (mx?: ClientMatrix) => {
  useEffect(() => {
    const handleLogout = async () => {
      await performLogout(mx);
    };

    mx?.on('Session.logged_out' as unknown as Parameters<ClientMatrix['on']>[0], handleLogout);
    return () => {
      mx?.removeListener(
        'Session.logged_out' as unknown as Parameters<ClientMatrix['on']>[0],
        handleLogout
      );
    };
  }, [mx]);
};

const useSyncResumeRetry = (mx?: ClientMatrix) => {
  useEffect(() => {
    if (!mx) return undefined;

    let retryTimer: number | undefined;

    const retrySyncIfNeeded = () => {
      retryTimer = undefined;
      if (document.visibilityState === 'hidden' || !mx.clientRunning) return;
      const state = mx.getSyncState();
      if (shouldRetrySyncOnResume(state)) {
        recordClientDiagnostic('session', 'sync.resume-retry', {
          source: 'resume',
          syncState: String(state ?? 'null'),
          documentVisible: document.visibilityState === 'visible',
          online: navigator.onLine,
        });
        mx.retryImmediately();
      }
    };

    const scheduleRetry = () => {
      if (retryTimer !== undefined) {
        window.clearTimeout(retryTimer);
      }
      retryTimer = window.setTimeout(retrySyncIfNeeded, 0);
    };

    document.addEventListener('visibilitychange', scheduleRetry);
    window.addEventListener('focus', scheduleRetry);
    window.addEventListener('online', scheduleRetry);
    window.addEventListener('pageshow', scheduleRetry);

    return () => {
      if (retryTimer !== undefined) {
        window.clearTimeout(retryTimer);
      }
      document.removeEventListener('visibilitychange', scheduleRetry);
      window.removeEventListener('focus', scheduleRetry);
      window.removeEventListener('online', scheduleRetry);
      window.removeEventListener('pageshow', scheduleRetry);
    };
  }, [mx]);
};

const useProactiveTokenRefresh = (_mx?: ClientMatrix) => {
  // D1C: the renderer ceded token custody to native — native owns refresh via
  // `session_updated` (readiness/generation only). No renderer timer/handle.
  const clientArg = _mx !== undefined ? 1 : 0; // eslint-disable-line @typescript-eslint/no-unused-vars
  useEffect(() => undefined, []);
  void clientArg;
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
  const { baseUrl, userId } = getActiveSession() ?? {};

  const [loadState, loadMatrix] = useAsyncCallback<ClientMatrix, Error, []>(
    useCallback(() => {
      const session = getActiveSession();
      if (!session) {
        throw new Error('No session Found!');
      }
      return (async () => {
        if (isSynaraDesktop() && getSessionBootstrapResult().source === 'native') {
          const restored = await invokeDesktopWithAvailability('matrix_restore_session');
          if (!restored.available || !restored.value) {
            throw new Error('Native Matrix session is unavailable.');
          }
        }
        const client = await initClient(session);
        return client;
      })();
    }, [])
  );
  const mx = loadState.status === AsyncStatus.Success ? loadState.data : undefined;
  const [startState, startMatrix] = useAsyncCallback<void, Error, [ClientMatrix]>(
    useCallback((m) => startClient(m), [])
  );

  useLogoutListener(mx);
  useSyncResumeRetry(mx);
  useProactiveTokenRefresh(mx);

  // One ClientRoot owns the facade's readiness poll. Other consumers only
  // subscribe to sync events, avoiding duplicate timers per mounted widget.
  useEffect(() => {
    if (!mx) return undefined;
    return mx.watchSync();
  }, [mx]);

  useEffect(() => {
    if (loadState.status === AsyncStatus.Idle) {
      loadMatrix();
    }
  }, [loadState, loadMatrix]);

  useEffect(() => {
    if (mx && !mx.clientRunning) {
      startMatrix(mx);
    }
  }, [mx, startMatrix]);

  useSyncState(
    mx,
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
    if (!mx) setSyncState(null);
  }, [mx]);

  useEffect(() => {
    if (!loading || !mx) {
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
  }, [loading, mx]);

  const handleSyncRecoveryRetry = useCallback(async () => {
    if (!mx || syncRetryInFlightRef.current) return;
    syncRetryInFlightRef.current = true;
    setSyncTimedOut(false);
    recordClientDiagnostic('session', 'sync.recovery-requested', {
      source: 'user',
      syncState: String(syncState ?? 'null'),
    });

    try {
      if (mx.clientRunning()) {
        mx.retryImmediately();
        return;
      }
      if (startState.status !== AsyncStatus.Loading) {
        await startMatrix(mx);
      }
    } finally {
      syncRetryInFlightRef.current = false;
    }
  }, [mx, startMatrix, startState.status, syncState]);

  const splashStatus = formatSyncSplashStatus(
    syncState as Parameters<typeof formatSyncSplashStatus>[0],
    Boolean(mx)
  );
  const splashView = selectSyncSplashView({
    hasError: loadState.status === AsyncStatus.Error || startState.status === AsyncStatus.Error,
    hasClient: Boolean(mx),
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
        {mx && <SyncStatus mx={mx} />}
        {loading && <ClientRootOptions mx={mx} />}
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
                  {continuityError ? (
                    <>
                      <Text size="T300" priority="400">
                        Synara stopped before changing any encryption keys. Your local crypto store
                        is still intact.
                      </Text>
                      <Text size="T300" priority="400">
                        Before signing out, confirm that another verified client can decrypt your
                        history or that you have tested your recovery key/key backup. Signing out
                        permanently removes this device&apos;s local encryption data.
                      </Text>
                      {canRetryCryptoStoreContinuityFailure(continuityError) && (
                        <Button variant="Primary" onClick={loadMatrix}>
                          <Text as="span" size="B400">
                            Retry Safety Check
                          </Text>
                        </Button>
                      )}
                      <Button variant="Critical" onClick={() => void performLogout(mx)}>
                        <Text as="span" size="B400">
                          Sign Out and Delete Local Encryption Data
                        </Text>
                      </Button>
                    </>
                  ) : (
                    <Button variant="Critical" onClick={mx ? () => startMatrix(mx) : loadMatrix}>
                      <Text as="span" size="B400">
                        Retry
                      </Text>
                    </Button>
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
                  <Text size="T300" priority="400">
                    {splashStatus}
                  </Text>
                  <Text size="T300" priority="400">
                    You can retry, clear the local cache, or sign out.
                  </Text>
                  <Button variant="Primary" onClick={() => void handleSyncRecoveryRetry()}>
                    <Text as="span" size="B400">
                      Retry
                    </Text>
                  </Button>
                  {mx && (
                    <Button variant="Secondary" onClick={() => void clearCacheAndReload(mx)}>
                      <Text as="span" size="B400">
                        Clear Cache and Reload
                      </Text>
                    </Button>
                  )}
                  <Button variant="Critical" onClick={() => void performLogout(mx)}>
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
        {splashView === 'client' && mx && (
          <MatrixClientProvider value={mx}>
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
          </MatrixClientProvider>
        )}
      </SpecVersions>
    </AutoDiscovery>
  );
}
