/** SDK-neutral message content projections + literal type constants. */
export type IContent = {
  msgtype?: string;
  body?: string;
  filename?: string;
  info?: Record<string, unknown>;
  url?: string;
  file?: Record<string, unknown>;
  [key: string]: unknown;
};

export type IMentions = {
  user_ids?: string[];
  room_ids?: string[];
  [key: string]: unknown;
};

export const MsgType = {
  Text: 'm.text',
  Emote: 'm.emote',
  Notice: 'm.notice',
  Image: 'm.image',
  Video: 'm.video',
  Audio: 'm.audio',
  File: 'm.file',
} as const;

export type RelationContent = {
  rel_type?: string;
  event_id?: string;
  [key: string]: unknown;
};

export const RelationType = {
  Thread: 'm.thread',
  Replace: 'm.replace',
  Reference: 'm.reference',
} as const;
