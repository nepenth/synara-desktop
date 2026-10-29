import { isDesktopPlatform, supportsPlatformUpdater } from '../platform';
import { APP_VERSION, SYNARA_RELEASES_URL } from './appLinks';
import { getDesktopPerformanceCapabilities, invokeDesktopWithAvailability } from './desktop';

export const SYNARA_LATEST_UPDATE_METADATA_URL =
  'https://github.com/nepenth/synara-desktop/releases/latest/download/latest.json';
export const DISMISSED_UPDATE_VERSION_KEY = 'synara.desktop.dismissedUpdateVersion';

export type DesktopUpdaterPlatform = 'macos' | 'windows' | 'linux' | 'unsupported';
/** Platforms that download, install and relaunch through the Tauri updater. */
export type InAppUpdaterPlatform = 'macos' | 'windows';
export type DesktopUpdateCheckSource = 'manual' | 'background';

export type DesktopUpdateDownloadEvent =
  | { event: 'Started'; data: { contentLength?: number } }
  | { event: 'Progress'; data: { chunkLength: number } }
  | { event: 'Finished' };

export type DesktopUpdateDownloadProgress = {
  contentLength?: number;
  downloadedBytes: number;
  finished: boolean;
};

export type InAppUpdateHandle = {
  currentVersion: string;
  version: string;
  date?: string;
  body?: string;
  downloadAndInstall: (onEvent?: (event: DesktopUpdateDownloadEvent) => void) => Promise<void>;
  close?: () => Promise<void>;
};

export type DesktopUpdateCheckResult =
  | {
      status: 'available';
      platform: InAppUpdaterPlatform | 'linux';
      currentVersion: string;
      version: string;
      date?: string;
      body?: string;
      releaseUrl: string;
      packageManagerHint?: string;
      inAppUpdate?: InAppUpdateHandle;
    }
  | {
      status: 'up-to-date';
      platform: InAppUpdaterPlatform | 'linux';
      currentVersion: string;
      version: string;
      releaseUrl: string;
    }
  | {
      status: 'unavailable';
      platform: DesktopUpdaterPlatform;
      currentVersion: string;
      message: string;
    };

export type DesktopUpdateCheckOptions = {
  currentVersion?: string;
  fetchImpl?: typeof fetch;
  getPlatform?: () => Promise<string>;
  inAppCheck?: () => Promise<InAppUpdateHandle | null>;
  supportsUpdater?: () => boolean;
};

type LatestJson = {
  version?: unknown;
  pub_date?: unknown;
};

export const normalizeDesktopUpdaterPlatform = (platform: string): DesktopUpdaterPlatform => {
  const normalized = platform.toLowerCase();
  if (normalized === 'darwin' || normalized === 'macos') return 'macos';
  if (normalized === 'windows') return 'windows';
  if (normalized === 'linux') return 'linux';
  return 'unsupported';
};

const versionParts = (version: string): number[] =>
  version
    .trim()
    .replace(/^v/i, '')
    .split(/[.-]/)
    .map((part) => Number.parseInt(part, 10))
    .filter((part) => Number.isFinite(part));

export const compareVersions = (left: string, right: string): number => {
  const leftParts = versionParts(left);
  const rightParts = versionParts(right);
  const length = Math.max(leftParts.length, rightParts.length);

  for (let index = 0; index < length; index += 1) {
    const leftPart = leftParts[index] ?? 0;
    const rightPart = rightParts[index] ?? 0;
    if (leftPart > rightPart) return 1;
    if (leftPart < rightPart) return -1;
  }

  return 0;
};

export const isNewerVersion = (candidate: string, current: string): boolean =>
  compareVersions(candidate, current) > 0;

export const getDismissedUpdateVersion = (storage: Pick<Storage, 'getItem'>): string | undefined =>
  storage.getItem(DISMISSED_UPDATE_VERSION_KEY) ?? undefined;

export const setDismissedUpdateVersion = (
  storage: Pick<Storage, 'setItem'>,
  version: string
): void => {
  storage.setItem(DISMISSED_UPDATE_VERSION_KEY, version);
};

