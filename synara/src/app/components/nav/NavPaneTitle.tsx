import React from 'react';
import { MiddleTruncatedText } from '../text';
import * as css from './styles.css';

/** Host names are short; split them sooner so the domain's end stays visible. */
const PANE_TITLE_MIN_LENGTH = 12;

/**
 * The nav pane header's title (homeserver or space name): 15px semibold and
 * middle-truncated, "matrix.whyl…and.net" rather than "matrix.whyl…".
 */
export function NavPaneTitle({ text }: { text: string }) {
  return (
    <div className={css.NavPaneTitle} title={text} data-testid="nav-pane-title">
      <MiddleTruncatedText text={text} minLength={PANE_TITLE_MIN_LENGTH} />
    </div>
  );
}
