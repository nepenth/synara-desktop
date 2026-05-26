import type { MouseEvent } from 'react';
import packageInfo from '../../../package.json';
import { isDesktopPlatform, openPlatformExternalUrl } from '../platform';

export const APP_VERSION = packageInfo.version;
export const SYNARA_SOURCE_CODE_URL = 'https://github.com/nepenth/synara-desktop';
export const SYNARA_RELEASES_URL = `${SYNARA_SOURCE_CODE_URL}/releases`;
export const SYNARA_PROJECT_URL = `${SYNARA_SOURCE_CODE_URL}#readme`;
export const MATRIX_URL = 'https://matrix.org';

export const openExternalUrlFromClick = (evt: MouseEvent<HTMLElement>, url: string): void => {
  if (!isDesktopPlatform()) return;

  evt.preventDefault();
  void openPlatformExternalUrl(url);
};