export const shouldPromptForUpdate = ({
  source,
  version,
  dismissedVersion,
}: {
  source: DesktopUpdateCheckSource;
  version: string;
  dismissedVersion?: string;
}): boolean => source === 'manual' || dismissedVersion !== version;

export const reduceDownloadProgress = (
  progress: DesktopUpdateDownloadProgress,
  event: DesktopUpdateDownloadEvent
): DesktopUpdateDownloadProgress => {
  if (event.event === 'Started') {
    return {
      contentLength: event.data.contentLength,
      downloadedBytes: 0,
      finished: false,
    };
  }
  if (event.event === 'Progress') {
    return {
      ...progress,
      downloadedBytes: progress.downloadedBytes + Math.max(0, event.data.chunkLength),
      finished: false,
    };
  }
  return {
    ...progress,
    finished: true,
  };
};

export const updateErrorMessage = (error: unknown): string => {
  if (error instanceof Error) return error.message;
  if (typeof error === 'string') return error;
  if (
    error &&
    typeof error === 'object' &&
    'message' in error &&
    typeof error.message === 'string'
  ) {
    return error.message;
  }

  try {
    const serialized = JSON.stringify(error);
    if (serialized && serialized !== '{}') return serialized;
  } catch {
    // Ignore serialization failures and fall through to the generic message.
  }

  return 'Unknown updater error.';
};

export const isUpdaterUnavailableError = (error: unknown): boolean => {
  const message = updateErrorMessage(error).toLowerCase();
  return (
    message.includes('plugin:updater') ||
    (message.includes('updater') && message.includes('not configured')) ||
    message.includes('unknown command') ||
    message.includes('not found') ||
    message.includes('permission')
  );
};

const defaultGetPlatform = async (): Promise<string> => {
  if (!isDesktopPlatform()) return 'web';
  const capabilities = await getDesktopPerformanceCapabilities();
  return capabilities.platform;
};

const defaultInAppCheck = async (): Promise<InAppUpdateHandle | null> => {
  const updater = await import('@tauri-apps/plugin-updater');
  return updater.check() as Promise<InAppUpdateHandle | null>;
};

const latestReleaseUrlForVersion = (version: string): string =>
  `${SYNARA_RELEASES_URL}/tag/v${version.replace(/^v/i, '')}`;

const fetchLatestJson = async (fetchImpl: typeof fetch): Promise<LatestJson> => {
  const response = await fetchImpl(SYNARA_LATEST_UPDATE_METADATA_URL, {
    cache: 'no-store',
  });
  if (!response.ok) {
    throw new Error(`GitHub release metadata request failed with HTTP ${response.status}.`);
  }
  return response.json() as Promise<LatestJson>;
};

const checkLinuxUpdate = async ({
  currentVersion,
  fetchImpl,
}: {
  currentVersion: string;
  fetchImpl: typeof fetch;
}): Promise<DesktopUpdateCheckResult> => {
  const metadata = await fetchLatestJson(fetchImpl);
  const latestVersion = typeof metadata.version === 'string' ? metadata.version : '';
  if (!latestVersion) {
    throw new Error('GitHub release metadata did not include a version.');
  }

  const releaseUrl = latestReleaseUrlForVersion(latestVersion);
  if (!isNewerVersion(latestVersion, currentVersion)) {
    return {
      status: 'up-to-date',
      platform: 'linux',
      currentVersion,
      version: latestVersion,
      releaseUrl,
    };
  }

  return {
    status: 'available',
    platform: 'linux',
    currentVersion,
    version: latestVersion,
    date: typeof metadata.pub_date === 'string' ? metadata.pub_date : undefined,
    releaseUrl,
    packageManagerHint:
      'Update with apt upgrade on Debian/Ubuntu/Pop!_OS, or paru -Syu / pacman -Syu on Arch, after syncing the Synara repository.',
  };
};

