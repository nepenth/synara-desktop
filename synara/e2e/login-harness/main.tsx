// The production app shell signed out: config, well-known discovery, spec
// versions and native login-flow IPC are answered by fixtures so the login
// screen renders exactly as it ships. `?delay=` slows discovery; hosts
// starting with `down.` are unreachable.
import React from 'react';
import { createRoot } from 'react-dom/client';
import { enableMapSet } from 'immer';
import { configClass, varsClass } from 'folds';
import 'folds/dist/style.css';
import '@fontsource-variable/inter';
import '../../src/index.css';
import App from '../../src/app/pages/App';
import '../../src/app/i18n';
import { setSessionBootstrapResult } from '../../src/app/state/sessionBootstrap';

enableMapSet();

const query = new URLSearchParams(location.search);
const delayMs = Number(query.get('delay') ?? 150);
const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

const discoveryRequests: string[] = [];
const realFetch = window.fetch.bind(window);
window.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
  const url = new URL(
    typeof input === 'string' ? input : input instanceof URL ? input.href : input.url,
    location.href
  );
  if (url.pathname.endsWith('/config.json')) {
    return json({
      defaultHomeserver: 0,
      homeserverList: ['matrix.example.test', 'agents.example.test'],
      allowCustomHomeservers: true,
    });
  }
  if (url.pathname === '/.well-known/matrix/client') {
    discoveryRequests.push(url.host);
    await sleep(delayMs);
    if (url.host.startsWith('down.')) throw new TypeError('Failed to fetch');
    return json({ 'm.homeserver': { base_url: `https://${url.host}` } });
  }
  if (url.pathname === '/_matrix/client/versions') {
    await sleep(delayMs);
    if (url.host.startsWith('down.')) throw new TypeError('Failed to fetch');
    return json({ versions: ['v1.11', 'v1.12'], unstable_features: {} });
  }
  return realFetch(input, init);
};

let callbackId = 0;
const fixtures: Record<string, unknown> = {
  matrix_login_flows: { flows: [{ kind: 'password', matrixType: 'm.login.password' }] },
  'plugin:window|is_maximized': false,
};
Object.assign(window, {
  __SYNARA_DESKTOP__: { platform: 'tauri', os: query.get('os') ?? 'linux' },
  __TAURI_INTERNALS__: {
    metadata: {
      currentWindow: { label: 'main' },
      currentWebview: { label: 'main', windowLabel: 'main' },
    },
    transformCallback: () => {
      callbackId += 1;
      return callbackId;
    },
    invoke: async (command: string) => {
      if (command in fixtures) {
        if (command === 'matrix_login_flows') await sleep(delayMs);
        return fixtures[command];
      }
      if (command.startsWith('plugin:event|')) return callbackId;
      throw new Error(`fixture: ${command} unavailable`);
    },
  },
  __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => undefined },
  synaraDiscoveryRequests: discoveryRequests,
});
setSessionBootstrapResult({ source: 'none' });
// Signed-out routes follow the system color scheme (emulate it in Playwright).
document.body.classList.add(configClass, varsClass);
createRoot(document.getElementById('root')!).render(<App />);
