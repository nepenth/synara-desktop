import React, { useCallback, useRef, useState } from 'react';
import { Box, Text, Tooltip, TooltipProvider } from 'folds';
import {
  formatTimelineHistoryMarkLabel,
  rowIndexForRailRatio,
  type TimelineHistoryMark,
} from '../../utils/timelineDateMarks';
import * as htmlCss from './nativeTimelineHtml.css';

type NativeTimelineDateRailProps = {
  marks: readonly TimelineHistoryMark[];
  rowCount: number;
  visibleStartIndex: number;
  activeMarkIndex: number;
  hour24Clock: boolean;
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

export function NativeTimelineDateRail({
  marks,
  rowCount,
  visibleStartIndex,
  activeMarkIndex,
  hour24Clock,
  onJumpToIndex,
}: NativeTimelineDateRailProps) {
  const trackRef = useRef<HTMLDivElement>(null);
  const [hoverLabel, setHoverLabel] = useState<string>();
  const activeMark = activeMarkIndex >= 0 ? marks[activeMarkIndex] : undefined;
  const thumbRatio = rowCount <= 1 ? 0 : visibleStartIndex / Math.max(1, rowCount - 1);

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
    },
    [hour24Clock, marks, onJumpToIndex, rowCount]
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
  };

  return (
    <Box className={htmlCss.DateRail} data-timeline-date-rail="true">
      <div
        ref={trackRef}
        className={htmlCss.DateRailTrack}
        role="scrollbar"
        aria-label="Jump to a date in loaded history"
        aria-orientation="vertical"
        aria-valuemin={0}
        aria-valuemax={Math.max(0, rowCount - 1)}
        aria-valuenow={visibleStartIndex}
        aria-valuetext={
          activeMark ? formatTimelineHistoryMarkLabel(activeMark, hour24Clock) : undefined
        }
        tabIndex={0}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
        onKeyDown={(event) => {
          if (event.key === 'ArrowUp' || event.key === 'Home') {
            event.preventDefault();
            onJumpToIndex(Math.max(0, visibleStartIndex - (event.key === 'Home' ? rowCount : 8)));
          }
          if (event.key === 'ArrowDown' || event.key === 'End') {
            event.preventDefault();
            onJumpToIndex(
              Math.min(rowCount - 1, visibleStartIndex + (event.key === 'End' ? rowCount : 8))
            );
          }
        }}
      >
        <div className={htmlCss.DateRailThumb} style={{ top: `${thumbRatio * 100}%` }} />
      </div>
      {marks.map((mark, index) => {
        const ratio = rowCount <= 1 ? 0 : mark.index / Math.max(1, rowCount - 1);
        const label = formatTimelineHistoryMarkLabel(mark, hour24Clock);
        return (
          <TooltipProvider
            key={mark.key}
            position="Left"
            offset={8}
            tooltip={
              <Tooltip>
                <Text size="T200">{label}</Text>
              </Tooltip>
            }
          >
            {(triggerRef) => (
              <button
                ref={triggerRef}
                type="button"
                className={htmlCss.DateRailTick}
                style={{ top: `${ratio * 100}%` }}
                aria-label={`Jump to ${label}`}
                aria-current={index === activeMarkIndex ? 'true' : undefined}
                onClick={() => onJumpToIndex(mark.index)}
              />
            )}
          </TooltipProvider>
        );
      })}
      {hoverLabel || activeMark ? (
        <Text className={htmlCss.DateRailLabel} size="T200" style={{ top: `${thumbRatio * 100}%` }}>
          {hoverLabel ??
            (activeMark ? formatTimelineHistoryMarkLabel(activeMark, hour24Clock) : '')}
        </Text>
      ) : null}
    </Box>
  );
}
