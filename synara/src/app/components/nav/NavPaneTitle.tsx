import React from 'react';
import * as css from './styles.css';

/**
 * The nav pane header's title (homeserver or space name): 15px semibold with
 * ordinary end truncation, taking the width left beside the header buttons.
 * Middle truncation is for room rows only; on a host name it read oddly.
 */
export function NavPaneTitle({ text }: { text: string }) {
  return (
    <div className={css.NavPaneTitle} title={text} data-testid="nav-pane-title">
      {text}
    </div>
  );
}