const checkInAppUpdate = async ({
  platform,
  currentVersion,
  inAppCheck,
}: {
  platform: InAppUpdaterPlatform;
  currentVersion: string;
  inAppCheck: () => Promise<InAppUpdateHandle | null>;
}): Promise<DesktopUpdateCheckResult> => {
  const update = await inAppCheck();
  if (!update) {
    return {
      status: 'up-to-date',
      platform,
      currentVersion,
      version: currentVersion,
      releaseUrl: latestReleaseUrlForVersion(currentVersion),
    };
  }

  return {
    status: 'available',
    platform,
    currentVersion: update.currentVersion || currentVersion,
    version: update.version,
    date: update.date,
    body: update.body,
    releaseUrl: latestReleaseUrlForVersion(update.version),
    inAppUpdate: update,
  };
};

export const checkDesktopUpdate = async ({
  currentVersion = APP_VERSION,
  fetchImpl = fetch,
  getPlatform = defaultGetPlatform,
  inAppCheck = defaultInAppCheck,
  supportsUpdater = supportsPlatformUpdater,
}: DesktopUpdateCheckOptions = {}): Promise<DesktopUpdateCheckResult> => {
  const platform = normalizeDesktopUpdaterPlatform(await getPlatform());

  if (platform === 'unsupported') {
    return {
      status: 'unavailable',
      platform,
      currentVersion,
      message:
        'Synara desktop updates are only available in macOS, Windows and Linux desktop builds.',
    };
  }

  if (platform === 'linux') {
    return checkLinuxUpdate({ currentVersion, fetchImpl });
  }

  if (!supportsUpdater()) {
    return {
      status: 'unavailable',
      platform,
      currentVersion,
      message: 'In-app updates are available in signed release builds only.',
    };
  }

  try {
    return await checkInAppUpdate({ platform, currentVersion, inAppCheck });
  } catch (error) {
    if (isUpdaterUnavailableError(error)) {
      return {
        status: 'unavailable',
        platform,
        currentVersion,
        message: 'In-app updates are available in signed release builds only.',
      };
    }
    throw error;
  }
};

export type InstallUpdateDeps = {
  /** Downloads, verifies and installs `version` in place (macOS). */
  installInPlace?: (
    version: string,
    onEvent?: (event: DesktopUpdateDownloadEvent) => void
  ) => Promise<void>;
  relaunch?: () => Promise<void>;
};

// The plugin's macOS install replaces the whole Synara.app directory, which
// the Dock then treats as a different app, so a pinned tile stops matching the
// running app. `desktop_updater_install` swaps only the bundle's Contents,
// keeping the directory the Dock remembers.
const defaultInstallInPlace: NonNullable<InstallUpdateDeps['installInPlace']> = async (
  version,
  onEvent
) => {
  const { Channel } = await import('@tauri-apps/api/core');
  const channel = new Channel<DesktopUpdateDownloadEvent>((event) => onEvent?.(event));
  const result = await invokeDesktopWithAvailability('desktop_updater_install', {
    expectedVersion: version,
    onEvent: channel,
  });
  if (!result.available) {
    throw new Error('In-app updates are available in signed release builds only.');
  }
};

const defaultRelaunch = async (): Promise<void> => {
  const processPlugin = await import('@tauri-apps/plugin-process');
  await processPlugin.relaunch();
};

// macOS installs through `desktop_updater_install`. Windows keeps the plugin's
// NSIS/MSI install, which closes the app itself once it starts, so the
// relaunch below only runs on macOS.
export const installUpdateAndRelaunch = async (
  update: InAppUpdateHandle,
  platform: InAppUpdaterPlatform,
  onEvent?: (event: DesktopUpdateDownloadEvent) => void,
  { installInPlace = defaultInstallInPlace, relaunch = defaultRelaunch }: InstallUpdateDeps = {}
): Promise<void> => {
  if (platform === 'macos') {
    await installInPlace(update.version, onEvent);
  } else {
    await update.downloadAndInstall(onEvent);
  }
  await relaunch();
};
