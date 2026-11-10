import { createContext, useContext, useEffect, useState } from 'react';

export const TABLET_BREAKPOINT = 1124;
export const MOBILE_BREAKPOINT = 750;

export enum ScreenSize {
  Desktop = 'Desktop',
  Tablet = 'Tablet',
  Mobile = 'Mobile',
}

export const getScreenSize = (width: number): ScreenSize => {
  if (width > TABLET_BREAKPOINT) return ScreenSize.Desktop;
  if (width > MOBILE_BREAKPOINT) return ScreenSize.Tablet;
  return ScreenSize.Mobile;
};

/**
 * The layout for a window `width` CSS pixels wide at page zoom `zoom` (1 at
 * 100%). Page zoom scales the root font size, not CSS pixels, so the window
 * measures the same at every zoom while everything sized in rem grows: at
 * 150% a 1200px window holds what an 800px one holds at 100%, and gets the
 * same layout, as browser zoom would give it.
 */
export const getZoomedScreenSize = (width: number, zoom: number): ScreenSize =>
  getScreenSize(Number.isFinite(zoom) && zoom > 0 ? width / zoom : width);

/**
 * A hidden 1rem-wide probe: its width follows the root font size (page zoom)
 * and its own `font-size: medium` is the unzoomed size, so their ratio is the
 * zoom. Observing it re-measures when only the zoom changes.
 */
const createZoomProbe = (doc: Document): HTMLElement => {
  const probe = doc.createElement('div');
  probe.setAttribute('aria-hidden', 'true');
  probe.setAttribute('data-screen-size-probe', '');
  probe.style.cssText =
    'position:fixed;top:0;left:0;width:1rem;height:0;overflow:hidden;visibility:hidden;pointer-events:none;font-size:medium;';
  doc.body.appendChild(probe);
  return probe;
};

const probeZoom = (probe: HTMLElement): number => {
  const rem = probe.getBoundingClientRect().width;
  const medium = parseFloat(getComputedStyle(probe).fontSize);
  return rem > 0 && medium > 0 ? rem / medium : 1;
};

/**
 * The window's layout size. Each webview measures its own `body` (the
 * window, not the screen), and page zoom counts: see `getZoomedScreenSize`.
 */
export const useScreenSize = (): ScreenSize => {
  const [size, setSize] = useState<ScreenSize>(getScreenSize(document.body.clientWidth));

  useEffect(() => {
    const probe = createZoomProbe(document);
    const update = () => setSize(getZoomedScreenSize(document.body.clientWidth, probeZoom(probe)));
    const observer = new ResizeObserver(update);
    observer.observe(document.body);
    observer.observe(probe);
    update();
    return () => {
      observer.disconnect();
      probe.remove();
    };
  }, []);

  return size;
};

const ScreenSizeContext = createContext<ScreenSize | null>(null);
export const ScreenSizeProvider = ScreenSizeContext.Provider;

export const useScreenSizeContext = (): ScreenSize => {
  const screenSize = useContext(ScreenSizeContext);
  if (screenSize === null) {
    throw new Error('Screen size not provided!');
  }
  return screenSize;
};
