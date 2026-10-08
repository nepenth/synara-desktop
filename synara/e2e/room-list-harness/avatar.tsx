// Real avatar component. Display uses a synara-media URL and does not download bytes.
import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { Avatar, configClass, varsClass } from 'folds';
import 'folds/dist/style.css';
import { UserAvatar } from '../../src/app/components/user-avatar';
import { setNativeIdentity } from '../../src/app/state/nativeIdentity';
import { useNativeMatrixMediaSrc } from '../../src/app/hooks/useNativeMatrixMediaSrc';

const accountA = { userId: '@a:example.test' };
const accountBIdentity = { userId: '@b:example.test' };
setNativeIdentity(accountA);
window.__SYNARA_DESKTOP__ = {
  platform: 'tauri',
  invoke: async () => {
    throw new Error('avatar display must not download file bytes');
  },
};
window.__TAURI_INTERNALS__ = {
  convertFileSrc: (filePath: string, protocol = 'asset') =>
    `${protocol}://localhost/${encodeURIComponent(filePath)}`,
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
      <span>Sender · protocol avatar</span>
    </div>
  );
}

function Fixture() {
  const [room, setRoom] = useState('A');
  const [accountB, setAccountB] = useState(false);
  const [uri, setUri] = useState('mxc://example.test/avatar-a');
  return (
    <main className={`${configClass} ${varsClass}`} style={{ fontFamily: 'sans-serif' }}>
      <h1>Avatar room return proof</h1>
      <button type="button" onClick={() => setRoom(room === 'A' ? 'B' : 'A')}>
        Switch room
      </button>
      <button type="button" onClick={() => setUri('mxc://example.test/avatar-new')}>
        Change avatar
      </button>
      <button
        type="button"
        onClick={() => {
          setNativeIdentity(accountB ? accountA : accountBIdentity);
          setAccountB(!accountB);
        }}
      >
        Switch account
      </button>
      <div data-testid="room">Room {room}</div>
      {room === 'A' ? <AvatarRow key="A" uri={uri} /> : <p>No avatar in this room.</p>}
    </main>
  );
}
createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <Fixture />
  </React.StrictMode>
);
