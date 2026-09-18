//! Empty-term Media/Files listing for one room.
//!
//! Does not paginate NativeTimelinePresenter. Core walks the room event cache
//! and `/messages` (never Client-Server `/search`) and returns SearchResult.

import { invokeDesktopWithAvailability } from '../../utils/desktop';
import {
  MessageSearchTypeFilter,
  isAttachmentListingType,
  listingDateRangeToTimestamps,
  resolveMessageSearchListingDateRange,
} from '../../utils/messageSearchFilters';
import {
  mapNativeSearchResult,
  type MappedSearchResult,
  type NativeMessageSearchResult,
} from './nativeMessageSearchMap';

export type RoomAttachmentListingKind = 'media' | 'files';

export type RoomAttachmentListingParams = {
  roomId: string;
  kind: RoomAttachmentListingKind;
  fromTs: number;
  toTs: number;
};

export const isRoomAttachmentListingKind = (type?: string): type is RoomAttachmentListingKind =>
  type === MessageSearchTypeFilter.Media || type === MessageSearchTypeFilter.Files;

export const isRoomAttachmentListingEnabled = (options: {
  term?: string;
  type?: string;
  rooms?: string[];
}): boolean => {
  const term = options.term?.trim() ?? '';
  return (
    term.length === 0 &&
    isAttachmentListingType(options.type) &&
    Array.isArray(options.rooms) &&
    options.rooms.length === 1 &&
    options.rooms[0].startsWith('!')
  );
};

export async function listRoomAttachments(
  params: RoomAttachmentListingParams
): Promise<MappedSearchResult> {
  const result = await invokeDesktopWithAvailability<NativeMessageSearchResult>(
    'matrix_message_search',
    {
      term: '',
      rooms: [params.roomId],
      listingKind: params.kind,
      fromTs: params.fromTs,
      toTs: params.toTs,
    }
  );
  if (!result.available) {
    throw new Error('Native attachment listing is unavailable.');
  }
  return result.value ? mapNativeSearchResult(result.value) : { highlights: [], groups: [] };
}

export function listingQueryRange(
  fromDate?: string,
  toDate?: string,
  now = new Date()
): { fromDate: string; toDate: string; fromTs: number; toTs: number } | undefined {
  const resolved = resolveMessageSearchListingDateRange(fromDate, toDate, now);
  const timestamps = listingDateRangeToTimestamps(resolved.fromDate, resolved.toDate);
  if (!timestamps) return undefined;
  return { ...resolved, ...timestamps };
}
