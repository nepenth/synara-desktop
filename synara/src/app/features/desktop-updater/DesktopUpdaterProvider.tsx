import React, {
  createContext,
  ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import {
  Box,
  Button,
  config,
  Dialog,
  Icon,
  Icons,
  Overlay,
  OverlayBackdrop,
  OverlayCenter,
  Spinner,
  Text,
} from 'folds';
import FocusTrap from 'focus-trap-react';
import { SettingTile } from '../../components/setting-tile';
import {
  checkDesktopUpdate,
  DesktopUpdateCheckResult,
  DesktopUpdateCheckSource,
  DesktopUpdateDownloadProgress,
  getDismissedUpdateVersion,
  installMacosUpdateAndRelaunch,
  MacosUpdateHandle,
  reduceDownloadProgress,
  setDismissedUpdateVersion,
  shouldPromptForUpdate,
  updateErrorMessage,
} from '../../utils/desktopUpdater';
import { APP_VERSION, openExternalUrl } from '../../utils/appLinks';
import { stopPropagation } from '../../utils/keyboard';

const CHECK_FOR_UPDATES_EVENT = 'synara://check-for-updates';
const BACKGROUND_CHECK_INITIAL_DELAY_MS = 10_000;
const BACKGROUND_CHECK_INTERVAL_MS = 12 * 60 * 60 * 1000;

type DesktopUpdaterState = {
  checking: boolean;
  installing: boolean;
  lastCheckedAt?: number;
  lastResult?: DesktopUpdateCheckResult;
  promptResult?: DesktopUpdateCheckResult;
  promptError?: string;
  downloadProgress?: DesktopUpdateDownloadProgress;
};

type DesktopUpdaterContextValue = {
  state: DesktopUpdaterState;
  checkForUpdates: (
    source?: DesktopUpdateCheckSource
  ) => Promise<DesktopUpdateCheckResult | undefined>;
  installAvailableUpdate: () => Promise<void>;
  dismissPrompt: () => void;
  closePrompt: () => void;
  openPromptReleasePage: () => Promise<void>;
};

const DesktopUpdaterContext = createContext<DesktopUpdaterContextValue | undefined>(undefined);

export const useDesktopUpdater = (): DesktopUpdaterContextValue => {
  const context = useContext(DesktopUpdaterContext);
  if (!context) {
    throw new Error('useDesktopUpdater must be used inside DesktopUpdaterProvider.');
  }
  return context;
};

const availableTitle = (result: DesktopUpdateCheckResult): string => {
  if (result.status !== 'available') return 'Synara updates';
  return `Synara ${result.version} is available`;
};

const describeResult = (result?: DesktopUpdateCheckResult, checkedAt?: number): string => {
  if (!result) {
    return `Current version: v${APP_VERSION}.`;
  }
  const checkedText = checkedAt ? ` Last checked ${new Date(checkedAt).toLocaleString()}.` : '';
  if (result.status === 'available') {
    if (result.platform === 'linux') {
      return `Version ${result.version} is available. Use apt upgrade, paru -Syu, or pacman -Syu to update.${checkedText}`;
    }
    return `Version ${result.version} is available.${checkedText}`;
  }
  if (result.status === 'up-to-date') {
    return `Synara is up to date at v${result.currentVersion}.${checkedText}`;
  }
  return `${result.message}${checkedText}`;
};

const progressLabel = (progress?: DesktopUpdateDownloadProgress): string => {
  if (!progress) return 'Preparing update...';
  if (progress.finished) return 'Installing update...';
  if (!progress.contentLength) {
    return `Downloaded ${Math.round(progress.downloadedBytes / 1024 / 1024)} MB.`;
  }
  const percent = Math.min(
    100,
    Math.round((progress.downloadedBytes / progress.contentLength) * 100)
  );
  return `Downloaded ${percent}%.`;
};

function DesktopUpdaterPrompt({
  state,
  installAvailableUpdate,
  dismissPrompt,
  closePrompt,
  openPromptReleasePage,
}: {
  state: DesktopUpdaterState;
  installAvailableUpdate: () => Promise<void>;
  dismissPrompt: () => void;
  closePrompt: () => void;
  openPromptReleasePage: () => Promise<void>;
}) {
  const result = state.promptResult;
  const open = Boolean(result || state.promptError);
  if (!open) return null;

  const isAvailable = result?.status === 'available';
  const canInstall = isAvailable && result.platform === 'macos';
  const canOpenRelease = isAvailable && result.platform === 'linux';

  return (
    <Overlay open backdrop={<OverlayBackdrop />}>
      <OverlayCenter>
        <FocusTrap
          focusTrapOptions={{
            onDeactivate: closePrompt,
            clickOutsideDeactivates: !state.installing,
            escapeDeactivates: state.installing ? false : stopPropagation,
          }}
        >
          <Dialog variant="Surface">
            <Box direction="Column" gap="400" style={{ padding: config.space.S400, maxWidth: 420 }}>
              <Box direction="Column" gap="100">
                <Text size="H4">
                  {state.promptError ? 'Update check failed' : availableTitle(result!)}
                </Text>
                <Text size="T300" priority="400">
                  {state.promptError ?? describeResult(result, state.lastCheckedAt)}
                </Text>
                {result?.status === 'available' && result.body && (
                  <Text size="T300" priority="400">
                    {result.body}
                  </Text>
                )}
                {state.installing && (
                  <Text size="T300" priority="400">
                    {progressLabel(state.downloadProgress)}
                  </Text>
                )}
              </Box>
              <Box gap="200" justifyContent="End" wrap="Wrap">
                {canInstall && (
                  <Button
                    variant="Primary"
                    size="300"
                    radii="300"
                    disabled={state.installing}
                    before={
                      state.installing ? (
                        <Spinner size="100" variant="Primary" fill="Solid" />
                      ) : undefined
                    }
                    onClick={() => void installAvailableUpdate()}
                  >
                    <Text size="B300">Install and Restart</Text>
                  </Button>
                )}
                {canOpenRelease && (
                  <Button
                    variant="Primary"
                    size="300"
                    radii="300"
                    onClick={() => void openPromptReleasePage()}
                  >
                    <Text size="B300">Open Release Page</Text>
                  </Button>
                )}
                {isAvailable ? (
                  <Button
                    variant="Secondary"
                    fill="Soft"
                    size="300"
                    radii="300"
                    disabled={state.installing}
                    onClick={dismissPrompt}
                  >
                    <Text size="B300">Later</Text>
                  </Button>
                ) : (
                  <Button
                    variant="Secondary"
                    fill="Soft"
                    size="300"
                    radii="300"
                    onClick={closePrompt}
                  >
                    <Text size="B300">OK</Text>
                  </Button>
                )}
              </Box>
            </Box>
          </Dialog>
        </FocusTrap>
      </OverlayCenter>
    </Overlay>
  );
}

type DesktopUpdaterProviderProps = {
  children: ReactNode;
};

export function DesktopUpdaterProvider({ children }: DesktopUpdaterProviderProps) {
  const macosUpdateRef = useRef<MacosUpdateHandle | undefined>(undefined);
  const [state, setState] = useState<DesktopUpdaterState>({
    checking: false,
    installing: false,
  });

  const closePrompt = useCallback(() => {
    setState((current) => ({
      ...current,
      promptResult: undefined,
      promptError: undefined,
    }));
  }, []);

  const dismissPrompt = useCallback(() => {
    setState((current) => {
      if (current.promptResult?.status === 'available') {
        setDismissedUpdateVersion(window.localStorage, current.promptResult.version);
      }
      return {
        ...current,
        promptResult: undefined,
        promptError: undefined,
      };
    });
  }, []);

  const checkForUpdates = useCallback(async (source: DesktopUpdateCheckSource = 'manual') => {
    setState((current) => ({
      ...current,
      checking: true,
      promptError: undefined,
    }));

    try {
      const result = await checkDesktopUpdate();
      macosUpdateRef.current = result.status === 'available' ? result.macosUpdate : undefined;
      const checkedAt = Date.now();
      const showPrompt =
        result.status === 'available'
          ? shouldPromptForUpdate({
              source,
              version: result.version,
              dismissedVersion: getDismissedUpdateVersion(window.localStorage),
            })
          : source === 'manual';

      setState((current) => ({
        ...current,
        checking: false,
        lastCheckedAt: checkedAt,
        lastResult: result,
        promptResult: showPrompt ? result : current.promptResult,
      }));
      return result;
    } catch (error) {
      const message = updateErrorMessage(error);
      setState((current) => ({
        ...current,
        checking: false,
        lastCheckedAt: Date.now(),
        promptError: source === 'manual' ? message : undefined,
      }));
      return undefined;
    }
  }, []);

  const installAvailableUpdate = useCallback(async () => {
    const update = macosUpdateRef.current;
    if (!update) return;

    setState((current) => ({
      ...current,
      installing: true,
      downloadProgress: {
        downloadedBytes: 0,
        finished: false,
      },
    }));

    try {
      await installMacosUpdateAndRelaunch(update, (event) => {
        setState((current) => ({
          ...current,
          downloadProgress: reduceDownloadProgress(
            current.downloadProgress ?? { downloadedBytes: 0, finished: false },
            event
          ),
        }));
      });
    } catch (error) {
      setState((current) => ({
        ...current,
        installing: false,
        promptError: updateErrorMessage(error),
      }));
    }
  }, []);

  const openPromptReleasePage = useCallback(async () => {
    const releaseUrl =
      state.promptResult?.status === 'available' ? state.promptResult.releaseUrl : undefined;
    if (!releaseUrl) return;
    await openExternalUrl(releaseUrl);
    dismissPrompt();
  }, [dismissPrompt, state.promptResult]);

  useEffect(() => {
    const initialCheck = window.setTimeout(() => {
      void checkForUpdates('background');
    }, BACKGROUND_CHECK_INITIAL_DELAY_MS);
    const interval = window.setInterval(() => {
      void checkForUpdates('background');
    }, BACKGROUND_CHECK_INTERVAL_MS);

    return () => {
      window.clearTimeout(initialCheck);
      window.clearInterval(interval);
    };
  }, [checkForUpdates]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void import('@tauri-apps/api/event')
      .then(({ listen }) =>
        listen(CHECK_FOR_UPDATES_EVENT, () => {
          void checkForUpdates('manual');
        })
      )
      .then((cleanup) => {
        if (disposed) {
          cleanup();
          return;
        }
        unlisten = cleanup;
      })
      .catch(() => undefined);

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [checkForUpdates]);

  const value = useMemo<DesktopUpdaterContextValue>(
    () => ({
      state,
      checkForUpdates,
      installAvailableUpdate,
      dismissPrompt,
      closePrompt,
      openPromptReleasePage,
    }),
    [
      checkForUpdates,
      closePrompt,
      dismissPrompt,
      installAvailableUpdate,
      openPromptReleasePage,
      state,
    ]
  );

  return (
    <DesktopUpdaterContext.Provider value={value}>
      <DesktopUpdaterPrompt
        state={state}
        installAvailableUpdate={installAvailableUpdate}
        dismissPrompt={dismissPrompt}
        closePrompt={closePrompt}
        openPromptReleasePage={openPromptReleasePage}
      />
      {children}
    </DesktopUpdaterContext.Provider>
  );
}

export function UpdateSettingsTile() {
  const { state, checkForUpdates } = useDesktopUpdater();
  const busy = state.checking || state.installing;

  return (
    <SettingTile
      title="Updates"
      description={describeResult(state.lastResult, state.lastCheckedAt)}
      after={
        <Button
          onClick={() => void checkForUpdates('manual')}
          variant="Secondary"
          fill="Soft"
          size="300"
          radii="300"
          outlined
          disabled={busy}
          before={
            busy ? (
              <Spinner size="100" variant="Secondary" fill="Soft" />
            ) : (
              <Icon src={Icons.Download} size="100" />
            )
          }
        >
          <Text size="B300">{state.checking ? 'Checking...' : 'Check for Updates'}</Text>
        </Button>
      }
    />
  );
}
