import { getDMRoomFor } from '../../utils/matrix';
import { addRoomIdToMDirect } from '../room/nativeMDirect';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import { createRoomWithNativeOwner } from '../../components/nativeRoomCreateOwner';
import { getSharedSettings } from '../../state/settings';
import { pushEncryptedStateEventsSetting } from '../settings/encryptedStateEvents';

/**
 * Creates a direct-message room with `userId`, invites them and records the
 * room in `m.direct`. Returns the room id.
 */
export const createDirectChat = async (
  userId: string,
  encrypted: boolean,
  encryptState: boolean
): Promise<string> => {
  await pushEncryptedStateEventsSetting(getSharedSettings().encryptedStateEvents);
  const roomId = await createRoomWithNativeOwner(
    {
      isDirect: true,
      invite: [userId],
      visibility: 'private',
      preset: 'trusted_private_chat',
      encryption: encrypted,
      encryptStateEvents: encrypted && encryptState,
    },
    isSynaraDesktop(),
    (command, args) => invokeDesktopWithAvailability(command, args)
  );

  await addRoomIdToMDirect(roomId, userId);

  return roomId;
};

export type DirectChatSources = {
  /** The joined direct room with the user, if there is one. */
  findExisting: (userId: string) => string | undefined;
  /** Creates a direct room with the user and returns its id. */
  create: (userId: string) => Promise<string>;
};

/**
 * Returns `resolve(userId)`: the direct room to open for a user, the one
 * that exists or a new one. While a room for a user is being created, further
 * calls for that user share it, so a second click cannot make a second room.
 */
export const directChatResolver = (sources: DirectChatSources) => {
  const creating = new Map<string, Promise<string>>();
  return (userId: string): Promise<string> => {
    const existing = sources.findExisting(userId);
    if (existing) return Promise.resolve(existing);
    const pending = creating.get(userId);
    if (pending) return pending;
    const created = sources.create(userId).finally(() => {
      creating.delete(userId);
    });
    creating.set(userId, created);
    return created;
  };
};

/**
 * The room to open when someone chooses Message on a user: their existing
 * direct message, or a new encrypted one with the new-chat form's defaults.
 * The form remains for starting a chat from a typed user ID.
 */
export const openOrCreateDirectChat = directChatResolver({
  findExisting: (userId) => getDMRoomFor(userId)?.roomId,
  create: (userId) => createDirectChat(userId, true, getSharedSettings().encryptedStateEvents),
});
