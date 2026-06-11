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
import { HttpApiEvent, HttpApiEventHandlerMap, MatrixClient } from 'matrix-js-sdk';
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
  scheduleProactiveTokenRefresh,
  startClient,
  type ProactiveTokenRefreshHandle,
} from '../../../client/initMatrix';
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
import { getActiveSession } from '../../state/sessionBootstrap';
import { AutoDiscovery } from './AutoDiscovery';
import { platformSessionStore, repairPlatformDeviceDisplayName } from '../../platform';
import { migrateLegacySessionToNativeAfterClientInit } from '../../state/sessionPersistence';
import { shouldRetrySyncOnResume } from '../../utils/syncLifecycle';
import {
  logSyncStateTransition,
  shouldShowSyncRecoveryUI,
  SYNC_PREPARED_TIMEOUT_MS,
} from '../../utils/syncSplashRecovery';

function ClientRootLoading() {
  return (
    <SplashScreen>
      <Box direction="Column" grow="Yes" alignItems="Center" justifyContent="Center" gap="400">
        <Spinner variant="Secondary" size="600" />
        <Text>Heating up</Text>
      </Box>
    </SplashScreen>
  );
}

function ClientRootOptions({ mx }: { mx?: MatrixClient }) {
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
                <MenuItem onClick={() => performLogout(mx)}
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

const useLogoutListener = (mx?: MatrixClient) => {
  useEffect(() => {
    const handleLogout: HttpApiEventHandlerMap[HttpApiEvent.SessionLoggedOut] = async () => {
      await performLogout(mx);
    };

    mx?.on(HttpApiEvent.SessionLoggedOut, handleLogout);
    return () => {
      mx?.removeListener(HttpApiEvent.SessionLoggedOut, handleLogout);
    };
  }, [mx]);
};

const useSyncResumeRetry = (mx?: MatrixClient) => {
  useEffect(() => {
    if (!mx) return undefined;

    let retryTimer: number | undefined;

    const retrySyncIfNeeded = () => {
      retryTimer = undefined;
      if (document.visibilityState === 'hidden' || !mx.clientRunning) return;
      if (shouldRetrySyncOnResume(mx.getSyncState())) {
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

const useProactiveTokenRefresh = (mx?: MatrixClient) => {
  useEffect(() => {
    if (!mx) return undefined;

    const session = getActiveSession();
    if (!session?.refreshToken) return undefined;

    const handle: ProactiveTokenRefreshHandle = scheduleProactiveTokenRefresh(mx, session, {
      nativeSessionStore: platformSessionStore,
    });

    return () => {
      handle.dispose();
    };
  }, [mx]);
};

const usePlatformDeviceDisplayNameRepair = (mx?: MatrixClient) => {
  useEffect(() => {
    const deviceId = mx?.getDeviceId();
    if (!mx || !deviceId) return undefined;

    let cancelled = false;
    void (async () => {
      if (cancelled) return;
      await repairPlatformDeviceDisplayName(mx);
    })().catch(() => undefined);

    return () => {
      cancelled = true;
    };
  }, [mx]);
};

type ClientRootProps = {
  children: ReactNode;
};
export function ClientRoot({ children }: ClientRootProps) {
  const [loading, setLoading] = useState(true);
  const [syncTimedOut, setSyncTimedOut] = useState(false);
  const syncRetryInFlightRef = useRef(false);
  const { baseUrl, userId } = getActiveSession() ?? {};

  const [loadState, loadMatrix] = useAsyncCallback<MatrixClient, Error, []>(
    useCallback(() => {
      const session = getActiveSession();
      if (!session) {
        throw new Error('No session Found!');
      }
      return initClient(session).then(async (client) => {
        await migrateLegacySessionToNativeAfterClientInit({
          nativeSessionStore: platformSessionStore,
        });
        return client;
      });
    }, [])
  );
  const mx = loadState.status === AsyncStatus.Success ? loadState.data : undefined;
  const [startState, startMatrix] = useAsyncCallback<void, Error, [MatrixClient]>(
    useCallback((m) => startClient(m), [])
  );

  useLogoutListener(mx);
  useSyncResumeRetry(mx);
  useProactiveTokenRefresh(mx);
  usePlatformDeviceDisplayNameRepair(mx);

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
      logSyncStateTransition(state, previous);
      if (state === 'PREPARED') {
        setLoading(false);
        setSyncTimedOut(false);
      }
    }, [])
  );

  useEffect(() => {
    if (!loading || !mx) {
      setSyncTimedOut(false);
      return undefined;
    }

    const timer = window.setTimeout(() => {
      setSyncTimedOut(true);
    }, SYNC_PREPARED_TIMEOUT_MS);

    return () => {
      window.clearTimeout(timer);
    };
  }, [loading, mx]);

  const handleSyncRecoveryRetry = useCallback(async () => {
    if (!mx || syncRetryInFlightRef.current) return;
    syncRetryInFlightRef.current = true;
    setSyncTimedOut(false);

    try {
      if (mx.clientRunning) {
        mx.retryImmediately();
        return;
      }
      if (startState.status !== AsyncStatus.Loading) {
        await startMatrix(mx);
      }
    } finally {
      syncRetryInFlightRef.current = false;
    }
  }, [mx, startMatrix, startState.status]);

  const showSyncRecoveryUI = shouldShowSyncRecoveryUI(loading, syncTimedOut);

  return (
    <AutoDiscovery userId={userId!} baseUrl={baseUrl!}>
      <SpecVersions baseUrl={baseUrl!}>
        {mx && <SyncStatus mx={mx} />}
        {loading && <ClientRootOptions mx={mx} />}
        {(loadState.status === AsyncStatus.Error || startState.status === AsyncStatus.Error) && (
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
                  <Button variant="Critical" onClick={mx ? () => startMatrix(mx) : loadMatrix}>
                    <Text as="span" size="B400">
                      Retry
                    </Text>
                  </Button>
                </Box>
              </Dialog>
            </Box>
          </SplashScreen>
        )}
        {showSyncRecoveryUI && (
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
        {loading || !mx ? (
          <ClientRootLoading />
        ) : (
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
