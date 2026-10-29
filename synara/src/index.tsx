/* eslint-disable import/first */
import React from 'react';
import { createRoot } from 'react-dom/client';
import { enableMapSet } from 'immer';
import 'folds/dist/style.css';
import { configClass, varsClass } from 'folds';

enableMapSet();

import './index.css';

import { retireLegacyServiceWorkers } from './app/platform/legacyServiceWorkers';
import App from './app/pages/App';

// import i18n (needs to be bundled ;))
import './app/i18n';
import { platformSessionStore } from './app/platform';
import { initializeSessionBootstrap } from './app/state/sessionBootstrap';
import { isLinuxOS } from './app/utils/user-agent';

document.body.classList.add(configClass, varsClass);
// Lets index.css put the bundled Inter ahead of Linux's system-ui.
if (isLinuxOS()) document.documentElement.dataset.synaraOs = 'linux';

const mountApp = () => {
  const rootContainer = document.getElementById('root');

  if (rootContainer === null) {
    console.error('Root container element not found!');
    return;
  }

  const root = createRoot(rootContainer);
  root.render(<App />);
};

initializeSessionBootstrap({ nativeSessionStore: platformSessionStore })
  .catch(() => undefined)
  .finally(() => {
    void retireLegacyServiceWorkers(navigator, import.meta.env.BASE_URL, window.location.href);
    mountApp();
  });
