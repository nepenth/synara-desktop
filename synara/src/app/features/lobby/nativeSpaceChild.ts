import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import {
  removeSpaceChildWithNativeOwner,
  setRoomJoinRulesWithNativeOwner,
  setSpaceChildWithNativeOwner,
  type JoinRulesSetInput,
  type SpaceChildSetInput,
} from './nativeSpaceChildOwner';

export async function setSpaceChild(input: SpaceChildSetInput): Promise<void> {
  await setSpaceChildWithNativeOwner(input, isSynaraDesktop(), (command, args) =>
    invokeDesktopWithAvailability(command, args)
  );
}

export async function removeSpaceChild(parentId: string, childId: string): Promise<void> {
  await removeSpaceChildWithNativeOwner(parentId, childId, isSynaraDesktop(), (command, args) =>
    invokeDesktopWithAvailability(command, args)
  );
}

export async function setRoomJoinRules(input: JoinRulesSetInput): Promise<void> {
  await setRoomJoinRulesWithNativeOwner(input, isSynaraDesktop(), (command, args) =>
    invokeDesktopWithAvailability(command, args)
  );
}
