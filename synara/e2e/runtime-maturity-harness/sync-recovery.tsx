// Mount the production startup/recovery UI. Only native IPC is controlled.
import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { MemoryRouter } from 'react-router-dom';
import { configClass, varsClass } from 'folds';
import 'folds/dist/style.css';
import '../../src/index.css';
import { darkTheme } from '../../src/colors.css';
import { ClientRoot } from '../../src/app/pages/client/ClientRoot';
import { SpecVersionsLoader } from '../../src/app/components/SpecVersionsLoader';
import { useSpecVersions } from '../../src/app/hooks/useSpecVersions';
import { setSessionBootstrapResult } from '../../src/app/state/sessionBootstrap';
import type { NativeReadiness } from '../../src/app/features/native-client/nativeClientFacade';

export function mountSyncRecovery() {
  const query = new URLSearchParams(location.search);
  let readiness = query.get('sync-recovery') as NativeReadiness;
  let recoveries = 0;
  let restores = 0;
  let roomReads = 0;
  let logouts = 0;
  let updateControls: () => void = () => undefined;
  let finishRecovery: ((success: boolean, connected?: boolean) => void) | undefined;
  const identity = {
    userId: '@fixture:example.invalid',
    deviceId: 'FIXTURE',
    homeserverUrl: `${location.origin}/mock-homeserver`,
    sessionGeneration: 1,
  };
  const syncStatus = () => ({
    readiness,
    sessionGeneration: 1,
    offlineModeEnabled: true,
    failureDiagnosticId: readiness === 'failed' ? 'p4.1-sync-service-error' : null,
    slidingSyncCapable: true,
  });
  Object.assign(window, {
    __TAURI_INTERNALS__: {
      invoke: async (command: string) => {
        if (command === 'matrix_restore_session') {
          restores += 1;
          updateControls();
          if (query.has('restore-error') && restores <= 2) {
            throw new Error('Controlled native restore failure');
          }
          return identity;
        }
        if (command === 'matrix_session_snapshot') return { status: 'logged_in', ...identity };
        if (command === 'matrix_sync_status') return syncStatus();
        if (command === 'matrix_room_list_snapshot') {
          roomReads += 1;
          if (query.has('startup-error') && roomReads === 2) {
            throw new Error('Controlled startup read failure');
          }
          return { sessionGeneration: 1, rooms: [] };
        }
        if (command === 'matrix_sync_recover') {
          recoveries += 1;
          const result = new Promise<void>((resolve, reject) => {
            finishRecovery = (success, connected = true) => {
              finishRecovery = undefined;
              if (success) {
                if (connected) readiness = 'running';
                resolve();
              } else {
                reject(new Error('Controlled native recovery failure'));
              }
              updateControls();
            };
          });
          updateControls();
          await result;
          return syncStatus();
        }
        if (command === 'matrix_logout') {
          logouts += 1;
          updateControls();
          return { status: 'logged_out' };
        }
        // Diagnostic logging is part of the normal failed IPC route.
        if (command === 'desktop_append_log') return true;
        throw new Error(`Unexpected fixture command: ${command}`);
      },
    },
  });
  setSessionBootstrapResult({
    source: 'native',
    session: {
      userId: identity.userId,
      deviceId: identity.deviceId,
      baseUrl: identity.homeserverUrl,
      sessionGeneration: String(identity.sessionGeneration),
    },
  });
  document.body.classList.add(configClass, varsClass, darkTheme, 'dark-theme');

  function NativeReady() {
    const versions = useSpecVersions();
    return (
      <>
        <p>Native client ready</p>
        <output data-testid="server-versions">{versions.versions.join(',')}</output>
      </>
    );
  }

  function RecoveryFixture() {
    const [, update] = useState(0);
    updateControls = () => update((value) => value + 1);
    if (query.has('pre-login-versions')) {
      return (
        <SpecVersionsLoader
          baseUrl={identity.homeserverUrl}
          fallback={() => <p>Checking login server</p>}
          error={(_error, retry) => <button onClick={retry}>Retry login server check</button>}
        >
          {(versions) => <p>Login server validated: {versions.versions.join(',')}</p>}
        </SpecVersionsLoader>
      );
    }
    return (
      <MemoryRouter>
        <section
          aria-label="Native sync fixture controls"
          style={{ position: 'fixed', top: 8, right: 8, zIndex: 10000 }}
        >
          <output data-testid="native-recovery-count">{recoveries}</output>
          <output data-testid="native-restore-count">{restores}</output>
          <output data-testid="native-readiness">{readiness}</output>
          <output data-testid="native-logout-count">{logouts}</output>
          <button disabled={!finishRecovery} onClick={() => finishRecovery?.(true)}>
            Complete native recovery
          </button>
          <button disabled={!finishRecovery} onClick={() => finishRecovery?.(false)}>
            Fail native recovery
          </button>
          <button disabled={!finishRecovery} onClick={() => finishRecovery?.(true, false)}>
            Acknowledge recovery without readiness
          </button>
          <button
            onClick={() => {
              readiness = 'offline';
              updateControls();
            }}
          >
            Lose native connection
          </button>
        </section>
        <ClientRoot>
          <NativeReady />
        </ClientRoot>
      </MemoryRouter>
    );
  }
  createRoot(document.getElementById('root')!).render(<RecoveryFixture />);
}
