export enum AccountDataEvent {
  PushRules = 'm.push_rules',
  Direct = 'm.direct',
  IgnoredUserList = 'm.ignored_user_list',
  MarkedUnread = 'm.marked_unread',

  SynaraSpaces = 'in.synara.spaces',
  SynaraLater = 'in.synara.later',
  SynaraRoomNotes = 'in.synara.room_notes',
  SynaraUnreadAnchor = 'in.synara.unread_anchor',

  ElementRecentEmoji = 'io.element.recent_emoji',

  PoniesUserEmotes = 'im.ponies.user_emotes',
  PoniesEmoteRooms = 'im.ponies.emote_rooms',

  SecretStorageDefaultKey = 'm.secret_storage.default_key',

  MegolmBackupV1 = 'm.megolm_backup.v1',
}

export type MDirectContent = Record<string, string[]>;

export type MarkedUnreadContent = {
  unread?: boolean;
};

export type SynaraLaterItemKind = 'saved' | 'reminder';

export type SynaraLaterItem = {
  id: string;
  kind: SynaraLaterItemKind;
  roomId: string;
  eventId: string;
  createdAt: number;
  dueTs?: number;
  remindedAt?: number;
  completedAt?: number;
};

export type SynaraLaterContent = {
  version?: number;
  items?: Record<string, SynaraLaterItem>;
};

export type SynaraRoomNoteItemKind = 'note' | 'todo' | 'message';

export type SynaraRoomNoteItem = {
  id: string;
  kind: SynaraRoomNoteItemKind;
  roomId: string;
  createdAt: number;
  updatedAt: number;
  order?: number;
  body?: string;
  completedAt?: number;
  eventId?: string;
  eventTs?: number;
  sender?: string;
};

export type SynaraRoomNotesContent = {
  version?: number;
  rooms?: Record<
    string,
    {
      items?: Record<string, SynaraRoomNoteItem>;
    }
  >;
};

export type SynaraUnreadAnchorContent = {
  version?: number;
  anchors?: Record<
    string,
    {
      eventId: string;
      ts: number;
    }
  >;
};

export type SecretStorageDefaultKeyContent = {
  key: string;
};

export type SecretStoragePassphraseContent = {
  algorithm: string;
  salt: string;
  iterations: number;
  bits?: number;
};

export type SecretStorageKeyContent = {
  name?: string;
  algorithm: string;
  iv?: string;
  mac?: string;
  passphrase?: SecretStoragePassphraseContent;
};

export type SecretContent = {
  iv: string;
  ciphertext: string;
  mac: string;
};
