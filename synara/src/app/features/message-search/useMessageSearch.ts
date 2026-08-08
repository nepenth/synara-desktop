/** Structural mirrors of the js-sdk message-search wire types (fields read here). */
type SearchEventReading = {
  event_id: string;
  type: string;
  sender: string;
  origin_server_ts: number;
  content: Record<string, any>;
  [key: string]: any;
};
type SearchResultReading = {
  rank: number;
  result: SearchEventReading;
  context: { [key: string]: any };
  [key: string]: any;
};
type SearchResponseReading = {
  search_categories: {
    room_events?: {
      next_batch?: string;
      highlights?: string[];
      results?: SearchResultReading[];
    };
  };
  [key: string]: any;
};
import { useCallback } from 'react';
import { useMatrixClient } from '../../hooks/useMatrixClient';

export type ResultItem = {
  rank: number;
  event: SearchEventReading;
  context: { [key: string]: any };
};

export type ResultGroup = {
  roomId: string;
  items: ResultItem[];
};

export type SearchResult = {
  nextToken?: string;
  highlights: string[];
  groups: ResultGroup[];
};

const groupSearchResult = (results: SearchResultReading[]): ResultGroup[] => {
  const groups: ResultGroup[] = [];

  results.forEach((item) => {
    const roomId = item.result.room_id;
    const resultItem: ResultItem = {
      rank: item.rank,
      event: item.result,
      context: item.context,
    };

    const lastAddedGroup: ResultGroup | undefined = groups[groups.length - 1];
    if (lastAddedGroup && roomId === lastAddedGroup.roomId) {
      lastAddedGroup.items.push(resultItem);
      return;
    }
    groups.push({
      roomId,
      items: [resultItem],
    });
  });

  return groups;
};

const parseSearchResult = (result: SearchResponseReading): SearchResult => {
  const roomEvents = result.search_categories.room_events;

  const searchResult: SearchResult = {
    nextToken: roomEvents?.next_batch,
    highlights: roomEvents?.highlights ?? [],
    groups: groupSearchResult(roomEvents?.results ?? []),
  };

  return searchResult;
};

export type MessageSearchParams = {
  term?: string;
  order?: string;
  rooms?: string[];
  senders?: string[];
};
export const useMessageSearch = (params: MessageSearchParams) => {
  const mx = useMatrixClient();
  const { term, order, rooms, senders } = params;

  const searchMessages = useCallback(
    async (nextBatch?: string) => {
      if (!term)
        return {
          highlights: [],
          groups: [],
        };
      const limit = 20;

      const requestBody: SearchRequestBody = {
        search_categories: {
          room_events: {
            event_context: {
              before_limit: 0,
              after_limit: 0,
              include_profile: false,
            },
            filter: {
              limit,
              rooms,
              senders,
            },
            include_state: false,
            order_by: order as 'recent',
            search_term: term,
          },
        },
      };

      type LocalMx = ReturnType<typeof useMatrixClient>;
      type SearchRequestBody = SearchResponseReading extends never
        ? never
        : {
            search_categories: {
              room_events: {
                search_term: string;
                order_by?: string;
                filter?: Record<string, unknown>;
                event_context?: Record<string, unknown>;
                include_state?: boolean;
              };
            };
          };
      const r = await mx.search({
        body: requestBody,
        next_batch: nextBatch === '' ? undefined : nextBatch,
      } as unknown as Parameters<LocalMx['search']>[0]);
      return parseSearchResult(r);
    },
    [mx, term, order, rooms, senders]
  );

  return searchMessages;
};
