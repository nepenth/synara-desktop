import { getAccountData } from '../utils/room';
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

/**
 * Record an emoji use. Recent emoji is account data, which has no native write
 * command, so nothing is recorded yet.
 */
export function addRecentEmoji(unicode: string): void {
  void unicode;
}
