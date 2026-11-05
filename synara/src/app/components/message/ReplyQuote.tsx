import React, { ReactNode } from 'react';
import { Icon, Icons, Text, as } from 'folds';
import classNames from 'classnames';
import * as css from './ReplyQuote.css';

type ReplyQuoteProps = {
  /** Who is being replied to; already resolved to a display name. */
  senderName: ReactNode;
  /** The quoted text; clamped to two lines by the recipe. */
  body: ReactNode;
};

/**
 * The shared "reply quote" block: the timeline renders it as a button that
 * opens the quoted message, the composer renders it as the "Replying to"
 * preview. Both read the same way: accent bar, name at 13/600, two-line body.
 */
export const ReplyQuote = as<'div', ReplyQuoteProps>(
  ({ as: Component = 'div', senderName, body, className, ...props }, ref) => (
    <Component className={classNames(css.ReplyQuote, className)} {...props} ref={ref}>
      <span className={css.ReplyQuoteSender}>
        <Icon size="50" src={Icons.ReplyArrow} />
        <Text as="span" className={css.ReplyQuoteSenderName} truncate>
          {senderName}
        </Text>
      </span>
      <Text as="span" className={css.ReplyQuoteBody}>
        {body}
      </Text>
    </Component>
  )
);
