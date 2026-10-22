import { invokeDesktopWithAvailability, type DesktopInvokeResult } from './desktop';

/**
 * Serialize an async list refresh without dropping a request. A call while a
 * run is in flight marks it dirty, and the run repeats once when it finishes,
 * so the last wake of a burst is always read. Callers during a run share the
 * promise of the run that will observe their request. Once `isDisposed`
 * reports true, no further run starts.
 */
export const createCoalescedRefresh = (
  run: () => Promise<void>,
  isDisposed: () => boolean = () => false
): (() => Promise<void>) => {
  let inFlight: Promise<void> | undefined;
  let dirty = false;

  const loop = async (): Promise<void> => {
    try {
      do {
        dirty = false;
        try {
          // eslint-disable-next-line no-await-in-loop
          await run();
        } catch {
          // Each list keeps its last known value on failure; the next wake retries.
        }
      } while (dirty && !isDisposed());
    } finally {
      inFlight = undefined;
    }
  };

  return () => {
    if (isDisposed()) return Promise.resolve();
    if (inFlight) {
      dirty = true;
      return inFlight;
    }
    inFlight = loop();
    return inFlight;
  };
};

/**
 * `current` when `next` carries the same JSON, otherwise `next`. Used in atom
 * updaters so a readback equal to what is shown keeps its identity and its
 * subscribers do not re-render.
 */
export const keepIfSameJson = <T>(current: T, next: T): T =>
  current === next || JSON.stringify(current) === JSON.stringify(next) ? current : next;

type SharedInvoke = (command: string) => Promise<DesktopInvokeResult<unknown>>;

/**
 * Every native list re-reads the session gate on the same room-list wake.
 * Readers that ask while one read is in flight share it, so a wake costs one
 * `matrix_session_snapshot` instead of one per list. A read that starts after
 * the shared one settled is a fresh IPC call.
 */
export const createSharedNativeRead = (invoke: SharedInvoke) => {
  const pending = new Map<string, Promise<DesktopInvokeResult<unknown>>>();
  return <T>(command: string): Promise<DesktopInvokeResult<T>> => {
    const current = pending.get(command);
    if (current) return current as Promise<DesktopInvokeResult<T>>;
    const next = invoke(command).finally(() => {
      if (pending.get(command) === next) pending.delete(command);
    });
    pending.set(command, next);
    return next as Promise<DesktopInvokeResult<T>>;
  };
};

const sharedRead = createSharedNativeRead((command) => invokeDesktopWithAvailability(command));

/** The installed session's status, shared between the lists of one wake. */
export const readNativeSessionSnapshotShared = <T>(): Promise<DesktopInvokeResult<T>> =>
  sharedRead<T>('matrix_session_snapshot');

/** Core sync readiness, shared between concurrent readers. */
export const readNativeSyncStatusShared = <T>(): Promise<DesktopInvokeResult<T>> =>
  sharedRead<T>('matrix_sync_status');
