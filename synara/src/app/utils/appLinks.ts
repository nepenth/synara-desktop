import { useEffect } from 'react';
import type { MouseEvent as ReactMouseEvent } from 'react';
import packageInfo from '../../../package.json';
import { isDesktopPlatform, openPlatformExternalUrl } from '../platform';

export const APP_VERSION = packageInfo.version;
export const SYNARA_SOURCE_CODE_URL = 'https://github.com/nepenth/synara-desktop';
export const SYNARA_RELEASES_URL = `${SYNARA_SOURCE_CODE_URL}/releases`;
export const SYNARA_PROJECT_URL = `${SYNARA_SOURCE_CODE_URL}#readme`;
export const MATRIX_URL = 'https://matrix.org';
export const DESKTOP_EXTERNAL_LINK_CLICK_OPTIONS: AddEventListenerOptions = { capture: true };

export const openExternalUrl = async (url: string): Promise<boolean> => {
  if (isDesktopPlatform()) {
    return openPlatformExternalUrl(url);
  }

  if (typeof window === 'undefined') return false;
  return window.open(url, '_blank', 'noopener,noreferrer') !== null;
};

type ExternalClickEvent = Pick<
  MouseEvent | ReactMouseEvent<HTMLElement>,
  'altKey' | 'button' | 'ctrlKey' | 'defaultPrevented' | 'metaKey' | 'shiftKey'
>;

const shouldIgnoreAnchorClick = (evt: ExternalClickEvent): boolean =>
  evt.defaultPrevented ||
  evt.button !== 0 ||
  evt.metaKey ||
  evt.ctrlKey ||
  evt.shiftKey ||
  evt.altKey;

const isAppRelativeHref = (href: string): boolean => href.startsWith('/') || href.startsWith('#');

const getCurrentAnchorHref = (evt: ReactMouseEvent<HTMLElement>): string | undefined => {
  if (typeof HTMLAnchorElement === 'undefined') return undefined;
  return evt.currentTarget instanceof HTMLAnchorElement ? evt.currentTarget.href : undefined;
};

export const openExternalUrlFromClick = (evt: ReactMouseEvent<HTMLElement>, url: string): void => {
  if (!isDesktopPlatform() || shouldIgnoreAnchorClick(evt)) return;

  if (!url || isAppRelativeHref(url)) return;
  const href = getCurrentAnchorHref(evt) ?? url;
  evt.preventDefault();
  void openExternalUrl(href);
};

const getClickedAnchor = (target: EventTarget | null): HTMLAnchorElement | undefined => {
  if (!(target instanceof Element)) return undefined;
  return target.closest<HTMLAnchorElement>('a[href]') ?? undefined;
};

export const openDesktopExternalAnchorFromClick = (evt: MouseEvent): void => {
  if (!isDesktopPlatform() || shouldIgnoreAnchorClick(evt)) return;

  const anchor = getClickedAnchor(evt.target);
  if (!anchor) return;

  const rawHref = anchor.getAttribute('href') ?? '';
  if (!rawHref || isAppRelativeHref(rawHref)) return;

  const href = anchor.href;
  if (!href) return;

  evt.preventDefault();
  void openExternalUrl(href);
};

export const useDesktopExternalLinkInterceptor = (): void => {
  useEffect(() => {
    if (!isDesktopPlatform()) return undefined;

    document.addEventListener(
      'click',
      openDesktopExternalAnchorFromClick,
      DESKTOP_EXTERNAL_LINK_CLICK_OPTIONS
    );
    return () => {
      document.removeEventListener(
        'click',
        openDesktopExternalAnchorFromClick,
        DESKTOP_EXTERNAL_LINK_CLICK_OPTIONS
      );
    };
  }, []);
};
