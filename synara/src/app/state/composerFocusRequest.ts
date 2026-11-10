import { atom } from 'jotai';

/** Ask the room's main composer to take focus once the room is shown. */
export type ComposerFocusRequest = {
  roomId: string;
};

export const composerFocusRequestAtom = atom<ComposerFocusRequest | undefined>(undefined);
