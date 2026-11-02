import { globalStyle, style } from '@vanilla-extract/css';
import { color, config, toRem } from 'folds';

/**
 * One quote recipe for the timeline reply block and the composer's
 * "Replying to" strip: a thin accent bar, no fill, 13px text, body clamped to
 * two lines. Secondary text uses the opaque colour role, not opacity.
 */
export const ReplyQuote = style({
  display: 'flex',
  flexDirection: 'column',
  gap: toRem(1),
  minWidth: 0,
  maxWidth: '100%',
  margin: 0,
  padding: `0 0 0 ${config.space.S300}`,
  border: 'none',
  borderLeft: `2px solid ${color.Primary.Main}`,
  borderRadius: 0,
  background: 'none',
  color: 'var(--synara-message-foreground)',
  font: 'inherit',
  textAlign: 'left',
  selectors: {
    'button&': {
      cursor: 'pointer',
    },
    'button&:focus-visible': {
      outline: `2px solid ${color.Primary.Main}`,
      outlineOffset: toRem(2),
      borderRadius: config.radii.R300,
    },
  },
});

export const ReplyQuoteSender = style({
  display: 'flex',
  alignItems: 'center',
  gap: config.space.S100,
  minWidth: 0,
  color: 'var(--synara-content-secondary)',
});

export const ReplyQuoteSenderName = style({
  fontSize: toRem(13),
  fontWeight: config.fontWeight.W600,
  lineHeight: 1.3,
  color: 'var(--synara-content-secondary)',
});

export const ReplyQuoteBody = style({
  display: '-webkit-box',
  WebkitLineClamp: 2,
  WebkitBoxOrient: 'vertical',
  overflow: 'hidden',
  overflowWrap: 'anywhere',
  whiteSpace: 'pre-wrap',
  fontSize: toRem(13),
  lineHeight: 1.4,
  color: 'var(--synara-content-secondary)',
});

globalStyle(`button${ReplyQuote}:hover ${ReplyQuoteSenderName}`, {
  color: 'var(--synara-content-heading)',
});
