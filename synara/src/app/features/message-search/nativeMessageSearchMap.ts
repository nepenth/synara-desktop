export type NativeMessageSearchItem = {
  rank: number;
  eventId: string;
  sender: string;
  originServerTs: number;
  body: string;
  roomId: string;
  msgType?: string;
};

export type NativeMessageSearchGroup = {
  roomId: string;
  items: NativeMessageSearchItem[];
};

export type NativeMessageSearchResult = {
  nextToken?: string;
  highlights: string[];
  groups: NativeMessageSearchGroup[];
};

export type MappedSearchResultItem = {
  rank: number;
  event: {
    event_id: string;
    type: string;
    sender: string;
    origin_server_ts: number;
    room_id: string;
    content: {
      msgtype: string;
      body: string;
    };
  };
  context: Record<string, never>;
};

export type MappedSearchResult = {
  nextToken?: string;
  highlights: string[];
  groups: Array<{
    roomId: string;
    items: MappedSearchResultItem[];
  }>;
};

export const mapNativeSearchResult = (value: NativeMessageSearchResult): MappedSearchResult => ({
  nextToken: value.nextToken,
  highlights: value.highlights ?? [],
  groups: (value.groups ?? []).map((group) => ({
    roomId: group.roomId,
    items: (group.items ?? []).map((item) => ({
      rank: item.rank,
      event: {
        event_id: item.eventId,
        type: 'm.room.message',
        sender: item.sender,
        origin_server_ts: item.originServerTs,
        room_id: item.roomId,
        content: {
          msgtype: item.msgType?.trim() || 'm.text',
          body: item.body,
        },
      },
      context: {},
    })),
  })),
});
