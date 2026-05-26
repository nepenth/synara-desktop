import { useEffect, useState } from 'react';
import { cancelIdle, requestIdle } from '../utils/performance';

export const useIdleRender = (enabled = true, timeout = 1200): boolean => {
  const [ready, setReady] = useState(!enabled);

  useEffect(() => {
    if (!enabled) {
      setReady(true);
      return undefined;
    }

    setReady(false);
    const handle = requestIdle(() => setReady(true), { timeout });
    return () => cancelIdle(handle);
  }, [enabled, timeout]);

  return ready;
};
