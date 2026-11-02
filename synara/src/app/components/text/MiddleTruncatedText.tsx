import React from 'react';
import classNames from 'classnames';
import {
  MIDDLE_TRUNCATION_MIN_LENGTH,
  MIDDLE_TRUNCATION_TAIL,
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

/**
 * One line of text that truncates in the middle (macOS Finder, Slack): the
 * start span ellipsizes and the last few characters stay, so names that share
 * a prefix ("Project - Home…", "Project - Hosting…") stay apart. Two spans in a
 * flex row, no measuring; the full text is still the element's text content.
 */
export function MiddleTruncatedText({
  text,
  tail = MIDDLE_TRUNCATION_TAIL,
  minLength = MIDDLE_TRUNCATION_MIN_LENGTH,
  className,
}: MiddleTruncatedTextProps) {
  const parts = splitForMiddleTruncation(text, tail, minLength);
  return (
    <span
      className={classNames(css.MiddleTruncatedText, className)}
      data-middle-truncated={!!parts}
    >
      {parts ? (
        <>
          <span className={css.MiddleTruncatedStart}>{parts.start}</span>
          <span className={css.MiddleTruncatedEnd}>{parts.end}</span>
        </>
      ) : (
        <span className={css.MiddleTruncatedStart}>{text}</span>
      )}
    </span>
  );
}
