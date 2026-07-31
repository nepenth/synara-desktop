import type { DesktopInvokeResult } from '../../utils/desktop';

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
};

export type NativeSpaceHierarchyRoom = {
  roomId: string;
  name?: string;
  canonicalAlias?: string;
  topic?: string;
  avatarUrl?: string;
  roomType?: string;
  numJoinedMembers: number;
  joinRule: string;
  worldReadable: boolean;
  guestCanJoin: boolean;
};

export type NativeSpaceHierarchySnapshot = {
  sessionGeneration: number;
  rooms: NativeSpaceHierarchyRoom[];
};

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

export async function readSpaceHierarchyWithNativeOwner(
  roomId: string,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeSpaceHierarchySnapshot> {
  if (!desktopAvailable) {
    throw new Error('Native Matrix space hierarchy is unavailable.');
  }
  const session = await invoke('matrix_session_snapshot');
  if (!session.available) {
    throw new Error('Native Matrix space hierarchy requires a logged-in session.');
  }
  const sessionSnapshot = session.value as NativeSessionSnapshot | undefined;
  if (sessionSnapshot?.status !== 'logged_in') {
    throw new Error('Native Matrix space hierarchy requires a logged-in session.');
  }
  const result = await invoke('matrix_space_hierarchy_snapshot', {
    roomId,
  });
  if (!result.available || !result.value) {
    throw new Error('Native Matrix space hierarchy is unavailable.');
  }
  return result.value as NativeSpaceHierarchySnapshot;
}
