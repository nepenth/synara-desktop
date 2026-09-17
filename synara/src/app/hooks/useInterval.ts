import { useEffect, useRef } from 'react';

export type IntervalCallback = () => void;

/**
 * @param callback interval callback.
 * @param ms interval time in milliseconds. negative value will stop the interval.
 * @returns interval id or undefined if not running.
 */
export const useInterval = (callback: IntervalCallback, ms: number): number | undefined => {
  const callbackRef = useRef(callback);
  callbackRef.current = callback;
  const idRef = useRef<number | undefined>(undefined);

  useEffect(() => {
    if (ms < 0) {
      idRef.current = undefined;
      return undefined;
    }
    const id = window.setInterval(() => callbackRef.current(), ms);
    idRef.current = id;
    return () => {
      window.clearInterval(id);
      if (idRef.current === id) idRef.current = undefined;
    };
  }, [ms]);

  return idRef.current;
};
