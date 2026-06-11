import { MatrixClient } from 'matrix-js-sdk';
import React, { useCallback, useState } from 'react';
import { Box, config, Line, Text } from 'folds';
import { useSyncState } from '../../hooks/useSyncState';
import { ContainerColor } from '../../styles/ContainerColor.css';
import { getSyncStatusBannerCopy, getSyncStatusBannerVariant } from './syncStatusCopy';

type StateData = {
  current: ReturnType<MatrixClient['getSyncState']>;
  previous: ReturnType<MatrixClient['getSyncState']> | null | undefined;
};

type SyncStatusProps = {
  mx: MatrixClient;
};
export function SyncStatus({ mx }: SyncStatusProps) {
  const [stateData, setStateData] = useState<StateData>({
    current: null,
    previous: undefined,
  });

  useSyncState(
    mx,
    useCallback((current, previous) => {
      setStateData((s) => {
        if (s.current === current && s.previous === previous) {
          return s;
        }
        return { current, previous };
      });
    }, [])
  );

  const bannerCopy = getSyncStatusBannerCopy(stateData.current);
  const bannerVariant = getSyncStatusBannerVariant(stateData.current);
  if (!bannerCopy || !bannerVariant) {
    return null;
  }

  return (
    <Box direction="Column" shrink="No">
      <Box
        className={ContainerColor({ variant: bannerVariant })}
        style={{ padding: `${config.space.S100} 0` }}
        alignItems="Center"
        justifyContent="Center"
      >
        <Text size="L400">{bannerCopy}</Text>
      </Box>
      <Line variant={bannerVariant} size="300" />
    </Box>
  );
}
