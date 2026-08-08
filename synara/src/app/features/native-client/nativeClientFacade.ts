import {
  hasForbiddenWireFields,
  isObject,
  optBoolean,
  optString,
  reqNumber,
  reqString,
} from '../matrix-dto/parseUtil';
import type { DesktopInvokeResult } from '../../utils/desktop';

/**
 * F1 — renderer NativeMatrixClient facade core (emitter + lifecycle + identity).
 * Operator-authorized Option A (complete native) + D1C (renderer cedes token
 * custody): this facade is the structural replacement for the js-sdk client
 * object that currently lives behind `Awaited<ReturnType<typeof initClient>>`.
 *
 * ADDITIVE SLICE (F1): this module is not yet wired into the boot path
 * (initMatrix.ts is untouched; importer drop happens at F6). It is unit-tested
 * in isolation via an injectable NativeInvoke, per the house owner pattern.
 *
 * D1C contract: NO token surface. getAccessToken/setAccessToken/refreshToken do
 * not exist here; token custody is native-only and never crosses IPC (a
 * `session_updated` event carries readiness/generation, never tokens).
 */

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

/** Serialized `SyncReadiness` enum (src-tauri/src/matrix/sync/readiness.rs). */
export type NativeReadiness =
  | 'unconfigured'
  | 'idle'
  | 'running'
  | 'offline'
  | 'failed'
  | 'terminated';

/** Structural mirror of the Rust `SyncReadinessSnapshot` DTO. */
export type NativeSyncStatus = {
  readiness: NativeReadiness;
  sessionGeneration: number;
  offlineModeEnabled: boolean;
  failureDiagnosticId?: string | null;
};

/** js-sdk-compatible sync-state strings the app UI already consumes. */
export type NativeSyncState =
  | 'PREPARED'
  | 'SYNCING'
  | 'CATCHUP'
  | 'ERROR'
  | 'RECONNECTING'
  | 'STOPPED';

export type NativeSyncStateData = {
  readiness: NativeReadiness;
  sessionGeneration: number;
  failureDiagnosticId?: string | null;
};

/** Structural mirror of the Rust `MatrixSessionSnapshot` DTO. */
export type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
  userId?: string;
  deviceId?: string;
  homeserverUrl?: string;
  sessionGeneration?: number;
};

export type NativeClientIdentity = {
  userId?: string;
  deviceId?: string;
  homeserverUrl?: string;
};

export type NativeInvokeResult = {
  status: 'ok';
};

export type NativeClientListener = (payload?: unknown) => void;

/** Minimal typed emitter surface covering the app's USED event names (probed). */
export type NativeClientEvents = {
  sync: NativeSyncState | null;
  session: NativeSessionSnapshot;
};

export type NativeClientEventName = keyof NativeClientEvents | string;

const UNAVAILABLE_MESSAGE = 'Native Matrix client is unavailable.';
const FORBIDDEN_READINESS = new Set(['offline', 'failed']);

/** Map Rust sync readiness onto the js-sdk SyncState literals the UI reads. */
export const readinessToSyncState = (readiness: NativeReadiness): NativeSyncState => {
  switch (readiness) {
    case 'running':
      return 'PREPARED';
    case 'offline':
      return 'RECONNECTING';
    case 'failed':
      return 'ERROR';
    case 'unconfigured':
    case 'idle':
      return 'STOPPED';
    default:
      return 'STOPPED';
  }
};

const isSafeGeneration = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value) && value > 0;

const parseSyncStatus = (value: unknown): NativeSyncStatus | null => {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const readiness = optString(value, 'readiness');
  const sessionGeneration = reqNumber(value, 'sessionGeneration');
  const offlineModeEnabled = optBoolean(value, 'offlineModeEnabled');
  const readinesses: readonly string[] = [
    'unconfigured',
    'idle',
    'running',
    'offline',
    'failed',
    'terminated',
  ];
  if (
    typeof readiness !== 'string' ||
    !readinesses.includes(readiness) ||
    sessionGeneration === null ||
    !isSafeGeneration(sessionGeneration) ||
    typeof offlineModeEnabled !== 'boolean'
  ) {
    return null;
  }
  return {
    readiness: readiness as NativeReadiness,
    sessionGeneration,
    offlineModeEnabled,
    failureDiagnosticId: optString(value, 'failureDiagnosticId') ?? null,
  };
};

