import React, { useCallback, useLayoutEffect, useRef, useState } from 'react';
import { Text } from 'folds';
import {
  activeTimelineHistoryMarkIndex,
  formatTimelineHistoryMarkLabel,
  rowIndexForRailRatio,
  type TimelineHistoryMark,
} from '../../utils/timelineDateMarks';
import * as htmlCss from './nativeTimelineHtml.css';

type NativeTimelineDateRailProps = {
  marks: readonly TimelineHistoryMark[];
  rowCount: number;
  hour24Clock: boolean;
  scrollRef: React.RefObject<HTMLDivElement | null>;
  getVisibleStartIndex: () => number;
  onJumpToIndex: (index: number) => void;
};

const markAtOrBefore = (
  marks: readonly TimelineHistoryMark[],
  index: number
): TimelineHistoryMark | undefined => {
  let current: TimelineHistoryMark | undefined;
  for (const mark of marks) {
    if (mark.index <= index) current = mark;
    else break;
  }
  return current;
};

export const NativeTimelineDateRail = React.memo(function NativeTimelineDateRail({
  marks,
  rowCount,
  hour24Clock,
  scrollRef,
  getVisibleStartIndex,
  onJumpToIndex,
}: NativeTimelineDateRailProps) {
  const trackRef = useRef<HTMLDivElement>(null);
  const thumbRef = useRef<HTMLDivElement>(null);
  const labelRef = useRef<HTMLDivElement>(null);
  const ticksRef = useRef<Array<HTMLButtonElement | null>>([]);
  const lastActiveMarkRef = useRef(-1);
  const [hoverLabel, setHoverLabel] = useState<string>();

  const paintThumb = useCallback(
    (index: number, updateAria: boolean) => {
      const track = trackRef.current;
      const thumb = thumbRef.current;
      if (!track || !thumb || rowCount <= 0) return;
      const ratio = rowCount <= 1 ? 0 : index / Math.max(1, rowCount - 1);
      const y = ratio * track.clientHeight;
      thumb.style.transform = `translate3d(0, ${y}px, 0)`;
      if (labelRef.current) {
        labelRef.current.style.transform = `translate3d(0, ${y}px, 0) translateY(-50%)`;
      }
      const active = activeTimelineHistoryMarkIndex(marks, index);
      if (!updateAria && active === lastActiveMarkRef.current) return;
      lastActiveMarkRef.current = active;
      track.setAttribute('aria-valuenow', String(index));
      const mark = active >= 0 ? marks[active] : markAtOrBefore(marks, index);
      const label = mark ? formatTimelineHistoryMarkLabel(mark, hour24Clock) : undefined;
      if (label) track.setAttribute('aria-valuetext', label);
      else track.removeAttribute('aria-valuetext');
      ticksRef.current.forEach((tick, tickIndex) => {
        if (!tick) return;
        if (tickIndex === active) tick.setAttribute('aria-current', 'true');
        else tick.removeAttribute('aria-current');
      });
    },
    [hour24Clock, marks, rowCount]
  );

  useLayoutEffect(() => {
    const scrollEl = scrollRef.current;
    const sync = () => paintThumb(getVisibleStartIndex(), false);
    sync();
    if (!scrollEl) return undefined;
    const onScroll = () => paintThumb(getVisibleStartIndex(), false);
    scrollEl.addEventListener('scroll', onScroll, { passive: true });
    const track = trackRef.current;
    const observer = track ? new ResizeObserver(sync) : undefined;
    if (track) observer?.observe(track);
    return () => {
      scrollEl.removeEventListener('scroll', onScroll);
      observer?.disconnect();
    };
  }, [getVisibleStartIndex, paintThumb, scrollRef]);

  const jumpFromClientY = useCallback(
    (clientY: number) => {
      const track = trackRef.current;
      if (!track || rowCount <= 0) return;
      const rect = track.getBoundingClientRect();
      const ratio = rect.height <= 0 ? 0 : (clientY - rect.top) / rect.height;
      const index = rowIndexForRailRatio(ratio, rowCount);
      onJumpToIndex(index);
      const mark = markAtOrBefore(marks, index);
      setHoverLabel(mark ? formatTimelineHistoryMarkLabel(mark, hour24Clock) : undefined);
      paintThumb(index, true);
    },
    [hour24Clock, marks, onJumpToIndex, paintThumb, rowCount]
  );

  const onPointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    jumpFromClientY(event.clientY);
  };
  const onPointerMove = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!event.currentTarget.hasPointerCapture(event.pointerId)) return;
    jumpFromClientY(event.clientY);
  };
  const onPointerUp = (event: React.PointerEvent<HTMLDivElement>) => {
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    setHoverLabel(undefined);
  };

  return (
    <div className={htmlCss.DateRail} data-timeline-date-rail="true">
      <div
        ref={trackRef}
        className={htmlCss.DateRailTrack}
        role="scrollbar"
        aria-label="Jump to a date in loaded history"
        aria-controls="native-timeline-history"
        aria-orientation="vertical"
        aria-valuemin={0}
        aria-valuemax={Math.max(0, rowCount - 1)}
        aria-valuenow={0}
        tabIndex={0}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
        onKeyDown={(event) => {
          const startIndex = getVisibleStartIndex();
          if (event.key === 'ArrowUp' || event.key === 'Home') {
            event.preventDefault();
            onJumpToIndex(Math.max(0, startIndex - (event.key === 'Home' ? rowCount : 8)));
          }
          if (event.key === 'ArrowDown' || event.key === 'End') {
            event.preventDefault();
            onJumpToIndex(
              Math.min(rowCount - 1, startIndex + (event.key === 'End' ? rowCount : 8))
            );
          }
        }}
      >
        <div ref={thumbRef} className={htmlCss.DateRailThumb} />
      </div>
      {marks.map((mark, index) => {
        const ratio = rowCount <= 1 ? 0 : mark.index / Math.max(1, rowCount - 1);
        const label = formatTimelineHistoryMarkLabel(mark, hour24Clock);
        return (
          <button
            key={mark.key}
            ref={(node) => {
              ticksRef.current[index] = node;
            }}
            type="button"
            className={htmlCss.DateRailTick}
            style={{ top: `${ratio * 100}%` }}
            title={label}
            aria-label={`Jump to ${label}`}
            onClick={() => onJumpToIndex(mark.index)}
          />
        );
      })}
      {hoverLabel ? (
        <div ref={labelRef} className={htmlCss.DateRailLabel}>
          <Text size="T200">{hoverLabel}</Text>
        </div>
      ) : null}
    </div>
  );
});
