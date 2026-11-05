import React, { useEffect, useRef, useState } from 'react';
import classNames from 'classnames';
import {
  CONNECTION_BEAM_CONNECTED_MS,
  connectionBeamHoldMs,
  connectionBeamLabel,
  isConnectionAttempt,
  isConnectionRecovered,
  type ConnectionBeamPhase,
  type SyncState,
} from './syncStatusCopy';
import * as css from './ConnectionBeam.css';

/**
 * Which phase the beam shows for `state`. A reconnect lights it only after
 * the hold; a recovery turns it green only when the user saw it red, then
 * hides it after the flash.
 */
export function useConnectionBeamPhase(
  state: SyncState | null,
  signedInSession: boolean
): ConnectionBeamPhase {
  const [phase, setPhase] = useState<ConnectionBeamPhase>('hidden');
  const shownRef = useRef(false);
  const attempting = isConnectionAttempt(state, signedInSession);
  const recovered = isConnectionRecovered(state);
  const holdMs = connectionBeamHoldMs(state);

  useEffect(() => {
    if (attempting) {
      if (shownRef.current) {
        setPhase('trying');
        return undefined;
      }
      // A green flash still on screen gives way while the hold runs.
      setPhase('hidden');
      const timer = window.setTimeout(() => {
        shownRef.current = true;
        setPhase('trying');
      }, holdMs);
      return () => window.clearTimeout(timer);
    }
    if (recovered && shownRef.current) {
      shownRef.current = false;
      setPhase('connected');
      const timer = window.setTimeout(() => setPhase('hidden'), CONNECTION_BEAM_CONNECTED_MS);
      return () => window.clearTimeout(timer);
    }
    if (!recovered) {
      shownRef.current = false;
      setPhase('hidden');
    }
    return undefined;
  }, [attempting, recovered, holdMs]);

  return phase;
}

/** A thin line under the titlebar that says the connection is coming back. */
export function ConnectionBeam({ phase }: { phase: ConnectionBeamPhase }) {
  const label = connectionBeamLabel(phase);
  return (
    <div className={css.Slot} data-connection-beam={phase}>
      {phase !== 'hidden' && (
        <div className={classNames(css.Beam, css.BeamPhase[phase])} aria-hidden="true">
          {phase === 'trying' && <div className={css.Pulse} />}
        </div>
      )}
      <span role="status" style={visuallyHidden}>
        {label ?? ''}
      </span>
    </div>
  );
}

const visuallyHidden: React.CSSProperties = {
  position: 'absolute',
  width: 1,
  height: 1,
  overflow: 'hidden',
  clipPath: 'inset(50%)',
  whiteSpace: 'nowrap',
};
