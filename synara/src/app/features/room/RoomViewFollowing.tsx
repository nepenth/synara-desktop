import React from 'react';
import * as css from './RoomViewFollowing.css';

export function RoomViewFollowingPlaceholder() {
  return <div className={css.RoomViewFollowingPlaceholder} />;
}

/**
 * The strip under the composer. It listed who had read the latest message
 * from the js-sdk room's receipts, which the native room facade never has.
 * Core does not report per-event readers yet, so the strip stays empty until
 * it does (D15).
 */
export function RoomViewFollowing() {
  return <div className={css.RoomViewFollowing({ clickable: false })} />;
}
