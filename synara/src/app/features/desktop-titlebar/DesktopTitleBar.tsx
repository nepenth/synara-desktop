import React, { useCallback, useEffect, useRef, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Box, Icon, IconButton, Icons } from 'folds';
import classNames from 'classnames';

import { ContainerColor } from '../../styles/ContainerColor.css';
import * as depthCss from '../../styles/Depth.css';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import { isLinuxOS } from '../../utils/user-agent';
import * as css from './DesktopTitleBar.css';
import { observeNativeMaximizedState } from './nativeMaximizedState';

/** A persistent drag strip and window controls for the borderless Linux shell.
 * macOS and Windows use their native titlebars.
 */
export function useDesktopTitleBarVisible(): boolean {
  return isSynaraDesktop() && isLinuxOS();
}

// Drawn on the folds 24-unit grid with the Cross icon's 1.5-unit line, and
// rendered through `Icon` so all three window controls share one size.
const MaximizeIconSrc = () => (
  <path
    fillRule="evenodd"
    clipRule="evenodd"
    d="M5 5H19V19H5V5ZM6.5 6.5V17.5H17.5V6.5H6.5Z"
    fill="currentColor"
  />
);

const RestoreIconSrc = () => (
  <path
    fillRule="evenodd"
    clipRule="evenodd"
    d="M8 5H19V16H16.5V14.5H17.5V6.5H9.5V7.5H8V5ZM5 8.5H15.5V19H5V8.5ZM6.5 10V17.5H14V10H6.5Z"
    fill="currentColor"
  />
);

export function DesktopTitleBar() {
  const [maximized, setMaximized] = useState(false);
  const visible = useDesktopTitleBarVisible();
  const observation = useRef<ReturnType<typeof observeNativeMaximizedState> | undefined>(undefined);
  useEffect(() => {
    if (!visible) return undefined;
    const current = observeNativeMaximizedState(getCurrentWindow(), setMaximized);
    observation.current = current;
    return () => {
      current.dispose();
      observation.current = undefined;
    };
  }, [visible]);
  const toggleMaximize = useCallback(() => {
    const current = observation.current;
    void invokeDesktopWithAvailability<boolean>('desktop_window_toggle_maximize').then(() => {
      // Read current authority; the command result may predate another native resize.
      void current?.refresh();
    });
  }, []);

  if (!visible) return null;

  return (
    <Box
      className={classNames(ContainerColor({ variant: 'Background' }), css.TitleBar)}
      aria-label="Window title bar"
      data-tauri-drag-region
      data-synara-window-drag
      alignItems="Center"
      gap="100"
    >
      <Box
        grow="Yes"
        alignItems="Center"
        data-tauri-drag-region
        data-synara-window-drag
        className={css.DragRegion}
      />
      <Box shrink="No" alignItems="Center" gap="100" data-synara-window-controls>
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
          <Icon size="100" src={maximized ? RestoreIconSrc : MaximizeIconSrc} />
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
