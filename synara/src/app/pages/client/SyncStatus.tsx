import React, { useCallback, useEffect, useState } from 'react';
import { Box, config, Line, Text } from 'folds';
import type { NativeSession } from '../../native/nativeSession';
import { useSyncState } from '../../hooks/useSyncState';
import { ContainerColor } from '../../styles/ContainerColor.css';
import {
  getSlidingSyncCapabilityBannerCopy,
  getSyncStatusBannerVariant,
  getTransientSyncStatusBannerCopy,
  isSignedInSessionForBanner,
  type SyncState,
} from './syncStatusCopy';
import { ConnectionBeam, useConnectionBeamPhase } from './ConnectionBeam';

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

  const signedInSession = isSignedInSessionForBanner(session, connectedDuringMount);
  // Reconnecting and the recovery flash belong to the beam; the text banner
  // is only for a connection that has failed outright.
  const beamPhase = useConnectionBeamPhase(currentSyncState, signedInSession);
  const bannerCopy = getTransientSyncStatusBannerCopy(
    currentSyncState,
    false,
    false,
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
  const beam = <ConnectionBeam phase={beamPhase} />;
  if (banners.length === 0) {
    return beam;
  }

  return (
    <Box direction="Column" shrink="No">
      {beam}
      {banners.map((banner, idx) => (
        <Box key={idx} direction="Column" shrink="No">
          {banner}
          <Line variant={idx === 0 ? 'Warning' : (bannerVariant ?? 'Warning')} size="300" />
        </Box>
      ))}
    </Box>
  );
}
