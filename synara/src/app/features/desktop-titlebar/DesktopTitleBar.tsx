import React, { useCallback, useState } from 'react';
import { Box, Icon, IconButton, Icons, config } from 'folds';
import classNames from 'classnames';

import { ContainerColor } from '../../styles/ContainerColor.css';
import * as depthCss from '../../styles/Depth.css';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import { isLinuxOS } from '../../utils/user-agent';
import * as css from './DesktopTitleBar.css';

/**
 * In-app window chrome for Linux borderless mode (`decorations: false`).
 * Renders only on the Linux desktop shell: a drag region plus native
 * minimize/maximize/close controls. macOS keeps overlay traffic lights (no
 * custom strip) and other platforms keep their native decorations.
 */
export function useDesktopTitleBarVisible(): boolean {
  return isSynaraDesktop() && isLinuxOS();
}

function MaximizeIcon({ maximized }: { maximized: boolean }) {
  return (
    <svg
      width="12"
      height="12"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      aria-hidden
    >
      {maximized ? (
        <path d="M9 5h12v12M5 9h12v12H5z" />
      ) : (
        <rect x="5" y="5" width="14" height="14" rx="1" />
      )}
    </svg>
  );
}

export function DesktopTitleBar() {
  const [maximized, setMaximized] = useState(false);
  const visible = useDesktopTitleBarVisible();
  const toggleMaximize = useCallback(() => {
    void invokeDesktopWithAvailability<boolean>('desktop_window_toggle_maximize').then((result) => {
      if (result.available && typeof result.value === 'boolean') {
        setMaximized(result.value);
      }
    });
  }, []);

  if (!visible) return null;

  return (
    <Box
      className={classNames(ContainerColor({ variant: 'Background' }), css.TitleBar)}
      alignItems="Center"
      gap="100"
      style={{ padding: `0 ${config.space.S200}` }}
    >
      <Box
        grow="Yes"
        alignItems="Center"
        data-tauri-drag-region
        onDoubleClick={toggleMaximize}
        className={css.DragRegion}
      />
      <Box shrink="No" alignItems="Center" gap="100">
        <IconButton
          size="300"
          radii="300"
          aria-label="Minimize"
          title="Minimize"
          className={depthCss.quietInteractiveSurface}
          onClick={() => {
            void invokeDesktopWithAvailability('desktop_window_minimize');
          }}
        >
          <Icon size="100" src={Icons.Minus} />
        </IconButton>
        <IconButton
          size="300"
          radii="300"
          aria-label={maximized ? 'Restore' : 'Maximize'}
          title={maximized ? 'Restore' : 'Maximize'}
          className={depthCss.quietInteractiveSurface}
          onClick={toggleMaximize}
        >
          <MaximizeIcon maximized={maximized} />
        </IconButton>
        <IconButton
          size="300"
          radii="300"
          aria-label="Close"
          title="Close"
          className={depthCss.quietInteractiveSurface}
          onClick={() => {
            void invokeDesktopWithAvailability('desktop_window_close');
          }}
        >
          <Icon size="100" src={Icons.Cross} />
        </IconButton>
      </Box>
    </Box>
  );
}
