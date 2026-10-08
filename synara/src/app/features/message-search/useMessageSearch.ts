/** One search hit's event, as the result view reads it. */
type SearchEventReading = {
  event_id: string;
  type: string;
  sender: string;
  origin_server_ts: number;
  content: Record<string, any>;
  [key: string]: any;
};
import { useCallback } from 'react';
import { invokeDesktopWithAvailability } from '../../utils/desktop';
import { isNativeMatrixSession } from '../verification/nativeVerification';
import { mapNativeSearchResult, type NativeMessageSearchResult } from './nativeMessageSearchMap';

export {
  mapNativeSearchResult,
  type NativeMessageSearchItem,
  type NativeMessageSearchResult,
} from './nativeMessageSearchMap';

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

export type MessageSearchParams = {
  term?: string;
  order?: string;
  rooms?: string[];
  senders?: string[];
};

export const useMessageSearch = (params: MessageSearchParams) => {
  const { term, order, rooms, senders } = params;
  const nativeSession = isNativeMatrixSession();

  const searchMessages = useCallback(
    async (nextBatch?: string) => {
      if (!term)
        return {
          highlights: [],
          groups: [],
        };

      if (nativeSession) {
        const result = await invokeDesktopWithAvailability<NativeMessageSearchResult>(
          'matrix_message_search',
          {
            term,
            nextToken: nextBatch === '' ? undefined : nextBatch,
            rooms,
            senders,
            order,
          }
        );
        if (!result.available) {
          throw new Error('Native message search is unavailable.');
        }
        return result.value
          ? mapNativeSearchResult(result.value)
          : { nextToken: undefined, highlights: [], groups: [] };
      }

      // Server-side search runs only through the native command.
      return { nextToken: undefined, highlights: [], groups: [] };
    },
    [nativeSession, term, order, rooms, senders]
  );

  return searchMessages;
};
