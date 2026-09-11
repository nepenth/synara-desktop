import { atom } from 'jotai';

export type ComposerMentionInsert = {
  roomId: string;
  userId: string;
  name: string;
};

export const composerMentionInsertAtom = atom<ComposerMentionInsert | undefined>(undefined);
