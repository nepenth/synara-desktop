import { useVirtualizer } from '@tanstack/react-virtual';
import { RefObject, useCallback, useLayoutEffect, useState } from 'react';

/**
 * Sidebar lists follow actions, favorites, and category headers inside a shared
 * scroller. TanStack measures scroll offsets from that scroller, so account for
 * the list's actual origin before deciding which rooms can be unmounted.
 */
export const useNavRoomVirtualizer = (
  scrollRef: RefObject<HTMLDivElement | null>,
  count: number,
  getItemKey: (index: number) => string
) => {
  const [listElement, listRef] = useState<HTMLDivElement | null>(null);
  const [scrollMargin, setScrollMargin] = useState(0);
  const measureOrigin = useCallback(() => {
    const scroller = scrollRef.current;
    if (!scroller || !listElement) return;
    const origin =
      listElement.getBoundingClientRect().top -
      scroller.getBoundingClientRect().top -
      scroller.clientTop +
      scroller.scrollTop;
    setScrollMargin((previous) => (Math.abs(previous - origin) < 0.5 ? previous : origin));
  }, [listElement, scrollRef]);

  // Also measure after React changes categories or replaces the list's contents.
  useLayoutEffect(measureOrigin);
  useLayoutEffect(() => {
    const scroller = scrollRef.current;
    if (!scroller || !listElement) return undefined;
    const observer = new ResizeObserver(measureOrigin);
    // Ancestors resize when preceding favorites/header content or font metrics
    // change, even when the list and viewport retain their own dimensions.
    let element: HTMLElement | null = listElement;
    while (element) {
      observer.observe(element);
      if (element === scroller) break;
      element = element.parentElement;
    }
    return () => observer.disconnect();
  }, [listElement, measureOrigin, scrollRef]);

  const virtualizer = useVirtualizer({
    count,
    getScrollElement: () => scrollRef.current,
    getItemKey,
    estimateSize: () => 40,
    overscan: 10,
    scrollMargin,
  });

  return { virtualizer, listRef, scrollMargin };
};
