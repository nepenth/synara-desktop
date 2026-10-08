import React, { useCallback, useEffect, useState } from 'react';
import { Box, config, Line, Text } from 'folds';
import type { NativeSession } from '../../native/nativeSession';
import { useSyncState } from '../../hooks/useSyncState';
import { ContainerColor } from '../../styles/ContainerColor.css';
import {
  CONNECTED_STATUS_BANNER_DURATION_MS,
  RECONNECTING_BANNER_HOLD_MS,
  getSlidingSyncCapabilityBannerCopy,
  getSyncStatusBannerVariant,
  getTransientSyncStatusBannerCopy,
  isSignedInSessionForBanner,
  shouldShowConnectedTransition,
  type SyncState,
} from './syncStatusCopy';

type StateData = {
  current: SyncState | null;
  previous: SyncState | null | undefined;
};

type SyncStatusProps = {
  session: NativeSession;
};
export function SyncStatus({ session }: SyncStatusProps) {
  const [stateData, setStateData] = useState<StateData>({
    current: null,
    previous: undefined,
  });
  /**
   * Tri-state server sliding-sync capability carried on the native sync-status
   * DTO (readiness.rs `sliding_sync_capable`): true=advertised, false=absent,
   * null=unprobed, undefined=js-sdk/non-native path (no banner).
   */
  const [slidingSyncCapable, setSlidingSyncCapable] = useState<boolean | null | undefined>(
    undefined
  );
  const [connectedTransitionVisible, setConnectedTransitionVisible] = useState(false);
  const [reconnectingBannerVisible, setReconnectingBannerVisible] = useState(false);
  const [recoveredFromVisibleDisconnect, setRecoveredFromVisibleDisconnect] = useState(false);

  useSyncState(
    session,
    useCallback((current, previous, data) => {
      setStateData((s) => {
        if (s.current === current && s.previous === previous) {
          return s;
        }
        return { current, previous };
      });
      if (data && typeof data === 'object' && 'slidingSyncCapable' in data) {
        setSlidingSyncCapable((prev) => {
          const next = (data as { slidingSyncCapable?: boolean | null }).slidingSyncCapable;
          return next === prev ? prev : next;
        });
      }
    }, [])
  );

  const currentSyncState = stateData.current;
  const [connectedDuringMount, setConnectedDuringMount] = useState(false);
  useEffect(() => {
    if (currentSyncState === 'PREPARED') setConnectedDuringMount(true);
  }, [currentSyncState]);

  // Native SyncService flickers Offline during short sliding-sync gaps. Hold
  // Connection Lost until Offline lasts, so a 1.5s poll blip never alarms.
  useEffect(() => {
    if (currentSyncState === 'ERROR') {
      setReconnectingBannerVisible(false);
      setRecoveredFromVisibleDisconnect(true);
      return undefined;
    }
    if (currentSyncState !== 'RECONNECTING') {
      setReconnectingBannerVisible(false);
      return undefined;
    }

    const timer = setTimeout(() => {
      setReconnectingBannerVisible(true);
      setRecoveredFromVisibleDisconnect(true);
    }, RECONNECTING_BANNER_HOLD_MS);
    return () => clearTimeout(timer);
  }, [currentSyncState]);

  // PREPARED is a steady state. Flash Connected only after a Lost banner the
  // user actually saw; warnings and failures stay while their state persists.
  useEffect(() => {
    if (!shouldShowConnectedTransition(currentSyncState, recoveredFromVisibleDisconnect)) {
      setConnectedTransitionVisible(false);
      return undefined;
    }

    setConnectedTransitionVisible(true);
    const timer = setTimeout(() => {
      setConnectedTransitionVisible(false);
      setRecoveredFromVisibleDisconnect(false);
    }, CONNECTED_STATUS_BANNER_DURATION_MS);
    return () => clearTimeout(timer);
  }, [session, currentSyncState, recoveredFromVisibleDisconnect]);

  const signedInSession = isSignedInSessionForBanner(session, connectedDuringMount);
  const bannerCopy = getTransientSyncStatusBannerCopy(
    currentSyncState,
    connectedTransitionVisible,
    reconnectingBannerVisible,
    signedInSession
  );
  const bannerVariant = getSyncStatusBannerVariant(currentSyncState, signedInSession);

  const banners: React.ReactElement[] = [];
  if (slidingSyncCapable === false) {
    banners.push(
      <Box
        key="capability"
        className={ContainerColor({ variant: 'Warning' })}
        style={{ padding: `${config.space.S100} 0` }}
        alignItems="Center"
        justifyContent="Center"
      >
        <Text size="L400">{getSlidingSyncCapabilityBannerCopy()}</Text>
      </Box>
    );
  }
  if (bannerCopy && bannerVariant) {
    banners.push(
      <Box
        key="connection"
        className={ContainerColor({ variant: bannerVariant })}
        style={{ padding: `${config.space.S100} 0` }}
        alignItems="Center"
        justifyContent="Center"
      >
        <Text size="L400">{bannerCopy}</Text>
      </Box>
    );
  }
  if (banners.length === 0) {
    return null;
  }

  return (
    <Box direction="Column" shrink="No">
      {banners.map((banner, idx) => (
        <Box key={idx} direction="Column" shrink="No">
          {banner}
          <Line variant={idx === 0 ? 'Warning' : (bannerVariant ?? 'Warning')} size="300" />
        </Box>
      ))}
    </Box>
  );
}
