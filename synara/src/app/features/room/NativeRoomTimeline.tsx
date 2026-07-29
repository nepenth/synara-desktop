import React, { ReactNode, useCallback, useEffect, useState } from 'react';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import { Modal500 } from '../../components/Modal500';
import { Settings, SettingsPages } from '../settings/Settings';
import { getNativeBackupStatus, NATIVE_BACKUP_CHANGED } from '../backup/nativeBackup';
import {
  getNativeSecretStorageStatus,
  NATIVE_SECRET_STORAGE_CHANGED,
} from '../secret-storage/nativeSecretStorage';

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
};

type NativeTimelineItem = {
  itemId: string;
  eventId: string;
  sender: string;
  type: string;
  body: string;
  originServerTs: number;
  decryptionState?: 'pending' | 'unavailable';
};

type NativeUtdStatus = {
  phase: 'idle' | 'recovering' | 'partial' | 'unavailable';
  pendingCount: number;
  unavailableCount: number;
  recoveredCount: number;
};

type NativeTimelineSnapshot = {
  sessionGeneration: number;
  roomId: string;
  isEncrypted: boolean;
  items: NativeTimelineItem[];
  hitStart: boolean;
  utd: NativeUtdStatus;
};

type NativeCryptoStatus = {
  sessionGeneration: number;
  encryptionEnabled: boolean;
  crossSigningState: 'unavailable' | 'not_set_up' | 'partial' | 'ready';
};

type NativeTimelineOwner = 'checking' | 'native' | 'legacy';

export function NativeRoomTimelineBoundary({
  roomId,
  legacyTimeline,
}: {
  roomId: string;
  legacyTimeline: ReactNode;
}) {
  const [owner, setOwner] = useState<NativeTimelineOwner>('checking');

  useEffect(() => {
    let disposed = false;
    const selectOwner = async () => {
      if (!isSynaraDesktop()) {
        setOwner('legacy');
        return;
      }
      const session = await invokeDesktopWithAvailability<NativeSessionSnapshot>(
        'matrix_session_snapshot'
      ).catch(() => undefined);
      if (disposed) return;
      setOwner(session?.available && session.value?.status === 'logged_in' ? 'native' : 'legacy');
    };
    void selectOwner();
    return () => {
      disposed = true;
    };
  }, []);

  if (owner === 'legacy') return legacyTimeline;
  if (owner === 'native') return <NativeRoomTimeline roomId={roomId} />;
  return <NativeTimelineStatus>Opening native timeline…</NativeTimelineStatus>;
}

