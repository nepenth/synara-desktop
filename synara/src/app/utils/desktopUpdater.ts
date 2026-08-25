import { isDesktopPlatform, supportsPlatformUpdater } from '../platform';
import { APP_VERSION, SYNARA_RELEASES_URL } from './appLinks';
import { getDesktopPerformanceCapabilities } from './desktop';

export const SYNARA_LATEST_UPDATE_METADATA_URL =
  'https://github.com/nepenth/synara-desktop/releases/latest/download/latest.json';
export const DISMISSED_UPDATE_VERSION_KEY = 'synara.desktop.dismissedUpdateVersion';

export type DesktopUpdaterPlatform = 'macos' | 'linux' | 'unsupported';
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

export type MacosUpdateHandle = {
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
      platform: 'macos' | 'linux';
      currentVersion: string;
      version: string;
      date?: string;
      body?: string;
      releaseUrl: string;
      packageManagerHint?: string;
      macosUpdate?: MacosUpdateHandle;
    }
  | {
      status: 'up-to-date';
      platform: 'macos' | 'linux';
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
  macosCheck?: () => Promise<MacosUpdateHandle | null>;
  supportsUpdater?: () => boolean;
};

type LatestJson = {
  version?: unknown;
  pub_date?: unknown;
};

export const normalizeDesktopUpdaterPlatform = (platform: string): DesktopUpdaterPlatform => {
  const normalized = platform.toLowerCase();
  if (normalized === 'darwin' || normalized === 'macos') return 'macos';
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

const defaultMacosCheck = async (): Promise<MacosUpdateHandle | null> => {
  const updater = await import('@tauri-apps/plugin-updater');
  return updater.check() as Promise<MacosUpdateHandle | null>;
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

const checkMacosUpdate = async ({
  currentVersion,
  macosCheck,
}: {
  currentVersion: string;
  macosCheck: () => Promise<MacosUpdateHandle | null>;
}): Promise<DesktopUpdateCheckResult> => {
  const update = await macosCheck();
  if (!update) {
    return {
      status: 'up-to-date',
      platform: 'macos',
      currentVersion,
      version: currentVersion,
      releaseUrl: latestReleaseUrlForVersion(currentVersion),
    };
  }

  return {
    status: 'available',
    platform: 'macos',
    currentVersion: update.currentVersion || currentVersion,
    version: update.version,
    date: update.date,
    body: update.body,
    releaseUrl: latestReleaseUrlForVersion(update.version),
    macosUpdate: update,
  };
};

export const checkDesktopUpdate = async ({
  currentVersion = APP_VERSION,
  fetchImpl = fetch,
  getPlatform = defaultGetPlatform,
  macosCheck = defaultMacosCheck,
  supportsUpdater = supportsPlatformUpdater,
}: DesktopUpdateCheckOptions = {}): Promise<DesktopUpdateCheckResult> => {
  const platform = normalizeDesktopUpdaterPlatform(await getPlatform());

  if (platform === 'unsupported') {
    return {
      status: 'unavailable',
      platform,
      currentVersion,
      message: 'Synara desktop updates are only available in macOS and Linux desktop builds.',
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
    return await checkMacosUpdate({ currentVersion, macosCheck });
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

export const installMacosUpdateAndRelaunch = async (
  update: MacosUpdateHandle,
  onEvent?: (event: DesktopUpdateDownloadEvent) => void
): Promise<void> => {
  await update.downloadAndInstall(onEvent);
  const processPlugin = await import('@tauri-apps/plugin-process');
  await processPlugin.relaunch();
};
