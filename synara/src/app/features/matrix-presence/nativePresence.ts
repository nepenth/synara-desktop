import { listen, type Event, type UnlistenFn } from '@tauri-apps/api/event';
import { useEffect, useMemo, useState } from 'react';
import {
  invokeDesktopWithAvailability,
  isSynaraDesktop,
  type DesktopInvokeResult,
} from '../../utils/desktop';
import { getSessionBootstrapResult } from '../../state/sessionBootstrap';

export enum Presence {
  Online = 'online',
  Unavailable = 'unavailable',
  Offline = 'offline',
}

export type UserPresence = {
  presence: Presence;
  status?: string;
  active: boolean;
  lastActiveTs?: number;
};

type NativePresenceState = 'unknown' | Presence;

type NativePresenceSnapshot = {
  userId: string;
  state: NativePresenceState;
  currentlyActive: boolean;
  lastActiveTs?: number;
  statusMsg?: string;
};

type NativePresenceSnapshotResult =
  | {
      status: 'ready';
      sessionGeneration: number;
      userId: string;
      snapshot: NativePresenceSnapshot;
    }
  | {
      status: 'unknown';
      sessionGeneration: number;
      userId: string;
    };

type NativePresenceSubscription = {
  subscriptionId: string;
  userId: string;
  sessionGeneration: number;
};

type NativePresenceUpdate = {
  subscriptionId: string;
  userId: string;
  sessionGeneration: number;
  outcome:
    | { status: 'ready'; snapshot: NativePresenceSnapshot }
    | { status: 'unknown' }
    | { status: 'unavailable'; diagnosticId: string };
};

export type NativePresenceInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

export type NativePresenceListen = <T>(
  event: string,
  handler: (event: Event<T>) => void
) => Promise<UnlistenFn>;

export type NativePresenceDependencies = {
  desktopAvailable: boolean;
  invoke: NativePresenceInvoke;
  listen: NativePresenceListen;
};

const PRESENCE_UPDATED_EVENT = 'matrix-presence-updated';
const unavailableMessage = 'Native Matrix presence is unavailable.';

const defaultDependencies: NativePresenceDependencies = {
  desktopAvailable: false,
  invoke: (command, args) => invokeDesktopWithAvailability(command, args),
  listen,
};

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

const onlyKnownKeys = (value: Record<string, unknown>, keys: string[]): boolean =>
  Object.keys(value).every((key) => keys.includes(key));

const isSafeGeneration = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;

const isUserId = (value: unknown): value is string =>
  typeof value === 'string' && value.length <= 255 && /^@[^:\s]+:[^\s]+$/.test(value);

const isBoundedStatus = (value: unknown): value is string =>
  typeof value === 'string' && [...value].length <= 256;

const isBoundedSubscriptionId = (value: unknown): value is string =>
  typeof value === 'string' && value.length > 0 && value.length <= 255;

const isPresenceState = (value: unknown): value is NativePresenceState =>
  value === 'unknown' || value === 'offline' || value === 'online' || value === 'unavailable';

const isSafeTimestamp = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;

const parseSnapshot = (value: unknown): NativePresenceSnapshot | undefined => {
  if (
    !isRecord(value) ||
    !onlyKnownKeys(value, ['userId', 'state', 'currentlyActive', 'lastActiveTs', 'statusMsg'])
  ) {
    return undefined;
  }
  if (
    !isUserId(value.userId) ||
    !isPresenceState(value.state) ||
    typeof value.currentlyActive !== 'boolean'
  ) {
    return undefined;
  }
  if (
    'lastActiveTs' in value &&
    value.lastActiveTs !== undefined &&
    !isSafeTimestamp(value.lastActiveTs)
  ) {
    return undefined;
  }
  if ('statusMsg' in value && value.statusMsg !== undefined && !isBoundedStatus(value.statusMsg)) {
    return undefined;
  }
  return {
    userId: value.userId,
    state: value.state,
    currentlyActive: value.currentlyActive,
    ...(value.lastActiveTs === undefined ? {} : { lastActiveTs: value.lastActiveTs as number }),
    ...(value.statusMsg === undefined ? {} : { statusMsg: value.statusMsg as string }),
  };
};

