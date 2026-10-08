import type {
  SynaraAgentApprovalHistoryItem,
  SynaraLaterItem,
  SynaraRoomNoteItem,
} from '../../app/features/matrix-dto/generated';
export type {
  SynaraAgentApprovalHistoryDecision,
  SynaraAgentApprovalHistoryItem,
  SynaraLaterItem,
  SynaraLaterItemKind,
  SynaraRoomNoteItem,
  SynaraRoomNoteItemKind,
} from '../../app/features/matrix-dto/generated';
export enum AccountDataEvent {
  PushRules = 'm.push_rules',
  Direct = 'm.direct',
  IgnoredUserList = 'm.ignored_user_list',
  MarkedUnread = 'm.marked_unread',

  SynaraSpaces = 'in.synara.spaces',
  SynaraLater = 'in.synara.later',
  SynaraRoomNotes = 'in.synara.room_notes',
  SynaraAgentApprovalHistory = 'in.synara.agent_approval_history',
  SynaraUnreadAnchor = 'in.synara.unread_anchor',

  ElementRecentEmoji = 'io.element.recent_emoji',

  PoniesUserEmotes = 'im.ponies.user_emotes',
  PoniesEmoteRooms = 'im.ponies.emote_rooms',

  MegolmBackupV1 = 'm.megolm_backup.v1',
}

export type MDirectContent = Record<string, string[]>;

export type MarkedUnreadContent = {
  unread?: boolean;
};

export type SynaraLaterContent = {
  version?: number;
  items?: Record<string, SynaraLaterItem>;
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

export type SynaraAgentApprovalHistoryContent = {
  version?: number;
  items?: SynaraAgentApprovalHistoryItem[];
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
