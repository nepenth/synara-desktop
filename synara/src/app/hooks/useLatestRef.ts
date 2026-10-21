import { useLayoutEffect, useRef } from 'react';

/**
 * The latest value for long-lived listeners. A listener registered once reads
 * `ref.current` when it fires, instead of being torn down and registered again
 * (an async IPC round trip on desktop) whenever the value changes.
 */
export const useLatestRef = <T>(value: T) => {
  const ref = useRef(value);
  useLayoutEffect(() => {
    ref.current = value;
  }, [value]);
  return ref;
};
