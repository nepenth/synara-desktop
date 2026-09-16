import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';

export const NATIVE_THREAD_LIST_SCHEMA_VERSION = 1;

export type NativeThreadSummary = {
  roomId: string;
  rootEventId: string;
  replyCount: number;
  latestEventId?: string;
  latestOriginServerTs?: number;
  participated: boolean;
  unreadCount?: number;
};

export type NativeThreadListSnapshot = {
  schemaVersion: number;
  roomId: string;
  threads: NativeThreadSummary[];
  endReached: boolean;
  truncated: boolean;
};

export type NativeThreadListAction = 'open' | 'paginate' | 'close';

const acceptSnapshot = (
  value: unknown,
  expectedRoomId: string
): NativeThreadListSnapshot | undefined => {
  if (!value || typeof value !== 'object') return undefined;
  const snapshot = value as NativeThreadListSnapshot;
  if (
    snapshot.schemaVersion !== NATIVE_THREAD_LIST_SCHEMA_VERSION ||
    snapshot.roomId !== expectedRoomId ||
    !Array.isArray(snapshot.threads)
  ) {
    return undefined;
  }
  return snapshot;
};

export async function nativeThreadList(
  roomId: string,
  action: NativeThreadListAction
): Promise<NativeThreadListSnapshot | 'unavailable'> {
  if (!isSynaraDesktop()) return 'unavailable';
  const result = await invokeDesktopWithAvailability('matrix_thread_list', {
    request: { roomId, action },
  });
  if (!result.available) return 'unavailable';
  return acceptSnapshot(result.value, roomId) ?? 'unavailable';
}