const parseSessionSnapshot = (value: unknown): NativeSessionSnapshot => {
  if (!isObject(value) || hasForbiddenWireFields(value)) {
    return { status: 'logged_out' };
  }
  if (value.status !== 'logged_in') {
    return { status: 'logged_out' };
  }
  return {
    status: 'logged_in',
    userId: reqString(value, 'user_id') ?? reqString(value, 'userId') ?? undefined,
    deviceId: reqString(value, 'device_id') ?? reqString(value, 'deviceId') ?? undefined,
    homeserverUrl:
      reqString(value, 'homeserver_url') ?? reqString(value, 'homeserverUrl') ?? undefined,
    sessionGeneration: reqNumber(value, 'sessionGeneration') ?? undefined,
  };
};

/** In-process emitter (F1). A native sync-state PUSH event is a later slice. */
export class NativeClientEmitter {
  private readonly listeners = new Map<string, Set<NativeClientListener>>();

  public on(event: NativeClientEventName, listener: NativeClientListener): this {
    let set = this.listeners.get(event);
    if (!set) {
      set = new Set();
      this.listeners.set(event, set);
    }
    set.add(listener);
    return this;
  }

  public once(event: NativeClientEventName, listener: NativeClientListener): this {
    const wrapped: NativeClientListener = (payload) => {
      this.removeListener(event, wrapped);
      listener(payload);
    };
    return this.on(event, wrapped);
  }

  public removeListener(event: NativeClientEventName, listener: NativeClientListener): this {
    this.listeners.get(event)?.delete(listener);
    return this;
  }

  public off(event: NativeClientEventName, listener: NativeClientListener): this {
    return this.removeListener(event, listener);
  }

  public emit(event: NativeClientEventName, payload?: unknown): boolean {
    const set = this.listeners.get(event);
    if (!set || set.size === 0) return false;
    for (const listener of [...set]) listener(payload);
    return true;
  }

  public setMaxListeners(): this {
    // no-op cap for F1; kept for API parity with the js-sdk emitter surface.
    return this;
  }

  public listenerCount(event: NativeClientEventName): number {
    return this.listeners.get(event)?.size ?? 0;
  }
}

/**
 * F1 facade — command-backed native client proxy.
 * D1C: no token getters/setters; sync-readiness and identity come from
 * `matrix_sync_status` / `matrix_session_snapshot`, never secrets.
 */
