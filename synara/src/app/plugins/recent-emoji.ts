import { getAccountData } from '../utils/room';
import { getCachedAccountData, setNativeAccountData } from '../native/nativeAccountData';
import { IEmoji, emojis } from './emoji';
import { AccountDataEvent } from '../../types/matrix/accountData';

type EmojiUnicode = string;
type EmojiUsageCount = number;

export type IRecentEmojiContent = {
  recent_emoji?: [EmojiUnicode, EmojiUsageCount][];
};

export const getRecentEmojis = (limit?: number): IEmoji[] => {
  const recentEmojiEvent = getAccountData(AccountDataEvent.ElementRecentEmoji);
  const recentEmoji = recentEmojiEvent?.getContent<IRecentEmojiContent>().recent_emoji;
  if (!Array.isArray(recentEmoji)) return [];

  return recentEmoji
    .sort((e1, e2) => e2[1] - e1[1])
    .slice(0, limit)
    .reduce<IEmoji[]>((list, [unicode]) => {
      const emoji = emojis.find((e) => e.unicode === unicode);
      if (emoji) list.push(emoji);
      return list;
    }, []);
};

/** Most-recent-first usage list after one more use of `unicode`, capped at 100. */
export const nextRecentEmoji = (
  current: unknown,
  unicode: EmojiUnicode
): [EmojiUnicode, EmojiUsageCount][] => {
  const recentEmoji: [EmojiUnicode, EmojiUsageCount][] = Array.isArray(current)
    ? current
        .filter(
          (entry): entry is [EmojiUnicode, EmojiUsageCount] =>
            Array.isArray(entry) && typeof entry[0] === 'string' && typeof entry[1] === 'number'
        )
        .map(([u, count]) => [u, count])
    : [];
  const emojiIndex = recentEmoji.findIndex(([u]) => u === unicode);
  let entry: [EmojiUnicode, EmojiUsageCount];
  if (emojiIndex < 0) {
    entry = [unicode, 1];
  } else {
    [entry] = recentEmoji.splice(emojiIndex, 1);
    entry = [entry[0], entry[1] + 1];
  }
  recentEmoji.unshift(entry);
  return recentEmoji.slice(0, 100);
};

/** Record an emoji use in the `io.element.recent_emoji` account data. */
export function addRecentEmoji(unicode: string): void {
  const current = getCachedAccountData(AccountDataEvent.ElementRecentEmoji) as
    IRecentEmojiContent | null | undefined;
  void setNativeAccountData(AccountDataEvent.ElementRecentEmoji, {
    ...(current ?? {}),
    recent_emoji: nextRecentEmoji(current?.recent_emoji, unicode),
  }).catch(() => undefined);
}
