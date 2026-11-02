import React, { ReactNode } from 'react';
import { Icon, Icons, Text } from 'folds';
import classNames from 'classnames';
import * as css from './InlineMessage.css';

export type InlineMessageTone = 'critical' | 'success' | 'info';

type InlineMessageProps = {
  tone?: InlineMessageTone;
  /** Defaults to `alert` for critical messages and `status` otherwise. */
  role?: 'alert' | 'status';
  className?: string;
  children: ReactNode;
};

const GLYPH = {
  critical: Icons.Warning,
  success: Icons.Check,
  info: Icons.Info,
} as const;

/**
 * The one treatment for inline errors and status lines in settings forms and
 * dialogs: tone colour plus a glyph, regular weight, T200. Errors are never
 * grey and never bold.
 */
export function InlineMessage({ tone = 'info', role, className, children }: InlineMessageProps) {
  return (
    <div
      className={classNames(css.InlineMessage, css.tone[tone], className)}
      role={role ?? (tone === 'critical' ? 'alert' : 'status')}
    >
      <Icon className={css.Glyph} src={GLYPH[tone]} size="50" aria-hidden />
      <Text as="span" size="T200" className={css.Copy}>
        {children}
      </Text>
    </div>
  );
}
