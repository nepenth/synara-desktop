export enum MessageSearchTypeFilter {
  All = 'all',
  Media = 'media',
  Files = 'files',
  Audio = 'audio',
  Links = 'links',
  Polls = 'polls',
}

export type SearchFilterResultItem = {
  event: {
    origin_server_ts?: number;
    content?: Record<string, unknown>;
  };
};

export type SearchFilterResultGroup<TItem extends SearchFilterResultItem = SearchFilterResultItem> =
  {
    roomId: string;
    items: TItem[];
  };

export type MessageSearchFilterOptions = {
  type?: string;
  fromDate?: string;
  toDate?: string;
};

const HTTP_URL_REGEX = /\bhttps?:\/\/\S+/i;

const toStartOfDay = (date?: string): number | undefined => {
  if (!date) return undefined;
  const timestamp = new Date(`${date}T00:00:00.000`).getTime();
  return Number.isNaN(timestamp) ? undefined : timestamp;
};

const toEndOfDay = (date?: string): number | undefined => {
  if (!date) return undefined;
  const timestamp = new Date(`${date}T23:59:59.999`).getTime();
  return Number.isNaN(timestamp) ? undefined : timestamp;
};

const padDatePart = (value: number): string => String(value).padStart(2, '0');

const isoDay = (date: Date): string =>
  `${date.getFullYear()}-${padDatePart(date.getMonth() + 1)}-${padDatePart(date.getDate())}`;

/** Inclusive local calendar range ending today. `days: 7` is today plus the previous 6 days. */
export const lastNDaysDateRange = (
  days: number,
  nowMs = Date.now()
): { fromDate: string; toDate: string } => {
  const safeDays = Math.max(1, Math.floor(days));
  const to = new Date(nowMs);
  const from = new Date(nowMs);
  from.setHours(0, 0, 0, 0);
  from.setDate(from.getDate() - (safeDays - 1));
  return { fromDate: isoDay(from), toDate: isoDay(to) };
};

export const isLastNDaysDateRange = (
  fromDate: string | undefined,
  toDate: string | undefined,
  days: number,
  nowMs = Date.now()
): boolean => {
  const range = lastNDaysDateRange(days, nowMs);
  return fromDate === range.fromDate && toDate === range.toDate;
};

export const parseSenderFilter = (value: string): string[] | undefined => {
  const senders = value
    .split(',')
    .map((sender) => sender.trim())
    .filter(Boolean);

  return senders.length > 0 ? senders : undefined;
};

export const isAttachmentListingType = (type?: string): boolean =>
  type === MessageSearchTypeFilter.Media || type === MessageSearchTypeFilter.Files;

export const defaultMessageSearchListingDateRange = (
  now = new Date()
): { fromDate: string; toDate: string } => lastNDaysDateRange(7, now.getTime());

export const resolveMessageSearchListingDateRange = (
  fromDate?: string,
  toDate?: string,
  now = new Date()
): { fromDate: string; toDate: string } => {
  const defaults = defaultMessageSearchListingDateRange(now);
  return {
    fromDate: fromDate || defaults.fromDate,
    toDate: toDate || defaults.toDate,
  };
};

export const listingDateRangeToTimestamps = (
  fromDate: string,
  toDate: string
): { fromTs: number; toTs: number } | undefined => {
  const fromTs = toStartOfDay(fromDate);
  const toTs = toEndOfDay(toDate);
  if (fromTs === undefined || toTs === undefined || fromTs > toTs) return undefined;
  return { fromTs, toTs };
};

export const isMessageSearchResultForType = (
  item: SearchFilterResultItem,
  type?: string
): boolean => {
  if (!type || type === MessageSearchTypeFilter.All) return true;

  const content = item.event.content ?? {};
  const { msgtype, body } = content;

  if (type === MessageSearchTypeFilter.Media) {
    return msgtype === 'm.image' || msgtype === 'm.video';
  }
  if (type === MessageSearchTypeFilter.Files) {
    return msgtype === 'm.file' || !!content.file || !!content.filename;
  }
  if (type === MessageSearchTypeFilter.Audio) {
    return msgtype === 'm.audio';
  }
  if (type === MessageSearchTypeFilter.Links) {
    return (
      (typeof content.url === 'string' && content.url.startsWith('mxc://') === false) ||
      (typeof body === 'string' && HTTP_URL_REGEX.test(body))
    );
  }
  if (type === MessageSearchTypeFilter.Polls) {
    return !!content['m.poll'] || msgtype === 'm.poll.start';
  }

  return true;
};

export const isMessageSearchResultInDateRange = (
  item: SearchFilterResultItem,
  fromDate?: string,
  toDate?: string
): boolean => {
  const fromTs = toStartOfDay(fromDate);
  const toTs = toEndOfDay(toDate);
  const eventTs = item.event.origin_server_ts;

  if (eventTs === undefined || Number.isNaN(eventTs)) return true;
  if (fromTs !== undefined && eventTs < fromTs) return false;
  if (toTs !== undefined && eventTs > toTs) return false;
  return true;
};

export const filterMessageSearchGroups = <
  TItem extends SearchFilterResultItem,
  TGroup extends SearchFilterResultGroup<TItem>
>(
  groups: TGroup[],
  options: MessageSearchFilterOptions
): TGroup[] =>
  groups
    .map((group) => ({
      ...group,
      items: group.items.filter(
        (item) =>
          isMessageSearchResultForType(item, options.type) &&
          isMessageSearchResultInDateRange(item, options.fromDate, options.toDate)
      ),
    }))
    .filter((group) => group.items.length > 0);
