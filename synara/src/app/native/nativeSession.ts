import type { RoomSummary } from '../features/matrix-dto/room';
import type { MatrixEventReading, MemberReading, RoomReading } from '../utils/room';
import { invokeDesktopWithAvailability } from '../utils/desktop';
import { RoomType } from '../../types/matrix/room';
import { setNativeIdentity } from '../state/nativeIdentity';
import {
  parseRoomListSnapshot,
  parseSessionSnapshot,
  parseSyncStatus,
  readinessToSyncState,
  UNAVAILABLE_MESSAGE,
  type NativeClientIdentity,
  type NativeInvoke,
  type NativeRoomListSnapshot,
  type NativeSessionSnapshot,
  type NativeSyncState,
  type NativeSyncStateData,
  type NativeSyncStatus,
} from './nativeWire';

/**
 * The renderer's view of the installed native session: identity, sync
 * readiness, the room-list projection, and the session lifecycle. Native owns
 * sync, tokens and crypto; this module caches what the native commands report
 * so components can read it synchronously.
 */

/**
 * A room as the native room-list snapshot describes it. Reads go through the
 * latest summary, so a holder never sees a stale name, avatar, membership or
 * unread count. Room state, members and timeline come from their own native
 * owners, so those reads are empty here. Native rooms emit no per-room events:
 * timeline and state updates arrive through the native timeline and room
 * owners, so `on`/`removeListener` register nothing.
 */
export type NativeRoom = RoomReading & {
  on(event: string, listener: (...args: unknown[]) => void): void;
  removeListener(event: string, listener: (...args: unknown[]) => void): void;
  getUsersReadUpTo(event: MatrixEventReading): string[];
  hasEncryptionStateEvent(): boolean;
  findEventById(eventId: string): MatrixEventReading | undefined;
  getJoinedMembers(): MemberReading[];
};

type EmptyStateReading = {
  getStateEvents(eventType: string): MatrixEventReading[];
  getStateEvents(eventType: string, stateKey: string): MatrixEventReading | null;
  events: Map<string, Map<string, MatrixEventReading>>;
};

const EMPTY_STATE: EmptyStateReading = {
  getStateEvents: (_eventType: string, stateKey?: string) => (stateKey === undefined ? [] : null),
  events: new Map(),
} as EmptyStateReading;

type RoomSummaryRef = { current: RoomSummary };

const noListener = (): void => undefined;

const createNativeRoom = (summaryRef: RoomSummaryRef): NativeRoom => {
  const summary = (): RoomSummary => summaryRef.current;
  return {
    get roomId() {
      return summary().roomId;
    },
    get name() {
      return summary().name ?? '';
    },
    currentState: EMPTY_STATE as RoomReading['currentState'],
    getLiveTimeline: () => ({
      getState: () => undefined,
      getEvents: () => [],
    }),
    getMember: () => null,
    getMembers: () => [],
    getMxcAvatarUrl: () => summary().avatarUrl ?? null,
    getAvatarFallbackMember: () => undefined,
    getUnreadNotificationCount: () => summary().unreadCount,
    getEventReadUpTo: () => null,
    getLastActiveTimestamp: () => summary().lastActivityTs,
    getBumpStamp: () => summary().lastActivityTs,
    get lastMessagePreview() {
      return summary().lastMessagePreview;
    },
    getThreads: () => [],
    accountData: new Map(),
    getMyMembership: () => summary().membership,
    getJoinRule: () => summary().joinRule ?? '',
    getJoinedMemberCount: () => 0,
    getCanonicalAlias: () => summary().canonicalAlias ?? null,
    getType: () => (summary().isCall ? RoomType.Call : undefined),
    getVersion: () => '',
    isCallRoom: () => summary().isCall,
    isSpaceRoom: () => summary().isSpace,
    get isFavorite() {
      return summary().isFavorite;
    },
    get isDirect() {
      return summary().isDirect;
    },
    get isEncrypted() {
      return summary().isEncrypted;
    },
    get notificationMode() {
      return summary().notificationMode;
    },
    getTimelineForEvent: () => null,
    hasMembershipState: () => summary().membership === 'join',
    on: noListener,
    removeListener: noListener,
    getUsersReadUpTo: () => [],
    hasEncryptionStateEvent: () => false,
    findEventById: () => undefined,
    getJoinedMembers: () => [],
  };
};

/** What a session subscriber hears. */
export type NativeSessionEvents = {
  /** A connection-state transition, or `'STOPPED'` when the session's caches clear. */
  sync: NativeSyncState;
  /** A replacement session (new generation, user or device) or a logout snapshot. */
  session: NativeSessionSnapshot;
  /** The signed-in session ended (native reported logged out). */
  loggedOut: undefined;
};

export type NativeSessionEvent = keyof NativeSessionEvents;

type Listener<E extends NativeSessionEvent> = (payload: NativeSessionEvents[E]) => void;

const FORBIDDEN_READINESS = new Set(['offline', 'failed']);

