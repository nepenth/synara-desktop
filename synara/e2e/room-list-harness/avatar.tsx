// Real avatar component and media hook; only native media IPC is deterministic.
import React, { useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { Avatar, configClass, varsClass } from 'folds';
import 'folds/dist/style.css';
import avatarImageUrl from '../../../assets/branding/synara-app-icon-small.png';
import { UserAvatar } from '../../src/app/components/user-avatar';
import { MatrixClientProvider } from '../../src/app/hooks/useMatrixClient';
import { useNativeMatrixMediaSrc } from '../../src/app/hooks/useNativeMatrixMediaSrc';
import type { MatrixClient } from '../../src/client/initMatrix';
import {
  clearMediaObjectUrlCaches,
  getClientMediaObjectUrlCache,
} from '../../src/app/matrix/mediaObjectUrlCache';

const pressure = new URLSearchParams(location.search).has('pressure');
const recovery = new URLSearchParams(location.search).has('recovery');
let downloads = 0;
const clientA = {} as MatrixClient;
const clientB = {} as MatrixClient;
window.__SYNARA_DESKTOP__ = {
  platform: 'tauri',
  invoke: async <T,>(command: string): Promise<T> => {
    if (command !== 'matrix_media_download') throw new Error(`Unexpected command ${command}`);
    downloads += 1;
    const downloadNumber = downloads;
    window.dispatchEvent(new Event('avatar-download'));
    await new Promise((resolve) => setTimeout(resolve, 120));
    if (recovery && downloadNumber === 1) {
      document.body.dataset.downloadFailed = 'true';
      throw new Error('First avatar download fails for recovery regression');
    }
    const bytes = new Uint8Array(await (await fetch(avatarImageUrl)).arrayBuffer());
    return { bytes: Array.from(bytes) } as T;
  },
};

function AvatarRow({ uri }: { uri: string }) {
  const src = useNativeMatrixMediaSrc(uri);
  const [firstSrc] = useState(src);
  return (
    <div
      data-testid="avatar-row"
      data-first-src={firstSrc ?? ''}
      data-current-src={src ?? ''}
      style={{ display: 'flex', alignItems: 'center', gap: 12, padding: 24 }}
    >
      <Avatar size="500" radii="400">
        <UserAvatar
          userId="@sender:example.test"
          src={uri}
          alt="Sender avatar"
          renderFallback={() => 'S'}
        />
      </Avatar>
      <span>Sender · cached native avatar</span>
    </div>
  );
}

function Fixture() {
  const [room, setRoom] = useState('A');
  const [accountB, setAccountB] = useState(false);
  const [uri, setUri] = useState(
    recovery ? 'timeline-media-recovery' : 'mxc://example.test/avatar-a'
  );
  const [retryShown, setRetryShown] = useState(false);
  const [count, setCount] = useState(0);
  const [primed, setPrimed] = useState(false);
  useEffect(() => {
    if (!pressure || room !== 'B' || primed) return;
    let cancelled = false;
    void (async () => {
      const cache = getClientMediaObjectUrlCache(clientA);
      // A is oldest among 128 idle images. B remains active until returning A.
      for (let index = 0; index < 127; index += 1) {
        const lease = cache.acquire(`pressure-${index}`, async () => new Blob(['image']));
        await lease.promise;
        lease.release();
      }
      if (!cancelled) setPrimed(true);
    })();
    return () => {
      cancelled = true;
    };
  }, [room, primed]);
  useEffect(() => {
    const changed = () => setCount(downloads);
    window.addEventListener('avatar-download', changed);
    changed(); // Commit-phase media acquisition can precede this passive subscription.
    return () => window.removeEventListener('avatar-download', changed);
  }, []);
  return (
    <main className={`${configClass} ${varsClass}`} style={{ fontFamily: 'sans-serif' }}>
      <h1>Avatar room return proof</h1>
      <p>Cold load performs native IPC. Returning to Room A keeps the same decoded blob URL.</p>
      <button type="button" onClick={() => setRoom(room === 'A' ? 'B' : 'A')}>
        Switch room
      </button>
      <button type="button" onClick={() => setUri('mxc://example.test/avatar-new')}>
        Change avatar
      </button>
      <button
        type="button"
        onClick={() => {
          clearMediaObjectUrlCaches();
          setAccountB(!accountB);
        }}
      >
        Switch account
      </button>
      {recovery && (
        <button type="button" onClick={() => setRetryShown(!retryShown)}>
          {retryShown ? 'Hide retry avatar' : 'Show retry avatar'}
        </button>
      )}
      <div data-testid="cache-pressure">{primed ? 'Primed' : 'Not primed'}</div>
      <p>
        Native downloads: <span data-testid="downloads">{count}</span>
      </p>
      <MatrixClientProvider value={accountB ? clientB : clientA}>
        <div data-testid="room">Room {room}</div>
        {room === 'A' ? (
          <AvatarRow key="A" uri={uri} />
        ) : pressure ? (
          <AvatarRow key="B" uri="mxc://example.test/avatar-b" />
        ) : (
          <p>No avatar in this room.</p>
        )}
        {retryShown && <AvatarRow key="retry" uri={uri} />}
      </MatrixClientProvider>
    </main>
  );
}
createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <Fixture />
  </React.StrictMode>
);
