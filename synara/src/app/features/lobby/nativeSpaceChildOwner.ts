import type { DesktopInvokeResult } from '../../utils/desktop';
import type { MSpaceChildContent } from '../../../types/matrix/room';

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
};

export type NativeSpaceChildEdge = {
  parentId: string;
  childId: string;
  order?: string;
  suggested: boolean;
  via: string[];
  originServerTs: number;
};

export type NativeSpaceChildrenSnapshot = {
  sessionGeneration: number;
  edges: NativeSpaceChildEdge[];
};

export type NativeSpaceChildMutationResult = {
  parentId: string;
  childId: string;
  status: 'updated' | 'removed';
};

export type NativeRestrictedJoinReparentResult = {
  roomId: string;
  status: 'updated' | 'skipped';
};

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

async function requireLoggedIn(invoke: NativeInvoke): Promise<void> {
  const session = await invoke('matrix_session_snapshot');
  if (!session.available) {
    throw new Error('Native Matrix space child ownership is unavailable.');
  }
  const snapshot = session.value as NativeSessionSnapshot | undefined;
  if (snapshot?.status !== 'logged_in') {
    throw new Error('Native Matrix space child ownership requires a logged-in session.');
  }
}

export async function readSpaceChildrenWithNativeOwner(
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeSpaceChildrenSnapshot> {
  if (!desktopAvailable) {
    throw new Error('Native Matrix space child graph is unavailable.');
  }
  await requireLoggedIn(invoke);
  const result = await invoke('matrix_space_children_snapshot');
  if (!result.available || !result.value) {
    throw new Error('Native Matrix space child graph is unavailable.');
  }
  return result.value as NativeSpaceChildrenSnapshot;
}

export async function setSpaceChildWithNativeOwner(
  parentId: string,
  childId: string,
  content: MSpaceChildContent,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<void> {
  if (!desktopAvailable) {
    throw new Error('Native Matrix space child ownership is unavailable.');
  }
  await requireLoggedIn(invoke);
  const result = await invoke('matrix_space_child_set', {
    parentId,
    childId,
    via: content.via ?? [],
    order: content.order,
    suggested: content.suggested,
  });
  if (!result.available) {
    throw new Error('Native Matrix space child ownership is unavailable.');
  }
  const value = result.value as NativeSpaceChildMutationResult | undefined;
  if (value?.status !== 'updated') {
    throw new Error('Native Matrix space child ownership is unavailable.');
  }
}

export async function removeSpaceChildWithNativeOwner(
  parentId: string,
  childId: string,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<void> {
  if (!desktopAvailable) {
    throw new Error('Native Matrix space child ownership is unavailable.');
  }
  await requireLoggedIn(invoke);
  const result = await invoke('matrix_space_child_remove', {
    parentId,
    childId,
  });
  if (!result.available) {
    throw new Error('Native Matrix space child ownership is unavailable.');
  }
  const value = result.value as NativeSpaceChildMutationResult | undefined;
  if (value?.status !== 'removed') {
    throw new Error('Native Matrix space child ownership is unavailable.');
  }
}

export async function reparentRestrictedJoinWithNativeOwner(
  roomId: string,
  removeParentId: string | undefined,
  addParentId: string,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<void> {
  if (!desktopAvailable) {
    throw new Error('Native Matrix restricted join reparent is unavailable.');
  }
  await requireLoggedIn(invoke);
  const result = await invoke('matrix_restricted_join_reparent', {
    roomId,
    removeParentId,
    addParentId,
  });
  if (!result.available) {
    throw new Error('Native Matrix restricted join reparent is unavailable.');
  }
  const value = result.value as NativeRestrictedJoinReparentResult | undefined;
  if (value?.status !== 'updated' && value?.status !== 'skipped') {
    throw new Error('Native Matrix restricted join reparent is unavailable.');
  }
}

/** Convert a native edge into the product MSpaceChildContent shape. */
export const spaceChildContentFromEdge = (edge: NativeSpaceChildEdge): MSpaceChildContent => ({
  via: edge.via ?? [],
  suggested: edge.suggested,
  order: edge.order,
});
