import { useEffect } from 'react';
import { getDesktopRenderingStatus } from '../utils/desktopRendering';
import {
  RENDER_MODE_ATTRIBUTE,
  WINDOW_ACTIVE_ATTRIBUTE,
  renderModeAttribute,
  windowActiveAttribute,
  writeRootAttribute,
} from '../utils/motion';

/**
 * Keeps the motion-budget attributes on <html> (see `utils/motion.ts`):
 * `data-window-active` from window focus and page visibility, and
 * `data-render-mode` from the shell's rendering setting. Mount once, at the
 * app root.
 *
 * WebKitGTK and WKWebView forward the native window's focus to the page, so
 * window `focus`/`blur` cover an unfocused Tauri window and
 * `visibilitychange` covers a minimised or hidden one, the same signals the
 * timeline uses for read receipts.
 */
export function useMotionPolicy(): void {
  useEffect(() => {
    const root = document.documentElement;
    let blurTimer: number | undefined;

    const update = () => {
      blurTimer = undefined;
      writeRootAttribute(
        root,
        WINDOW_ACTIVE_ATTRIBUTE,
        windowActiveAttribute({
          visibilityState: document.visibilityState,
          hasFocus: document.hasFocus(),
        })
      );
    };
    // Focus moving into an embedded frame blurs the window first; read
    // `hasFocus()` once focus has settled so that does not count as inactive.
    const updateAfterBlur = () => {
      if (blurTimer === undefined) blurTimer = window.setTimeout(update, 0);
    };

    update();
    window.addEventListener('focus', update);
    window.addEventListener('blur', updateAfterBlur);
    document.addEventListener('visibilitychange', update);
    return () => {
      if (blurTimer !== undefined) window.clearTimeout(blurTimer);
      window.removeEventListener('focus', update);
      window.removeEventListener('blur', updateAfterBlur);
      document.removeEventListener('visibilitychange', update);
      root.removeAttribute(WINDOW_ACTIVE_ATTRIBUTE);
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    // The mode is fixed for the life of the process, so one read is enough.
    void getDesktopRenderingStatus().then((status) => {
      if (!cancelled)
        writeRootAttribute(
          document.documentElement,
          RENDER_MODE_ATTRIBUTE,
          renderModeAttribute(status)
        );
    });
    return () => {
      cancelled = true;
    };
  }, []);
}
