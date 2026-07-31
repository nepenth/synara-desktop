import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import {
  addRoomToMDirectWithNativeOwner,
  removeRoomFromMDirectWithNativeOwner,
} from './nativeMDirectOwner';

export async function addRoomIdToMDirect(roomId: string, userId: string): Promise<void> {
  await addRoomToMDirectWithNativeOwner(roomId, userId, isSynaraDesktop(), (command, args) =>
    invokeDesktopWithAvailability(command, args)
  );
}

export async function removeRoomIdFromMDirect(roomId: string): Promise<void> {
  await removeRoomFromMDirectWithNativeOwner(roomId, isSynaraDesktop(), (command, args) =>
    invokeDesktopWithAvailability(command, args)
  );
}
