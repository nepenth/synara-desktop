const PERF_STORAGE_KEY = 'synara.performance.debug';
const importMetaEnv = (import.meta as ImportMeta & { env?: Record<string, string | undefined> })
  .env;
const PERF_BUILD_FLAG = importMetaEnv?.VITE_SYNARA_PERFORMANCE_DEBUG === 'true';

export const isLegacyPerformanceDebugEnabled = (): boolean => {
  if (PERF_BUILD_FLAG) return true;
  try {
    return window.localStorage.getItem(PERF_STORAGE_KEY) === 'true';
  } catch {
    return false;
  }
};

// Keep the legacy console/Performance API instrumentation isolated from the
// structured diagnostic store. Structured capture has its own privacy filter
// and must not implicitly enable unsanitized console output.
export const isPerformanceDebugEnabled = isLegacyPerformanceDebugEnabled;

export const perfMark = (name: string): void => {
  if (!isPerformanceDebugEnabled()) return;
  performance.mark(name);
};

export const perfMeasure = (name: string, startMark: string, endMark?: string): void => {
  if (!isPerformanceDebugEnabled()) return;
  try {
    performance.measure(name, startMark, endMark);
  } catch {
    // A dropped mark should never affect client behavior.
  }
};

export const perfLog = (label: string, data: Record<string, unknown>): void => {
  if (!isPerformanceDebugEnabled()) return;
  // eslint-disable-next-line no-console
  console.debug(`[synara:perf] ${label}`, data);
};

type IdleCallbackHandle = number;

type IdleDeadline = {
  didTimeout: boolean;
  timeRemaining: () => number;
};

type RequestIdleCallback = (
  callback: (deadline: IdleDeadline) => void,
  options?: { timeout?: number }
) => IdleCallbackHandle;

type CancelIdleCallback = (handle: IdleCallbackHandle) => void;

const fallbackRequestIdleCallback: RequestIdleCallback = (callback) =>
  window.setTimeout(
    () =>
      callback({
        didTimeout: false,
        timeRemaining: () => 0,
      }),
    1
  );

const fallbackCancelIdleCallback: CancelIdleCallback = (handle) => window.clearTimeout(handle);

export const requestIdle = (
  callback: (deadline: IdleDeadline) => void,
  options?: { timeout?: number }
): IdleCallbackHandle => {
  const requestIdleCallback =
    (window as Window & { requestIdleCallback?: RequestIdleCallback }).requestIdleCallback ??
    fallbackRequestIdleCallback;
  return requestIdleCallback(callback, options);
};

export const cancelIdle = (handle: IdleCallbackHandle): void => {
  const cancelIdleCallback =
    (window as Window & { cancelIdleCallback?: CancelIdleCallback }).cancelIdleCallback ??
    fallbackCancelIdleCallback;
  cancelIdleCallback(handle);
};
