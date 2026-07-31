import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import type { MSpaceChildContent } from '../../../types/matrix/room';
import {
  reparentRestrictedJoinWithNativeOwner,
  removeSpaceChildWithNativeOwner,
  setSpaceChildWithNativeOwner,
  type NativeInvoke,
} from './nativeSpaceChildOwner';

const desktopInvoke: NativeInvoke = async (command, args) =>
  invokeDesktopWithAvailability(command, args);

export async function setSpaceChild(
  parentId: string,
  childId: string,
  content: MSpaceChildContent,
) {
  await setSpaceChildWithNativeOwner(parentId, childId, content, isSynaraDesktop(), desktopInvoke);
}

export async function removeSpaceChild(parentId: string, childId: string) {
  await removeSpaceChildWithNativeOwner(parentId, childId, isSynaraDesktop(), desktopInvoke);
}

export async function reparentRestrictedJoin(
  roomId: string,
  removeParentId: string | undefined,
  addParentId: string,
) {
  await reparentRestrictedJoinWithNativeOwner(
    roomId,
    removeParentId,
    addParentId,
    isSynaraDesktop(),
    desktopInvoke,
  );
}