function NativeRoomTimeline({ roomId }: { roomId: string }) {
  const [snapshot, setSnapshot] = useState<NativeTimelineSnapshot>();
  const [cryptoStatus, setCryptoStatus] = useState<NativeCryptoStatus>();
  const [cryptoStatusUnavailable, setCryptoStatusUnavailable] = useState(false);
  const [error, setError] = useState(false);
  const [paginating, setPaginating] = useState(false);
  const [recoverySettingsOpen, setRecoverySettingsOpen] = useState(false);
  const [recoveryGuidance, setRecoveryGuidance] = useState<
    'checking' | 'ready' | 'action_required' | 'unavailable'
  >('checking');
  const pendingUtdCount = snapshot?.utd.pendingCount ?? 0;
  const unavailableUtdCount = snapshot?.utd.unavailableCount ?? 0;

  const loadSnapshot = useCallback(
    async (command: string) => {
      const result = await invokeDesktopWithAvailability<NativeTimelineSnapshot>(command, {
        roomId,
      });
      if (!result.available || !result.value) throw new Error('Native timeline IPC unavailable');
      setSnapshot(result.value);
      setError(false);
    },
    [roomId]
  );

  useEffect(() => {
    let disposed = false;
    let pollId: number | undefined;
    const open = async () => {
      try {
        const result = await invokeDesktopWithAvailability<NativeTimelineSnapshot>(
          'matrix_timeline_open',
          { roomId }
        );
        if (disposed) return;
        if (!result.available || !result.value) throw new Error('Native timeline IPC unavailable');
        setSnapshot(result.value);
        setError(false);
        if (result.value.isEncrypted) {
          const crypto = await invokeDesktopWithAvailability<NativeCryptoStatus>(
            'matrix_crypto_status'
          );
          if (disposed) return;
          if (!crypto.available || !crypto.value) {
            setCryptoStatusUnavailable(true);
          } else {
            setCryptoStatus(crypto.value);
            setCryptoStatusUnavailable(false);
          }
        }
        pollId = window.setInterval(() => {
          void loadSnapshot('matrix_timeline_snapshot').catch(() => {
            if (!disposed) setError(true);
          });
        }, 1000);
      } catch {
        if (!disposed) setError(true);
      }
    };
    void open();
    return () => {
      disposed = true;
      if (pollId !== undefined) window.clearInterval(pollId);
    };
  }, [loadSnapshot, roomId]);

  useEffect(() => {
    if (pendingUtdCount + unavailableUtdCount === 0) return;
    let disposed = false;
    const refreshRecoveryGuidance = async () => {
      const [secretStorage, backup] = await Promise.all([
        getNativeSecretStorageStatus().catch(() => undefined),
        getNativeBackupStatus().catch(() => undefined),
      ]);
      if (disposed) return;
      if (!secretStorage || !backup) {
        setRecoveryGuidance('unavailable');
      } else if (secretStorage.action !== 'none' || backup.action !== 'none') {
        setRecoveryGuidance('action_required');
      } else {
        setRecoveryGuidance('ready');
      }
    };
    void refreshRecoveryGuidance();
    window.addEventListener(NATIVE_SECRET_STORAGE_CHANGED, refreshRecoveryGuidance);
    window.addEventListener(NATIVE_BACKUP_CHANGED, refreshRecoveryGuidance);
    return () => {
      disposed = true;
      window.removeEventListener(NATIVE_SECRET_STORAGE_CHANGED, refreshRecoveryGuidance);
      window.removeEventListener(NATIVE_BACKUP_CHANGED, refreshRecoveryGuidance);
    };
  }, [pendingUtdCount, unavailableUtdCount]);

  const paginateBackwards = async () => {
    if (paginating) return;
    setPaginating(true);
    try {
      const result = await invokeDesktopWithAvailability<NativeTimelineSnapshot>(
        'matrix_timeline_paginate',
        { roomId, dir: 'backwards' }
      );
      if (!result.available || !result.value) throw new Error('Native timeline IPC unavailable');
      setSnapshot(result.value);
      setError(false);
    } catch {
      setError(true);
    } finally {
      setPaginating(false);
    }
  };

  if (!snapshot && error) {
    return <NativeTimelineStatus>Native timeline is unavailable.</NativeTimelineStatus>;
  }
  if (!snapshot) {
    return <NativeTimelineStatus>Loading messages from Rust…</NativeTimelineStatus>;
  }

  return (
    <div
      data-timeline-owner="matrix-rust-sdk"
      style={{
        minHeight: 0,
        flex: '1 1 auto',
        overflowY: 'auto',
        padding: '16px 24px',
      }}
    >
      {snapshot.isEncrypted && (
        <p
          role="status"
          data-native-crypto-status={
            cryptoStatus?.encryptionEnabled ? cryptoStatus.crossSigningState : 'unavailable'
          }
          style={{
            margin: '0 0 12px',
            padding: '8px 12px',
            borderRadius: 6,
            background: 'rgba(127, 127, 127, 0.12)',
            fontSize: 13,
          }}
        >
          {cryptoStatus?.encryptionEnabled
            ? `End-to-end encrypted with Rust crypto${
                cryptoStatus.crossSigningState === 'ready'
                  ? '.'
                  : '; device verification setup is incomplete.'
              }`
            : cryptoStatusUnavailable
            ? 'Encrypted room; native crypto readiness could not be confirmed.'
            : 'Confirming native encryption readiness…'}
        </p>
      )}
      {snapshot.utd.phase !== 'idle' && (
        <div
          role="status"
          data-native-utd-phase={snapshot.utd.phase}
          style={{
            margin: '0 0 12px',
            padding: '10px 12px',
            borderRadius: 6,
            background: 'rgba(127, 127, 127, 0.12)',
            fontSize: 13,
          }}
        >
          {snapshot.utd.pendingCount > 0 && (
            <p style={{ margin: 0 }}>
              {snapshot.utd.pendingCount} encrypted message
              {snapshot.utd.pendingCount === 1 ? ' is' : 's are'} waiting for keys. Synara retries
              automatically when recovery data arrives.
            </p>
          )}
          {snapshot.utd.unavailableCount > 0 && (
            <p style={{ margin: snapshot.utd.pendingCount > 0 ? '6px 0 0' : 0 }}>
              {snapshot.utd.unavailableCount} message
              {snapshot.utd.unavailableCount === 1 ? ' is' : 's are'} currently unavailable on this
              device. Synara will keep checking for keys.
            </p>
          )}
          {snapshot.utd.recoveredCount > 0 && (
            <p style={{ margin: '6px 0 0' }}>
              {snapshot.utd.recoveredCount} message
              {snapshot.utd.recoveredCount === 1 ? ' has' : 's have'} been recovered.
            </p>
          )}
          <button
            type="button"
            data-native-recovery-guidance={recoveryGuidance}
            onClick={() => setRecoverySettingsOpen(true)}
            style={{ marginTop: 8 }}
          >
            Review encryption recovery
          </button>
        </div>
      )}
      {!snapshot.hitStart && (
        <div style={{ display: 'flex', justifyContent: 'center', paddingBottom: 12 }}>
          <button type="button" disabled={paginating} onClick={() => void paginateBackwards()}>
            {paginating ? 'Loading…' : 'Load older messages'}
          </button>
        </div>
      )}
      {error && (
        <p role="status" style={{ textAlign: 'center' }}>
          Live refresh paused; showing the last native snapshot.
        </p>
      )}
      {snapshot.items.length === 0 && (
        <NativeTimelineStatus>No messages in the native timeline yet.</NativeTimelineStatus>
      )}
      {snapshot.items.map((item) => (
        <article
          key={item.itemId}
          data-event-id={item.eventId}
          style={{ padding: '8px 0', overflowWrap: 'anywhere' }}
        >
          <header style={{ display: 'flex', alignItems: 'baseline', gap: 8 }}>
            <strong>{item.sender}</strong>
            <time
              dateTime={new Date(item.originServerTs).toISOString()}
              style={{ fontSize: 12, opacity: 0.65 }}
            >
              {new Date(item.originServerTs).toLocaleString()}
            </time>
          </header>
          <p style={{ margin: '4px 0 0', whiteSpace: 'pre-wrap' }}>{item.body}</p>
        </article>
      ))}
      {recoverySettingsOpen && (
        <Modal500 requestClose={() => setRecoverySettingsOpen(false)}>
          <Settings
            initialPage={SettingsPages.DevicesPage}
            requestClose={() => setRecoverySettingsOpen(false)}
          />
        </Modal500>
      )}
    </div>
  );
}

function NativeTimelineStatus({ children }: { children: ReactNode }) {
  return (
    <div
      role="status"
      style={{
        minHeight: 0,
        flex: '1 1 auto',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: 24,
      }}
    >
      {children}
    </div>
  );
}
