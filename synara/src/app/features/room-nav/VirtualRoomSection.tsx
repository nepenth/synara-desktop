import React, { ReactNode, RefObject, useCallback, useEffect, useMemo, useState } from 'react';
import { NavCategory } from '../../components/nav';
import { VirtualTile } from '../../components/virtualizer';
import { useSharedScrollVirtualizer } from '../../hooks/useSharedScrollVirtualizer';

/** Scrolls row `index` of a section into view (keyboard previous/next room). */
export type RoomSectionReveal = (index: number) => void;

type VirtualRoomSectionProps = {
  /** The list's one scroller, shared by every section. */
  scrollRef: RefObject<HTMLDivElement | null>;
  /** Always mounted, so sections stay reachable however long the list is. */
  header: ReactNode;
  /** Room ids in display order (already reduced while collapsed). */
  rows: readonly string[];
  renderRoom: (roomId: string) => ReactNode;
  /** Shown instead of rows when the section has none (an empty folder). */
  empty?: ReactNode;
  /** Receives this section's reveal while mounted, `undefined` after. */
  onReveal?: (reveal: RoomSectionReveal | undefined) => void;
  /** Exposed as `data-room-section` (the header carries `data-section-id`). */
  sectionId?: string;
};

/**
 * One room-list section: its header, then its rooms in a virtual list that
 * shares the pane's scroller with the other sections (the Rooms and Direct
 * messages pattern). A folder of hundreds of rooms renders only what is on
 * screen, as Rooms does.
 */
export function VirtualRoomSection({
  scrollRef,
  header,
  rows,
  renderRoom,
  empty,
  onReveal,
  sectionId,
}: VirtualRoomSectionProps) {
  // The scroller belongs to the parent, whose ref (re)attaches after this
  // section's layout effects on every commit. Read it once the commit is
  // done, so the virtualizer never observes a missing scroller.
  const [scrollElement, setScrollElement] = useState<HTMLDivElement | null>(null);
  // After every commit, so a replaced scroller is picked up; the same element
  // is a no-op state update, so this never loops.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => {
    setScrollElement(scrollRef.current);
  });
  const sectionScrollRef = useMemo(() => ({ current: scrollElement }), [scrollElement]);

  const getKey = useCallback((index: number) => rows[index], [rows]);
  const { virtualizer, listRef, scrollMargin } = useSharedScrollVirtualizer(
    sectionScrollRef,
    rows.length,
    getKey,
    10,
    // Room rows (`RoomSurface`) are 32px.
    32
  );

  useEffect(() => {
    if (!onReveal) return undefined;
    onReveal((index) => virtualizer.scrollToIndex(index, { align: 'auto' }));
    return () => onReveal(undefined);
  }, [onReveal, virtualizer]);

  return (
    <NavCategory data-room-section={sectionId}>
      {header}
      {rows.length === 0 && empty}
      <div ref={listRef} style={{ position: 'relative', height: virtualizer.getTotalSize() }}>
        {virtualizer.getVirtualItems().map((vItem) => (
          <VirtualTile
            virtualItem={vItem}
            scrollMargin={scrollMargin}
            key={vItem.key}
            ref={virtualizer.measureElement}
          >
            {renderRoom(rows[vItem.index])}
          </VirtualTile>
        ))}
      </div>
    </NavCategory>
  );
}
