import React, { useEffect } from 'react';
import { Provider as JotaiProvider } from 'jotai';
import { OverlayContainerProvider, PopOutContainerProvider, TooltipContainerProvider } from 'folds';
import { RouterProvider } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';

import { ClientConfigLoader } from '../components/ClientConfigLoader';
import { ClientConfigProvider } from '../hooks/useClientConfig';
import { ConfigConfigError, ConfigConfigLoading } from './ConfigConfig';
import { createRouter } from './Router';
import { DesktopTitleBar } from '../features/desktop-titlebar/DesktopTitleBar';
import { ScreenSizeProvider, useScreenSize } from '../hooks/useScreenSize';
import { useCompositionEndTracking } from '../hooks/useComposingCheck';
import { useDesktopExternalLinkInterceptor } from '../utils/appLinks';
import { useMotionPolicy } from '../hooks/useMotionPolicy';
import { reportDesktopRenderingStarted } from '../utils/desktopRendering';
import { useNativeBeforeQuit } from '../utils/beforeQuit';
import '../styles/FocusRing.css';
import '../styles/motion.css';

const queryClient = new QueryClient();

function App() {
  const screenSize = useScreenSize();
  useCompositionEndTracking();
  useDesktopExternalLinkInterceptor();
  useMotionPolicy();
  useNativeBeforeQuit();
  // The app shell is up: once it has drawn, the shell knows this start
  // worked and keeps the hardware acceleration mode for the next one.
  useEffect(() => reportDesktopRenderingStarted(), []);

  const portalContainer = document.getElementById('portalContainer') ?? undefined;

  return (
    <TooltipContainerProvider value={portalContainer}>
      <PopOutContainerProvider value={portalContainer}>
        <OverlayContainerProvider value={portalContainer}>
          <ScreenSizeProvider value={screenSize}>
            {/* One store for the title strip and the app, so the Tiling mode
                setting shows or hides the strip at once. */}
            <JotaiProvider>
              <DesktopTitleBar />
              <ClientConfigLoader
                fallback={() => <ConfigConfigLoading />}
                error={(err, retry, ignore) => (
                  <ConfigConfigError error={err} retry={retry} ignore={ignore} />
                )}
              >
                {(clientConfig) => (
                  <ClientConfigProvider value={clientConfig}>
                    <QueryClientProvider client={queryClient}>
                      <RouterProvider router={createRouter(clientConfig, screenSize)} />
                      <ReactQueryDevtools initialIsOpen={false} />
                    </QueryClientProvider>
                  </ClientConfigProvider>
                )}
              </ClientConfigLoader>
            </JotaiProvider>
          </ScreenSizeProvider>
        </OverlayContainerProvider>
      </PopOutContainerProvider>
    </TooltipContainerProvider>
  );
}

export default App;