export const createNativeSession = (invoke: NativeInvoke) => {
  const listeners: { [E in NativeSessionEvent]: Set<Listener<E>> } = {
    sync: new Set(),
    session: new Set(),
    loggedOut: new Set(),
  };
  const emit = <E extends NativeSessionEvent>(event: E, payload: NativeSessionEvents[E]) => {
    for (const listener of [...listeners[event]]) (listener as Listener<E>)(payload);
  };

  let identity: NativeClientIdentity = {};
  let syncState: NativeSyncState | null = null;
  let syncData: NativeSyncStateData | null = null;
  let rooms: NativeRoom[] = [];
  let roomEntries = new Map<string, { summaryRef: RoomSummaryRef; room: NativeRoom }>();
  let sessionGeneration: number | undefined;
  // A voluntary logout closes the command gate on purpose. While it runs, the
  // sync poll is not a connection signal and must not paint Connection Lost.
  let logoutInFlight = false;
  // stop() tears down renderer reads before a reload; that STOPPED is not a
  // lost connection either.
  let stopped = false;

  /** A definitive logged-in snapshot is cached and the session is not winding down. */
  const hasSignedInSession = (): boolean => !logoutInFlight && !stopped && Boolean(identity.userId);

  /**
   * A closed gate means connection loss only for a signed-in session. Signed
   * out (or mid-logout) Core has no owners by design, so the gate reads open.
   */
  const syncStateForStatus = (status: NativeSyncStatus): NativeSyncState =>
    readinessToSyncState(status.readiness, hasSignedInSession() ? status.commandGate : 'open');

  const clearRooms = (): void => {
    rooms = [];
    roomEntries = new Map();
  };

  const readSyncStatus = (): Promise<NativeSyncStatus | null> =>
    invoke('matrix_sync_status').then((result) =>
      result.available ? parseSyncStatus(result.value) : null
    );

  const readSession = async (): Promise<NativeSessionSnapshot | null> => {
    const result = await invoke('matrix_session_snapshot');
    return result.available ? parseSessionSnapshot(result.value) : null;
  };

  const readRooms = async (): Promise<NativeRoomListSnapshot | null> => {
    const result = await invoke('matrix_room_list_snapshot');
    return result.available ? parseRoomListSnapshot(result.value) : null;
  };

  /**
   * Apply one already-validated room-list projection. Both refreshes and the
   * room-list atom owner call this, so `getRoom()` observes the same snapshot
   * that drives sidebar ordering. Existing room identities stay stable while
   * their summary reads become current.
   */
  const applyRoomListSnapshot = (snapshot: NativeRoomListSnapshot): void => {
    if (sessionGeneration !== undefined && snapshot.sessionGeneration !== sessionGeneration) {
      return;
    }
    const nextEntries = new Map<string, { summaryRef: RoomSummaryRef; room: NativeRoom }>();
    const nextRooms: NativeRoom[] = [];
    for (const summary of snapshot.rooms) {
      let entry = roomEntries.get(summary.roomId);
      if (entry) {
        entry.summaryRef.current = summary;
      } else {
        const summaryRef = { current: summary };
        entry = { summaryRef, room: createNativeRoom(summaryRef) };
      }
      nextEntries.set(summary.roomId, entry);
      nextRooms.push(entry.room);
    }
    roomEntries = nextEntries;
    rooms = nextRooms;
  };

  const applySyncStatus = (status: NativeSyncStatus): void => {
    syncState = syncStateForStatus(status);
    syncData = {
      readiness: status.readiness,
      sessionGeneration: status.sessionGeneration,
      failureDiagnosticId: status.failureDiagnosticId,
      slidingSyncCapable: status.slidingSyncCapable,
    };
  };

  const emitSyncState = (): void => {
    if (syncState !== null) emit('sync', syncState);
  };

  const clearSession = ({
    clearIdentity = false,
    notifyLoggedOut = false,
  }: { clearIdentity?: boolean; notifyLoggedOut?: boolean } = {}): void => {
    const hadIdentity = Boolean(identity.userId || identity.deviceId);
    const hadSessionState = hadIdentity || syncState !== null || rooms.length > 0;
    if (clearIdentity) {
      identity = {};
      setNativeIdentity({});
    }
    syncState = null;
    syncData = null;
    sessionGeneration = undefined;
    clearRooms();
    if (hadSessionState) emit('sync', 'STOPPED');
    if (notifyLoggedOut && hadIdentity) {
      emit('session', { status: 'logged_out' });
      emit('loggedOut', undefined);
    }
  };

  /**
   * Apply a definitive session snapshot before its room-list projection. A new
   * generation/user/device never reuses the previous session's rooms, and stale
   * projections are rejected by applyRoomListSnapshot.
   */
  const applySessionSnapshot = (session: NativeSessionSnapshot): boolean => {
    if (session.status === 'logged_out') {
      clearSession({ clearIdentity: true, notifyLoggedOut: true });
      return false;
    }

    const sessionChanged =
      (sessionGeneration !== undefined && sessionGeneration !== session.sessionGeneration) ||
      (identity.userId !== undefined && identity.userId !== session.userId) ||
      (identity.deviceId !== undefined && identity.deviceId !== session.deviceId);
    if (sessionChanged) {
      const hadSyncState = syncState !== null;
      clearRooms();
      syncState = null;
      syncData = null;
      if (hadSyncState) emit('sync', 'STOPPED');
      // A replacement is not a logout: callers rebuild user-scoped state without
      // issuing matrix_logout against the newly active native session.
      emit('session', session);
    }

    identity = {
      userId: session.userId,
      deviceId: session.deviceId,
      homeserverUrl: session.homeserverUrl,
    };
    setNativeIdentity(identity);
    sessionGeneration = session.sessionGeneration;
    return true;
  };

  /**
   * Hydrate every cached read from native commands. Fail-closed: an
   * unavailable command keeps its last-known value.
   */
  const refresh = async (): Promise<void> => {
    const [status, session, roomList] = await Promise.all([
      readSyncStatus(),
      readSession(),
      readRooms(),
    ]);

    if (session && !applySessionSnapshot(session)) return;

    const syncStateBeforeStatus = syncState;
    if (status) applySyncStatus(status);
    if (roomList) applyRoomListSnapshot(roomList);
    if (syncState !== syncStateBeforeStatus) emitSyncState();
  };

  return Object.freeze({
    /** Listen for a session event; returns the unsubscribe function. */
    subscribe<E extends NativeSessionEvent>(event: E, listener: Listener<E>): () => void {
      listeners[event].add(listener as never);
      return () => {
        listeners[event].delete(listener as never);
      };
    },
    refresh,
    applyRoomListSnapshot,
    applySessionSnapshot,

    getIdentity(): NativeClientIdentity {
      return identity;
    },
    /** Generation from the same definitive native snapshot as the cached identity. */
    getSessionGeneration(): number | undefined {
      return sessionGeneration;
    },
    getSyncState(): NativeSyncState | null {
      return syncState;
    },
    getSyncStateData(): NativeSyncStateData | null {
      return syncData;
    },
    isSyncRunning(): boolean {
      return syncData?.readiness === 'running';
    },
    /** Signed in per the last definitive native snapshot, and not logging out. */
    hasSignedInSession,
    /** False while native reports the session offline or failed. */
    isReady(): boolean {
      return !FORBIDDEN_READINESS.has(syncData?.readiness ?? 'unconfigured');
    },

    getRooms(): NativeRoom[] {
      return rooms;
    },
    getRoom(roomId: string | null | undefined): NativeRoom | null {
      if (typeof roomId !== 'string') return null;
      return roomEntries.get(roomId)?.room ?? null;
    },

    /** Ask native to recover sync now, then refresh the cached reads. */
    async retrySyncNow(): Promise<void> {
      try {
        await invoke('matrix_sync_recover');
      } finally {
        await refresh();
      }
    },
    async start(): Promise<void> {
      stopped = false;
      await refresh();
    },
    /** Clear renderer caches before a reload; identity stays for user-keyed cleanup. */
    async stop(): Promise<void> {
      stopped = true;
      clearSession();
    },
    async logout(): Promise<void> {
      logoutInFlight = true;
      try {
        const result = await invoke('matrix_logout');
        if (!result.available) throw new Error(UNAVAILABLE_MESSAGE);
        if (!result.value || (result.value as NativeSessionSnapshot).status !== 'logged_out') {
          throw new Error('Native logout did not complete. Retry before signing out.');
        }
      } finally {
        logoutInFlight = false;
      }
      try {
        clearSession({ clearIdentity: true });
      } catch {
        // The authoritative logout is complete and caches are already cleared.
        // A listener must not report native cleanup failure to the caller.
      }
    },

    /** Poll native sync readiness and emit only transitions; returns unsubscribe. */
    watchSync(pollMs = 1500): () => void {
      let watchStopped = false;
      let inFlight = false;
      let last: NativeSyncState | null = syncState;
      const tick = async (): Promise<void> => {
        if (watchStopped || inFlight) return;
        inFlight = true;
        try {
          const status = await readSyncStatus();
          if (watchStopped || !status || logoutInFlight) return;
          const next = syncStateForStatus(status);
          if (next === last && syncState === next) return;
          last = next;
          applySyncStatus(status);
          emitSyncState();
        } catch {
          // A transient IPC failure must not strand or stop the lifecycle poll.
        } finally {
          inFlight = false;
        }
      };
      void tick();
      const timer = setInterval(() => void tick(), pollMs);
      timer.unref?.();
      return () => {
        watchStopped = true;
        clearInterval(timer);
      };
    },
  });
};

export type NativeSession = ReturnType<typeof createNativeSession>;

let activeSession: NativeSession = createNativeSession((command, args) =>
  invokeDesktopWithAvailability(command, args)
);

/** The renderer's single native session. */
export const nativeSession = (): NativeSession => activeSession;

/** Swap the active session (tests inject one backed by a fake invoke). */
export const setNativeSessionForTests = (session: NativeSession): void => {
  activeSession = session;
};

/** The room the native room list knows by this id, or `null`. */
export const getNativeRoom = (roomId: string | null | undefined): NativeRoom | null =>
  activeSession.getRoom(roomId);

/** Every room in the latest native room-list snapshot. */
export const getNativeRooms = (): NativeRoom[] => activeSession.getRooms();
