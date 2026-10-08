import { useEffect, useRef, useState } from 'react';
import { getRecentEmojis } from '../plugins/recent-emoji';
import { IEmoji } from '../plugins/emoji';
import { AccountDataEvent } from '../../types/matrix/accountData';
import { useNativeAccountData } from '../native/nativeAccountData';

/**
 * Recently used emoji (Element's `io.element.recent_emoji` account data).
 * The list is fixed once loaded so it does not reorder while a picker is open;
 * a first load that finishes after mount still fills it in.
 */
export const useRecentEmoji = (limit?: number): IEmoji[] => {
  const content = useNativeAccountData(AccountDataEvent.ElementRecentEmoji);
  const [recentEmoji, setRecentEmoji] = useState(() => getRecentEmojis(limit));
  const loaded = useRef(content !== undefined);

  useEffect(() => {
    if (loaded.current || content === undefined) return;
    loaded.current = true;
    setRecentEmoji(getRecentEmojis(limit));
  }, [content, limit]);

  return recentEmoji;
};
