import type { DesktopInvokeResult } from '../../utils/desktop';

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
};

export type NativeSpaceChildMutationResult = {
  parentId: string;
  childId: string;
  status: 'updated';
};

export type NativeJoinRulesMutationResult = {
  roomId: string;
  status: 'updated';
};

export type SpaceChildSetInput = {
  parentId: string;
  childId: string;
  via: string[];
  order?: string;
  suggested?: boolean;
};

export type JoinRuleAllowInput = {
  type?: string;
  roomId: string;
};

export type JoinRulesSetInput = {
  roomId: string;
  joinRule: string;
  allow?: JoinRuleAllowInput[];
};

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

async function requireLoggedIn(invoke: NativeInvoke): Promise<void> {
  const session = await invoke('matrix_session_snapshot');
  if (!session.available) {
    throw new Error('Native Matrix space mutation is unavailable.');
  }
  const snapshot = session.value as NativeSessionSnapshot | undefined;
  if (snapshot?.status !== 'logged_in') {
    throw new Error('Native Matrix space mutation is unavailable.');
  }
}

export async function setSpaceChildWithNativeOwner(
  input: SpaceChildSetInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<void> {
  if (!desktopAvailable) {
    throw new Error('Native Matrix space mutation is unavailable.');
  }
  await requireLoggedIn(invoke);
  const result = await invoke('matrix_space_child_set', {
    parentId: input.parentId,
    childId: input.childId,
    via: input.via,
    order: input.order,
    suggested: input.suggested ?? false,
  });
  if (!result.available) {
    throw new Error('Native Matrix space mutation is unavailable.');
  }
  const value = result.value as NativeSpaceChildMutationResult | undefined;
  if (value?.status !== 'updated') {
    throw new Error('Native Matrix space mutation is unavailable.');
  }
}

export async function removeSpaceChildWithNativeOwner(
  parentId: string,
  childId: string,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<void> {
  if (!desktopAvailable) {
    throw new Error('Native Matrix space mutation is unavailable.');
  }
  await requireLoggedIn(invoke);
  const result = await invoke('matrix_space_child_remove', { parentId, childId });
  if (!result.available) {
    throw new Error('Native Matrix space mutation is unavailable.');
  }
  const value = result.value as NativeSpaceChildMutationResult | undefined;
  if (value?.status !== 'updated') {
    throw new Error('Native Matrix space mutation is unavailable.');
  }
}

export async function setRoomJoinRulesWithNativeOwner(
  input: JoinRulesSetInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<void> {
  if (!desktopAvailable) {
    throw new Error('Native Matrix space mutation is unavailable.');
  }
  await requireLoggedIn(invoke);
  const result = await invoke('matrix_room_join_rules_set', {
    roomId: input.roomId,
    joinRule: input.joinRule,
    allow: (input.allow ?? []).map((entry) => ({
      type: entry.type ?? 'm.room_membership',
      roomId: entry.roomId,
    })),
  });
  if (!result.available) {
    throw new Error('Native Matrix space mutation is unavailable.');
  }
  const value = result.value as NativeJoinRulesMutationResult | undefined;
  if (value?.status !== 'updated') {
    throw new Error('Native Matrix space mutation is unavailable.');
  }
}
