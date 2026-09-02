import { atom } from 'jotai';
import { atomFamily } from 'jotai/utils';
import { Descendant } from 'slate';
import { EncryptedAttachmentInfo } from '../../../types/matrix/common';
import { createUploadAtomFamily } from '../upload';
import { TUploadContent } from '../../utils/matrix';
import { createListAtom } from '../list';

export type TUploadMetadata = {
  markedAsSpoiler: boolean;
};

export type TUploadItem = {
  transactionId: string;
  file: TUploadContent;
  originalFile: TUploadContent;
  metadata: TUploadMetadata;
  encInfo: EncryptedAttachmentInfo | undefined;
};

export type TUploadListAtom = ReturnType<typeof createListAtom<TUploadItem>>;

export const roomIdToUploadItemsAtomFamily = atomFamily<string, TUploadListAtom>(createListAtom);

export const roomUploadAtomFamily = createUploadAtomFamily();

export type RoomIdToMsgAction =
  | {
      type: 'PUT';
      roomId: string;
      msg: Descendant[];
    }
  | {
      type: 'DELETE';
      roomId: string;
    };

const createMsgDraftAtom = () => atom<Descendant[]>([]);
export type TMsgDraftAtom = ReturnType<typeof createMsgDraftAtom>;
export const roomIdToMsgDraftAtomFamily = atomFamily<string, TMsgDraftAtom>(() =>
  createMsgDraftAtom()
);
