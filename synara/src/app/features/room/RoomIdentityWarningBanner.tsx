import React, { useCallback, useEffect, useState } from 'react';
import { Box, Button, Icon, Icons, Text, color, config } from 'folds';
import { startVisibilityAwarePoll } from '../../utils/visibilityPoll';
import type { NativeRoomIdentityWarning } from '../matrix-dto/generated';
import {
  ROOM_IDENTITY_WARNINGS_POLL_MS,
  loadRoomIdentityWarnings,
  resolveRoomIdentityWarning,
  roomIdentityBanner,
} from './nativeRoomIdentityWarnings';

const FAILURE_COPY = "Couldn't update this identity change.";

/** Warns above the composer when a room member's cryptographic identity changed. */
export function RoomIdentityWarningBanner({ roomId }: { roomId: string }) {
  const [warnings, setWarnings] = useState<NativeRoomIdentityWarning[]>([]);
  const [pending, setPending] = useState(false);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let disposed = false;
    const refresh = () => {
      loadRoomIdentityWarnings(roomId)
        .then((next) => {
          if (!disposed && next) setWarnings(next);
        })
        .catch(() => undefined);
    };
    setWarnings([]);
    setFailed(false);
    refresh();
    const stop = startVisibilityAwarePoll(
      refresh,
      ROOM_IDENTITY_WARNINGS_POLL_MS,
      ROOM_IDENTITY_WARNINGS_POLL_MS * 4
    );
    return () => {
      disposed = true;
      stop();
    };
  }, [roomId]);

  const banner = roomIdentityBanner(warnings);

  const resolve = useCallback(() => {
    if (!banner || pending) return;
    setPending(true);
    setFailed(false);
    resolveRoomIdentityWarning(roomId, banner.userId, banner.action)
      .then(setWarnings)
      .catch(() => setFailed(true))
      .finally(() => setPending(false));
  }, [banner, pending, roomId]);

  if (!banner) return null;
  const critical = banner.kind === 'verification_violation';
  return (
    <Box
      role="status"
      aria-live="polite"
      data-room-identity-warning={banner.kind}
      alignItems="Center"
      gap="300"
      style={{
        padding: `${config.space.S200} ${config.space.S300}`,
        marginBottom: config.space.S200,
        borderRadius: config.radii.R400,
        background: critical ? color.Critical.Container : color.SurfaceVariant.Container,
        color: critical ? color.Critical.OnContainer : color.SurfaceVariant.OnContainer,
      }}
    >
      <Icon src={critical ? Icons.Warning : Icons.Shield} size="200" />
      <Box grow="Yes" direction="Column">
        <Text size="T300">{banner.message}</Text>
        {failed ? (
          <Text size="T200" role="alert">
            {FAILURE_COPY}
          </Text>
        ) : null}
      </Box>
      <Button
        size="300"
        variant={critical ? 'Critical' : 'Secondary'}
        fill="Soft"
        radii="300"
        disabled={pending}
        aria-busy={pending || undefined}
        onClick={resolve}
      >
        <Text size="B300">{banner.actionLabel}</Text>
      </Button>
    </Box>
  );
}
