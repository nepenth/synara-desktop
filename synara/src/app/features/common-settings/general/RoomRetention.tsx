import React, { useEffect, useState } from 'react';
import { Spinner, Text, color } from 'folds';
import { SequenceCard } from '../../../components/sequence-card';
import { SequenceCardStyle } from '../../room-settings/styles.css';
import { SettingTile } from '../../../components/setting-tile';
import { useRoom } from '../../../hooks/useRoom';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../../utils/desktop';
import {
  getRoomRetentionWithNativeOwner,
  type NativeRoomRetentionSnapshot,
} from './nativeRoomRetentionOwner';

export function RoomRetention() {
  const room = useRoom();
  const [snapshot, setSnapshot] = useState<NativeRoomRetentionSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    void getRoomRetentionWithNativeOwner(
      room.roomId,
      isSynaraDesktop(),
      invokeDesktopWithAvailability
    )
      .then((next) => {
        if (cancelled) return;
        setSnapshot(next);
        setLoading(false);
      })
      .catch((cause: Error) => {
        if (cancelled) return;
        setSnapshot(null);
        setError(cause.message);
        setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [room.roomId]);

  return (
    <SequenceCard
      className={SequenceCardStyle}
      variant="SurfaceVariant"
      direction="Column"
      gap="400"
    >
      <SettingTile title="Message retention">
        {loading && <Spinner size="100" variant="Secondary" />}
        {!loading && snapshot && (
          <>
            <Text size="T200">{snapshot.summary}</Text>
            <Text size="T200">{snapshot.distinction}</Text>
            <Text size="T200">{snapshot.mediaCacheSummary}</Text>
          </>
        )}
        {!loading && error && (
          <Text style={{ color: color.Critical.Main }} size="T200">
            {error}
          </Text>
        )}
      </SettingTile>
    </SequenceCard>
  );
}
