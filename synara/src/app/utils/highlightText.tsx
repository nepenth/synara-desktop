import React from 'react';
import * as css from '../styles/CustomHtml.css';
import { sanitizeForRegex } from './regex';
import { findAndReplace } from './findAndReplace';

/** A case-insensitive pattern matching any of the search terms, or none for no terms. */
export const makeHighlightRegex = (highlights: string[]): RegExp | undefined => {
  const pattern = highlights.map(sanitizeForRegex).join('|');
  if (!pattern) return undefined;
  return new RegExp(pattern, 'gi');
};

/** Marks every match of `regex` in the text parts; element parts pass through. */
export const highlightText = (
  regex: RegExp,
  data: (string | React.JSX.Element)[]
): (string | React.JSX.Element)[] =>
  data.flatMap((text) => {
    if (typeof text !== 'string') return text;

    return findAndReplace(
      text,
      regex,
      (match, pushIndex) => (
        <span key={`highlight-${pushIndex}`} className={css.highlightText}>
          {match[0]}
        </span>
      ),
      (txt) => txt
    );
  });
