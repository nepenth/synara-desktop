import React, { useCallback, useLayoutEffect, useRef, useState } from 'react';
import { Text } from 'folds';
import {
  activeTimelineHistoryMarkForTimestamp,
  formatTimelineHistoryMarkLabel,
  formatTimelineRailTimestamp,
  railRatioForTimestamp,
  timestampForRailRatio,
  type TimelineHistoryMark,
  type TimelineRailAxis,
} from '../../utils/timelineDateMarks';
import * as htmlCss from './nativeTimelineHtml.css';

type NativeTimelineDateRailProps = {
  marks: readonly TimelineHistoryMark[];
  axis: TimelineRailAxis;
  hour24Clock: boolean;
  scrollRef: React.RefObject<HTMLDivElement | null>;
  getVisibleTimestamp: () => number;
  onPreviewTimestamp: (timestampMs: number) => void;
  onCommitTimestamp: (timestampMs: number) => void;
};

export const NativeTimelineDateRail = React.memo(function NativeTimelineDateRail({
  marks,
  axis,
  hour24Clock,
  scrollRef,
  getVisibleTimestamp,
  onPreviewTimestamp,
  onCommitTimestamp,
}: NativeTimelineDateRailProps) {
  const trackRef = useRef<HTMLDivElement>(null);
  const thumbRef = useRef<HTMLDivElement>(null);
  const labelRef = useRef<HTMLDivElement>(null);
  const ticksRef = useRef<Array<HTMLButtonElement | null>>([]);
  const lastActiveMarkRef = useRef(-1);
  const lastTimestampRef = useRef(axis.endMs);
  const [hoverLabel, setHoverLabel] = useState<string>();

  const paintThumb = useCallback(
    (timestampMs: number, updateAria: boolean) => {
      const track = trackRef.current;
      const thumb = thumbRef.current;
      if (!track || !thumb) return;
      const ratio = railRatioForTimestamp(timestampMs, axis.startMs, axis.endMs);
      const y = ratio * track.clientHeight;
      thumb.style.transform = `translate3d(0, ${y}px, 0)`;
      if (labelRef.current) {
        labelRef.current.style.transform = `translate3d(0, ${y}px, 0) translateY(-50%)`;
      }
      const active = activeTimelineHistoryMarkForTimestamp(marks, timestampMs);
      if (!updateAria && active === lastActiveMarkRef.current) return;
      lastActiveMarkRef.current = active;
      track.setAttribute('aria-valuenow', String(timestampMs));
      const mark = active >= 0 ? marks[active] : undefined;
      const label = mark
        ? formatTimelineHistoryMarkLabel(mark, hour24Clock)
        : formatTimelineRailTimestamp(timestampMs, hour24Clock);
      if (label) track.setAttribute('aria-valuetext', label);
      else track.removeAttribute('aria-valuetext');
      ticksRef.current.forEach((tick, tickIndex) => {
        if (!tick) return;
        if (tickIndex === active) tick.setAttribute('aria-current', 'true');
        else tick.removeAttribute('aria-current');
      });
    },
    [axis, hour24Clock, marks]
  );

  useLayoutEffect(() => {
    const scrollEl = scrollRef.current;
    const sync = () => paintThumb(getVisibleTimestamp(), false);
    sync();
    if (!scrollEl) return undefined;
    const onScroll = () => paintThumb(getVisibleTimestamp(), false);
    scrollEl.addEventListener('scroll', onScroll, { passive: true });
    const track = trackRef.current;
    const observer = track ? new ResizeObserver(sync) : undefined;
    if (track) observer?.observe(track);
    return () => {
      scrollEl.removeEventListener('scroll', onScroll);
      observer?.disconnect();
    };
  }, [getVisibleTimestamp, paintThumb, scrollRef]);

  const previewFromClientY = useCallback(
    (clientY: number, commitPreview: boolean) => {
      const track = trackRef.current;
      if (!track) return;
      const rect = track.getBoundingClientRect();
      const ratio = rect.height <= 0 ? 0 : (clientY - rect.top) / rect.height;
      const timestampMs = timestampForRailRatio(ratio, axis.startMs, axis.endMs);
      lastTimestampRef.current = timestampMs;
      if (commitPreview) onPreviewTimestamp(timestampMs);
      setHoverLabel(formatTimelineRailTimestamp(timestampMs, hour24Clock));
      paintThumb(timestampMs, true);
    },
    [axis, hour24Clock, onPreviewTimestamp, paintThumb]
  );

  const onPointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    previewFromClientY(event.clientY, true);
  };
  const onPointerMove = (event: React.PointerEvent<HTMLDivElement>) => {
    previewFromClientY(event.clientY, event.currentTarget.hasPointerCapture(event.pointerId));
  };
  const onPointerUp = (event: React.PointerEvent<HTMLDivElement>) => {
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
      onCommitTimestamp(lastTimestampRef.current);
    }
    setHoverLabel(undefined);
  };

  return (
    <div className={htmlCss.DateRail} data-timeline-date-rail="true">
      <div
        ref={trackRef}
        className={htmlCss.DateRailTrack}
        role="scrollbar"
        aria-label="Jump to a date in the last 7 days"
        aria-controls="native-timeline-history"
        aria-orientation="vertical"
        aria-valuemin={axis.startMs}
        aria-valuemax={axis.endMs}
        aria-valuenow={axis.endMs}
        tabIndex={0}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
        onPointerLeave={() => setHoverLabel(undefined)}
        onKeyDown={(event) => {
          const current = getVisibleTimestamp();
          const span = Math.max(1, axis.endMs - axis.startMs);
          const step = Math.max(1, Math.round(span / 20));
          if (event.key === 'ArrowUp' || event.key === 'Home') {
            event.preventDefault();
            onCommitTimestamp(
              event.key === 'Home' ? axis.startMs : Math.max(axis.startMs, current - step)
            );
          }
          if (event.key === 'ArrowDown' || event.key === 'End') {
            event.preventDefault();
            onCommitTimestamp(
              event.key === 'End' ? axis.endMs : Math.min(axis.endMs, current + step)
            );
          }
        }}
      >
        <div ref={thumbRef} className={htmlCss.DateRailThumb} />
      </div>
      {marks.map((mark, index) => {
        const ratio = railRatioForTimestamp(mark.timestampMs, axis.startMs, axis.endMs);
        const label = formatTimelineRailTimestamp(mark.timestampMs, hour24Clock);
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
            onPointerEnter={() => {
              setHoverLabel(formatTimelineRailTimestamp(mark.timestampMs, hour24Clock));
              paintThumb(mark.timestampMs, true);
            }}
            onPointerLeave={() => {
              setHoverLabel(undefined);
            }}
            onClick={() => onCommitTimestamp(mark.timestampMs)}
          />
        );
      })}
      <div
        ref={labelRef}
        className={htmlCss.DateRailLabel}
        style={{ opacity: hoverLabel ? 1 : 0 }}
        aria-hidden={!hoverLabel}
      >
        <Text size="T200">{hoverLabel ?? '\u00a0'}</Text>
      </div>
    </div>
  );
});
