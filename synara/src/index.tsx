/* eslint-disable import/first */
import React from 'react';
import { createRoot } from 'react-dom/client';
import { enableMapSet } from 'immer';
import '@fontsource/inter/variable.css';
import 'folds/dist/style.css';
import { configClass, varsClass } from 'folds';

enableMapSet();

import './index.css';

import { trimTrailingSlash } from './app/utils/common';
import App from './app/pages/App';

// import i18n (needs to be bundled ;))
import './app/i18n';
import { platformSessionStore } from './app/platform';
import { initializeSessionBootstrap } from './app/state/sessionBootstrap';

document.body.classList.add(configClass, varsClass);

const registerServiceWorker = () => {
  if (!('serviceWorker' in navigator)) return;

  const swUrl =
    import.meta.env.MODE === 'production'
      ? `${trimTrailingSlash(import.meta.env.BASE_URL)}/sw.js`
      : `/dev-sw.js?dev-sw`;

  void navigator.serviceWorker.register(swUrl);
};

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
    registerServiceWorker();
    mountApp();
  });