const parseSnapshotResult = (
  value: unknown,
  requestedUserId: string
): NativePresenceSnapshotResult | undefined => {
  if (
    !isRecord(value) ||
    !onlyKnownKeys(value, ['status', 'sessionGeneration', 'userId', 'snapshot'])
  ) {
    return undefined;
  }
  if (!isSafeGeneration(value.sessionGeneration) || value.userId !== requestedUserId) {
    return undefined;
  }
  if (value.status === 'unknown') {
    return value.snapshot === undefined
      ? {
          status: 'unknown',
          sessionGeneration: value.sessionGeneration,
          userId: value.userId,
        }
      : undefined;
  }
  if (value.status !== 'ready') return undefined;
  const snapshot = parseSnapshot(value.snapshot);
  return snapshot && snapshot.userId === requestedUserId && snapshot.state !== 'unknown'
    ? {
        status: 'ready',
        sessionGeneration: value.sessionGeneration,
        userId: value.userId,
        snapshot,
      }
    : undefined;
};

const parseSubscription = (
  value: unknown,
  requestedUserId: string,
  expectedGeneration: number
): NativePresenceSubscription | undefined => {
  if (
    !isRecord(value) ||
    !onlyKnownKeys(value, ['subscriptionId', 'userId', 'sessionGeneration']) ||
    !isBoundedSubscriptionId(value.subscriptionId) ||
    value.userId !== requestedUserId ||
    value.sessionGeneration !== expectedGeneration
  ) {
    return undefined;
  }
  return {
    subscriptionId: value.subscriptionId,
    userId: value.userId,
    sessionGeneration: value.sessionGeneration,
  };
};

const parseUpdate = (value: unknown): NativePresenceUpdate | undefined => {
  if (
    !isRecord(value) ||
    !onlyKnownKeys(value, ['subscriptionId', 'userId', 'sessionGeneration', 'outcome']) ||
    !isBoundedSubscriptionId(value.subscriptionId) ||
    !isUserId(value.userId) ||
    !isSafeGeneration(value.sessionGeneration) ||
    !isRecord(value.outcome) ||
    typeof value.outcome.status !== 'string'
  ) {
    return undefined;
  }
  const outcome = value.outcome;
  if (!onlyKnownKeys(outcome, ['status', 'snapshot', 'diagnosticId'])) return undefined;
  if (
    outcome.status === 'unknown' &&
    outcome.snapshot === undefined &&
    outcome.diagnosticId === undefined
  ) {
    return { ...value, outcome: { status: 'unknown' } } as NativePresenceUpdate;
  }
  if (
    outcome.status === 'unavailable' &&
    typeof outcome.diagnosticId === 'string' &&
    /^[A-Za-z0-9._-]{1,128}$/.test(outcome.diagnosticId) &&
    outcome.snapshot === undefined
  ) {
    return {
      ...value,
      outcome: { status: 'unavailable', diagnosticId: outcome.diagnosticId },
    } as NativePresenceUpdate;
  }
  const snapshot = parseSnapshot(outcome.snapshot);
  if (
    outcome.status === 'ready' &&
    snapshot &&
    snapshot.state !== 'unknown' &&
    snapshot.userId === value.userId
  ) {
    return { ...value, outcome: { status: 'ready', snapshot } } as NativePresenceUpdate;
  }
  return undefined;
};

const presenceFromSnapshot = (snapshot: NativePresenceSnapshot): UserPresence | undefined =>
  snapshot.state === Presence.Online ||
  snapshot.state === Presence.Unavailable ||
  snapshot.state === Presence.Offline
    ? {
        presence: snapshot.state,
        status: snapshot.statusMsg,
        active: snapshot.currentlyActive,
        lastActiveTs: snapshot.lastActiveTs,
      }
    : undefined;

const invokeSafely = async (
  invoke: NativePresenceInvoke,
  command: string,
  args?: Record<string, unknown>
): Promise<DesktopInvokeResult<unknown>> => {
  try {
    return await invoke(command, args);
  } catch {
    throw new Error(unavailableMessage);
  }
};

const isWritablePresence = (state: Presence): boolean =>
  state === Presence.Online || state === Presence.Unavailable || state === Presence.Offline;

/** Own-presence write. Closed vocabulary only; never sends a userId. */
export const setOwnPresenceNative = async (
  state: Presence,
  statusMsg?: string,
  invoke: NativePresenceInvoke = defaultDependencies.invoke
): Promise<void> => {
  if (!isWritablePresence(state)) {
    throw new Error(unavailableMessage);
  }
  const args: Record<string, unknown> = { state };
  if (statusMsg !== undefined) args.statusMsg = statusMsg;
  const result = await invokeSafely(invoke, 'matrix_presence_set', args);
  if (!result.available || !isRecord(result.value) || result.value.status !== 'ok') {
    throw new Error(unavailableMessage);
  }
};

