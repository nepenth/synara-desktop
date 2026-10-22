import React, { useCallback, useEffect, useState } from 'react';
import { Box, Button, Icon, Icons, Text, color, config } from 'folds';
import { startVisibilityAwarePoll } from '../../utils/visibilityPoll';
import {
  INITIAL_ROOM_IDENTITY_WARNINGS_STATE,
  ROOM_IDENTITY_CHECK_FAILED_COPY,
  ROOM_IDENTITY_WARNINGS_POLL_MS,
  applyRoomIdentityWarningsLoad,
  loadRoomIdentityWarnings,
  resolveRoomIdentityWarning,
  roomIdentityBanner,
} from './nativeRoomIdentityWarnings';

const FAILURE_COPY = "Couldn't update this identity change.";

/** Warns above the composer when a room member's cryptographic identity changed. */
export function RoomIdentityWarningBanner({ roomId }: { roomId: string }) {
  const [state, setState] = useState(INITIAL_ROOM_IDENTITY_WARNINGS_STATE);
  const [pending, setPending] = useState(false);
  const [failed, setFailed] = useState(false);
  const [checkAttempt, setCheckAttempt] = useState(0);

  useEffect(() => {
    // Another room's warnings never apply here.
    setState(INITIAL_ROOM_IDENTITY_WARNINGS_STATE);
    setFailed(false);
  }, [roomId]);

  useEffect(() => {
    let disposed = false;
    const refresh = () => {
      loadRoomIdentityWarnings(roomId)
        .then((next) => {
          if (!disposed && next) {
            setState((current) =>
              applyRoomIdentityWarningsLoad(current, { ok: true, warnings: next })
            );
          }
        })
        .catch(() => {
          // Fail closed: keep the last known warnings and say the check failed.
          if (!disposed)
            setState((current) => applyRoomIdentityWarningsLoad(current, { ok: false }));
        });
    };
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
  }, [roomId, checkAttempt]);

  const banner = roomIdentityBanner(state.warnings);

  const resolve = useCallback(() => {
    if (!banner || pending) return;
    setPending(true);
    setFailed(false);
    resolveRoomIdentityWarning(roomId, banner.userId, banner.action)
      .then((warnings) =>
        setState((current) => applyRoomIdentityWarningsLoad(current, { ok: true, warnings }))
      )
      .catch(() => setFailed(true))
      .finally(() => setPending(false));
  }, [banner, pending, roomId]);

  const retryCheck = useCallback(() => setCheckAttempt((attempt) => attempt + 1), []);

  if (!banner) {
    if (!state.checkFailed) return null;
    return (
      <Box
        role="alert"
        data-room-identity-warning="check_failed"
        alignItems="Center"
        gap="300"
        style={{
          padding: `${config.space.S200} ${config.space.S300}`,
          marginBottom: config.space.S200,
          borderRadius: config.radii.R400,
          background: color.Warning.Container,
          color: color.Warning.OnContainer,
        }}
      >
        <Icon src={Icons.Warning} size="200" />
        <Box grow="Yes" direction="Column">
          <Text size="T300">{ROOM_IDENTITY_CHECK_FAILED_COPY}</Text>
        </Box>
        <Button size="300" variant="Warning" fill="Soft" radii="300" onClick={retryCheck}>
          <Text size="B300">Retry</Text>
        </Button>
      </Box>
    );
  }
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
        {state.checkFailed ? <Text size="T200">{ROOM_IDENTITY_CHECK_FAILED_COPY}</Text> : null}
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
