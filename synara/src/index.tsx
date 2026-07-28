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
import { pushActiveSessionToSW } from './sw-session';
import { platformSessionStore } from './app/platform';
import { getActiveSession, initializeSessionBootstrap } from './app/state/sessionBootstrap';
import {
  hasActiveNativeMatrixSession,
  restoreActiveNativeMatrixSession,
} from './app/state/nativeMatrixSession';

document.body.classList.add(configClass, varsClass);

const registerServiceWorker = () => {
  if (!('serviceWorker' in navigator)) return;

  const swUrl =
    import.meta.env.MODE === 'production'
      ? `${trimTrailingSlash(import.meta.env.BASE_URL)}/sw.js`
      : `/dev-sw.js?dev-sw`;

  const sendSessionToSW = () =>
    pushActiveSessionToSW(() => (hasActiveNativeMatrixSession() ? undefined : getActiveSession()));

  navigator.serviceWorker.register(swUrl).then(sendSessionToSW);
  navigator.serviceWorker.ready.then(sendSessionToSW);

  navigator.serviceWorker.addEventListener('message', (ev) => {
    const { type } = ev.data ?? {};

    if (type === 'requestSession') {
      sendSessionToSW();
    }
  });
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

restoreActiveNativeMatrixSession()
  .then((nativeIdentity) =>
    nativeIdentity
      ? undefined
      : initializeSessionBootstrap({ nativeSessionStore: platformSessionStore })
  )
  .catch(() => undefined)
  .finally(() => {
    registerServiceWorker();
    mountApp();
  });