/** Own-presence snapshot for Account Profile. Fail-closed when unavailable. */
export const snapshotOwnPresenceNative = async (
  userId: string,
  invoke: NativePresenceInvoke = defaultDependencies.invoke
): Promise<UserPresence | undefined> => {
  if (!isUserId(userId)) {
    throw new Error(unavailableMessage);
  }
  const result = await invokeSafely(invoke, 'matrix_presence_snapshot', { userId });
  if (!result.available) {
    throw new Error(unavailableMessage);
  }
  const parsed = parseSnapshotResult(result.value, userId);
  if (!parsed) {
    throw new Error(unavailableMessage);
  }
  return parsed.status === 'ready' ? presenceFromSnapshot(parsed.snapshot) : undefined;
};

/** Testable native owner for one profile's presence subscription. */
export const createNativePresenceSubscription = async (
  userId: string,
  dependencies: NativePresenceDependencies,
  onPresence: (presence: UserPresence | undefined) => void
): Promise<() => void> => {
  let disposed = false;
  let subscriptionId: string | undefined;
  let unlisten: UnlistenFn | undefined;
  let unlistenPromise: Promise<UnlistenFn> | undefined;
  let expectedGeneration: number | undefined;

  const dispose = () => {
    disposed = true;
    if (unlisten) {
      unlisten();
      unlisten = undefined;
    }
    if (subscriptionId) {
      const activeSubscription = subscriptionId;
      subscriptionId = undefined;
      void invokeSafely(dependencies.invoke, 'matrix_presence_unsubscribe', {
        subscriptionId: activeSubscription,
      }).catch(() => undefined);
    }
  };

  const markUnavailable = () => {
    if (!disposed) onPresence(undefined);
  };

  if (!dependencies.desktopAvailable || !isUserId(userId)) {
    markUnavailable();
    return dispose;
  }

  try {
    unlistenPromise = dependencies.listen<NativePresenceUpdate>(PRESENCE_UPDATED_EVENT, (event) => {
      if (disposed) return;
      const update = parseUpdate(event.payload);
      if (!subscriptionId) return;
      if (!update) {
        markUnavailable();
        return;
      }
      if (
        update.subscriptionId !== subscriptionId ||
        update.userId !== userId ||
        expectedGeneration === undefined ||
        update.sessionGeneration !== expectedGeneration
      ) {
        markUnavailable();
        return;
      }
      if (update.outcome.status === 'ready')
        onPresence(presenceFromSnapshot(update.outcome.snapshot));
      else onPresence(undefined);
    });
    const snapshotResult = await invokeSafely(dependencies.invoke, 'matrix_presence_snapshot', {
      userId,
    });
    if (disposed || !snapshotResult.available) throw new Error(unavailableMessage);
    const snapshot = parseSnapshotResult(snapshotResult.value, userId);
    if (!snapshot) throw new Error(unavailableMessage);
    expectedGeneration = snapshot.sessionGeneration;
    const subscriptionResult = await invokeSafely(
      dependencies.invoke,
      'matrix_presence_subscribe',
      {
        userId,
      }
    );
    if (!subscriptionResult.available) throw new Error(unavailableMessage);
    const subscription = parseSubscription(subscriptionResult.value, userId, expectedGeneration);
    if (!subscription) throw new Error(unavailableMessage);
    subscriptionId = subscription.subscriptionId;
    if (disposed) {
      dispose();
      return dispose;
    }
    if (snapshot.status === 'ready') onPresence(presenceFromSnapshot(snapshot.snapshot));
    else onPresence(undefined);

    unlisten = await unlistenPromise;
    if (disposed) {
      unlisten();
      unlisten = undefined;
    }
  } catch {
    markUnavailable();
    if (unlistenPromise) {
      void unlistenPromise
        .then((cleanup) => {
          if (disposed) cleanup();
          else unlisten = cleanup;
        })
        .catch(() => undefined);
    }
  }
  return dispose;
};

export const useNativeUserPresence = (userId: string): UserPresence | undefined => {
  const [presence, setPresence] = useState<UserPresence>();
  const dependencies = useMemo<NativePresenceDependencies>(
    () => ({
      ...defaultDependencies,
      desktopAvailable: isSynaraDesktop() && getSessionBootstrapResult().source === 'native',
    }),
    []
  );

  useEffect(() => {
    setPresence(undefined);
    let dispose: (() => void) | undefined;
    let active = true;
    void createNativePresenceSubscription(userId, dependencies, (nextPresence) => {
      if (active) setPresence(nextPresence);
    }).then((cleanup) => {
      if (active) dispose = cleanup;
      else cleanup();
    });
    return () => {
      active = false;
      dispose?.();
    };
  }, [dependencies, userId]);

  return presence;
};

export const usePresenceLabel = (): Record<Presence, string> =>
  useMemo(
    () => ({
      online: 'Active',
      unavailable: 'Busy',
      offline: 'Away',
    }),
    []
  );