export const createNativeMatrixClient = (invoke: NativeInvoke) => {
  const emitter = new NativeClientEmitter();
  let cachedSyncState: NativeSyncState | null = null;
  let cachedSyncData: NativeSyncStateData | null = null;

  const readSyncStatus = async (): Promise<NativeSyncStatus | null> => {
    const result = await invoke('matrix_sync_status');
    if (!result.available) return null;
    return parseSyncStatus(result.value);
  };

  const emitSyncState = (state: NativeSyncState, data: NativeSyncStateData | null): void => {
    cachedSyncState = state;
    cachedSyncData = data;
    emitter.emit('sync', state);
  };

  return Object.freeze({
    emitter,
    on: emitter.on.bind(emitter),
    once: emitter.once.bind(emitter),
    off: emitter.off.bind(emitter),
    removeListener: emitter.removeListener.bind(emitter),
    emit: emitter.emit.bind(emitter),
    setMaxListeners: emitter.setMaxListeners.bind(emitter),

    /** js-sdk-compatible lifecycle. readState=false: cached value only. */
    async getSyncState(readState?: boolean): Promise<NativeSyncState | null> {
      if (readState !== false || cachedSyncState === null) {
        const status = await readSyncStatus();
        if (status) {
          emitSyncState(readinessToSyncState(status.readiness), {
            readiness: status.readiness,
            sessionGeneration: status.sessionGeneration,
            failureDiagnosticId: status.failureDiagnosticId,
          });
        }
      }
      return cachedSyncState;
    },

    async getSyncStateData(): Promise<NativeSyncStateData | null> {
      if (cachedSyncData === null) {
        const status = await readSyncStatus();
        if (status) {
          emitSyncState(readinessToSyncState(status.readiness), {
            readiness: status.readiness,
            sessionGeneration: status.sessionGeneration,
            failureDiagnosticId: status.failureDiagnosticId,
          });
        }
      }
      return cachedSyncData;
    },

    async clientRunning(): Promise<boolean> {
      const status = await readSyncStatus();
      return status?.readiness === 'running';
    },

    async retryImmediately(): Promise<void> {
      const status = await readSyncStatus();
      if (!status) throw new Error(UNAVAILABLE_MESSAGE);
      emitSyncState(readinessToSyncState(status.readiness), {
        readiness: status.readiness,
        sessionGeneration: status.sessionGeneration,
        failureDiagnosticId: status.failureDiagnosticId,
      });
    },

    /** Native sync runs in Rust; startClient is a readiness confirmation. */
    async startClient(): Promise<void> {
      await this.getSyncState();
    },

    /** Session teardown path: retain the facade, drop readiness. */
    async stopClient(): Promise<void> {
      cachedSyncState = null;
      cachedSyncData = null;
      emitter.emit('sync', 'STOPPED');
    },

    async logout(): Promise<void> {
      const result = await invoke('matrix_logout');
      if (!result.available) throw new Error(UNAVAILABLE_MESSAGE);
      cachedSyncState = null;
      cachedSyncData = null;
      emitter.emit('sync', 'STOPPED');
    },

    /** D1C identity: user/device/homeserver from the native session snapshot. */
    async getIdentity(): Promise<NativeClientIdentity> {
      const result = await invoke('matrix_session_snapshot');
      if (!result.available) return {};
      return parseSessionSnapshot(result.value) as NativeClientIdentity;
    },

    async getUserId(): Promise<string | undefined> {
      return (await this.getIdentity()).userId;
    },

    async getSafeUserId(): Promise<string> {
      return (await this.getIdentity()).userId ?? '';
    },

    async getDeviceId(): Promise<string | undefined> {
      return (await this.getIdentity()).deviceId;
    },

    async setDisplayName(displayName: string): Promise<NativeInvokeResult> {
      const result = await invoke('matrix_set_own_display_name', { displayName });
      if (!result.available) throw new Error(UNAVAILABLE_MESSAGE);
      return result.value as NativeInvokeResult;
    },

    async setAvatarUrl(mxc: string): Promise<NativeInvokeResult> {
      const result = await invoke('matrix_set_own_avatar', { avatarUrl: mxc });
      if (!result.available) throw new Error(UNAVAILABLE_MESSAGE);
      return result.value as NativeInvokeResult;
    },

    /** Poll matrix_sync_status and emit 'sync' on change; returns unsubscribe. */
    watchSync(pollMs = 1500): () => void {
      let stopped = false;
      let last: NativeSyncState | null = cachedSyncState;
      const tick = async (): Promise<void> => {
        if (stopped) return;
        const status = await readSyncStatus();
        if (status) {
          const next = readinessToSyncState(status.readiness);
          if (next !== last) {
            last = next;
            emitSyncState(next, {
              readiness: status.readiness,
              sessionGeneration: status.sessionGeneration,
              failureDiagnosticId: status.failureDiagnosticId,
            });
          }
        }
      };
      void tick();
      const timer = setInterval(() => void tick(), pollMs);
      timer.unref?.();
      return () => {
        stopped = true;
        clearInterval(timer);
      };
    },

    // D1C guard: fail-closed readiness helper for callers that must not run
    // while the native session is offline/failed.
    isReady(): boolean {
      return !FORBIDDEN_READINESS.has(cachedSyncData?.readiness ?? 'unconfigured');
    },
  });
};

export type NativeMatrixClient = ReturnType<typeof createNativeMatrixClient>;
