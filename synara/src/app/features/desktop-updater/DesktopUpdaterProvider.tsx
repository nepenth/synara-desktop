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
import { Button, Icon, Icons, Spinner, Text } from 'folds';
import { ConfirmDialog } from '../../components/confirm-dialog';
import { SettingTile } from '../../components/setting-tile';
import {
  checkDesktopUpdate,
  DesktopUpdateCheckResult,
  DesktopUpdateCheckSource,
  DesktopUpdateDownloadProgress,
  getDismissedUpdateVersion,
  installUpdateAndRelaunch,
  InAppUpdateHandle,
  InAppUpdaterPlatform,
  reduceDownloadProgress,
  setDismissedUpdateVersion,
  shouldPromptForUpdate,
  updateErrorMessage,
} from '../../utils/desktopUpdater';
import { APP_VERSION, openExternalUrl } from '../../utils/appLinks';

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

/** "Checked just now", "Checked 5 min ago", then the clock time. */
const checkedLabel = (checkedAt?: number, now = Date.now()): string | undefined => {
  if (!checkedAt) return undefined;
  const minutes = Math.round((now - checkedAt) / 60000);
  if (minutes < 1) return 'Checked just now';
  if (minutes < 60) return `Checked ${minutes} min ago`;
  return `Checked ${new Date(checkedAt).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })}`;
};

/** Body of the update prompt: one sentence about what was found. */
const describeResult = (result?: DesktopUpdateCheckResult): string => {
  if (!result) return `Version ${APP_VERSION}.`;
  if (result.status === 'available') {
    if (result.platform === 'linux') {
      return `Version ${result.version} is available. Use apt upgrade, paru -Syu, or pacman -Syu to update.`;
    }
    return `Version ${result.version} is available.`;
  }
  if (result.status === 'up-to-date') {
    return `Synara is up to date at version ${result.currentVersion}.`;
  }
  return result.message;
};

/** Settings tile description: "Version 2.7.6 · Checked 5 min ago". */
const describeTile = (result?: DesktopUpdateCheckResult, checkedAt?: number): string => {
  const checked = checkedLabel(checkedAt);
  const version =
    result?.status === 'available'
      ? `Version ${APP_VERSION} · ${result.version} available`
      : result?.status === 'up-to-date'
        ? `Version ${result.currentVersion} · Up to date`
        : result
          ? result.message
          : `Version ${APP_VERSION}`;
  return checked ? `${version} · ${checked}` : version;
};

const progressLabel = (progress?: DesktopUpdateDownloadProgress): string => {
  if (!progress) return 'Preparing update…';
  if (progress.finished) return 'Installing update…';
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
  const canInstall = isAvailable && (result.platform === 'macos' || result.platform === 'windows');
  const canOpenRelease = isAvailable && result.platform === 'linux';
  const actionable = canInstall || canOpenRelease;

  return (
    <ConfirmDialog
      tone="primary"
      title={state.promptError ? 'Update check failed' : availableTitle(result!)}
      description={state.promptError ?? describeResult(result)}
      confirmLabel={
        canInstall ? 'Install and restart' : canOpenRelease ? 'Open release page' : 'OK'
      }
      busyLabel="Installing…"
      busy={state.installing}
      cancelLabel="Later"
      hideCancel={!actionable}
      onConfirm={() => {
        if (canInstall) void installAvailableUpdate();
        else if (canOpenRelease) void openPromptReleasePage();
        else closePrompt();
      }}
      onCancel={isAvailable ? dismissPrompt : closePrompt}
    >
      {result?.status === 'available' && result.body && (
        <Text size="T300" priority="400" style={{ whiteSpace: 'pre-wrap' }}>
          {result.body}
        </Text>
      )}
      {state.installing && (
        <Text size="T300" priority="400" role="status">
          {progressLabel(state.downloadProgress)}
        </Text>
      )}
    </ConfirmDialog>
  );
}

type DesktopUpdaterProviderProps = {
  children: ReactNode;
};

export function DesktopUpdaterProvider({ children }: DesktopUpdaterProviderProps) {
  const inAppUpdateRef = useRef<
    { update: InAppUpdateHandle; platform: InAppUpdaterPlatform } | undefined
  >(undefined);
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
      inAppUpdateRef.current =
        result.status === 'available' && result.inAppUpdate && result.platform !== 'linux'
          ? { update: result.inAppUpdate, platform: result.platform }
          : undefined;
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
    const pending = inAppUpdateRef.current;
    if (!pending) return;

    setState((current) => ({
      ...current,
      installing: true,
      downloadProgress: {
        downloadedBytes: 0,
        finished: false,
      },
    }));

    try {
      await installUpdateAndRelaunch(pending.update, pending.platform, (event) => {
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
      description={describeTile(state.lastResult, state.lastCheckedAt)}
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
          <Text size="B300">{state.checking ? 'Checking…' : 'Check for updates'}</Text>
        </Button>
      }
    />
  );
}
