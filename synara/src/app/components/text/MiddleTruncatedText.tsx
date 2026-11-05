import React, { useLayoutEffect, useState } from 'react';
import classNames from 'classnames';
import {
  MIDDLE_TRUNCATION_MIN_LENGTH,
  MIDDLE_TRUNCATION_TAIL,
  middleTruncationDisplay,
  splitForMiddleTruncation,
} from './middleTruncation';
import * as css from './MiddleTruncatedText.css';

type MiddleTruncatedTextProps = {
  text: string;
  /** Characters always shown at the end. */
  tail?: number;
  /** Text this long or shorter is one end-truncating span. */
  minLength?: number;
  className?: string;
};

/** True while `element`'s content is wider than its box; follows resizes. */
const useOverflowing = (element: HTMLElement | null, content: string | undefined): boolean => {
  const [overflowing, setOverflowing] = useState(false);
  useLayoutEffect(() => {
    if (!element) {
      setOverflowing(false);
      return undefined;
    }
    const measure = () => setOverflowing(element.scrollWidth > element.clientWidth);
    measure();
    if (typeof ResizeObserver === 'undefined') return undefined;
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [element, content]);
  return overflowing;
};

/**
 * One line of text that truncates in the middle (macOS Finder, Slack): the
 * last few characters stay, so names that share a prefix ("Project - Home…",
 * "Project - Hosting…") stay apart. Two spans in a flex row: while the start
 * overflows it is clipped under a short fade and the end span leads with the
 * ellipsis, flush against the tail. Assistive tech reads one hidden copy of
 * the full name; the visual spans are aria-hidden.
 */
export function MiddleTruncatedText({
  text,
  tail = MIDDLE_TRUNCATION_TAIL,
  minLength = MIDDLE_TRUNCATION_MIN_LENGTH,
  className,
}: MiddleTruncatedTextProps) {
  const parts = splitForMiddleTruncation(text, tail, minLength);
  const [start, setStart] = useState<HTMLSpanElement | null>(null);
  const overflowing = useOverflowing(parts ? start : null, parts?.start);
  const shown = parts ? middleTruncationDisplay(parts, overflowing) : undefined;
  return (
    <span
      className={classNames(css.MiddleTruncatedText, className)}
      data-middle-truncated={!!parts}
    >
      {shown ? (
        <>
          <span className={css.MiddleTruncatedFull}>{text}</span>
          <span
            ref={setStart}
            className={css.MiddleTruncatedStart}
            data-overflowing={overflowing}
            aria-hidden="true"
          >
            {shown.start}
          </span>
          <span className={css.MiddleTruncatedEnd} aria-hidden="true">
            {shown.end}
          </span>
        </>
      ) : (
        <span className={css.MiddleTruncatedWhole}>{text}</span>
      )}
    </span>
  );
}
