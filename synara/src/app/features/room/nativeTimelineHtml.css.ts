import { globalStyle, style } from '@vanilla-extract/css';
import { color, config, toRem } from 'folds';

export const MessageRow = style({
  paddingTop: config.space.S300,
  paddingBottom: config.space.S300,
  paddingLeft: toRem(32),
  paddingRight: toRem(88),
});

export const MessageBody = style({
  background: color.SurfaceVariant.Container,
  borderRadius: config.radii.R400,
  padding: `${config.space.S300} ${config.space.S400}`,
  minWidth: 0,
});

export const FormattedBody = style({
  overflowWrap: 'anywhere',
  wordBreak: 'break-word',
  fontSize: 'inherit',
  lineHeight: 1.45,
});

export const CodePanel = style({
  display: 'flex',
  flexDirection: 'column',
  width: '100%',
  maxWidth: '100%',
  minWidth: 0,
  boxSizing: 'border-box',
});

export const CodeLanguage = style({
  flex: 'none',
  padding: `${config.space.S100} ${config.space.S300}`,
  borderBottom: `1px solid ${color.Background.ContainerLine}`,
  color: color.Background.OnContainer,
  fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace',
  fontSize: '0.78em',
  letterSpacing: '0.02em',
  textTransform: 'lowercase',
  userSelect: 'none',
  opacity: 0.78,
});

export const CodeScroll = style({
  display: 'flex',
  flexDirection: 'row',
  alignItems: 'flex-start',
  overflowX: 'auto',
  whiteSpace: 'pre',
  padding: `${config.space.S200} ${config.space.S300}`,
});

export const CodeLineNumbers = style({
  flex: 'none',
  userSelect: 'none',
  textAlign: 'right',
  whiteSpace: 'pre',
  lineHeight: 1.5,
  paddingRight: config.space.S200,
  marginRight: config.space.S200,
  borderRight: `1px solid ${color.Background.ContainerLine}`,
  color: color.Background.OnContainer,
  opacity: 0.48,
  fontVariantNumeric: 'tabular-nums',
});

globalStyle(`${FormattedBody} p`, {
  margin: `0 0 ${config.space.S200}`,
});
globalStyle(`${FormattedBody} p:last-child`, {
  marginBottom: 0,
});
globalStyle(
  `${FormattedBody} h1, ${FormattedBody} h2, ${FormattedBody} h3, ${FormattedBody} h4, ${FormattedBody} h5, ${FormattedBody} h6`,
  {
    margin: `${config.space.S300} 0 ${config.space.S200}`,
    fontWeight: 600,
    lineHeight: 1.25,
  }
);
globalStyle(
  `${FormattedBody} h1:first-child, ${FormattedBody} h2:first-child, ${FormattedBody} h3:first-child, ${FormattedBody} h4:first-child, ${FormattedBody} h5:first-child, ${FormattedBody} h6:first-child`,
  {
    marginTop: 0,
  }
);
globalStyle(`${FormattedBody} blockquote`, {
  margin: `${config.space.S200} 0`,
  paddingLeft: config.space.S200,
  borderLeft: `3px solid ${color.SurfaceVariant.ContainerLine}`,
  fontStyle: 'italic',
  opacity: 0.92,
});
globalStyle(`${FormattedBody} ul, ${FormattedBody} ol`, {
  margin: `${config.space.S200} 0`,
  paddingLeft: '1.6em',
});
globalStyle(`${FormattedBody} li`, {
  margin: `${config.space.S100} 0`,
});
globalStyle(`${FormattedBody} pre`, {
  width: '100%',
  maxWidth: '100%',
  minWidth: 0,
  margin: `${config.space.S200} 0`,
  padding: 0,
  overflow: 'hidden',
  overflowWrap: 'normal',
  wordBreak: 'normal',
  whiteSpace: 'normal',
  borderRadius: config.radii.R400,
  background: color.Background.Container,
  color: color.Background.OnContainer,
  border: `1px solid ${color.Background.ContainerLine}`,
});
globalStyle(`${FormattedBody} pre:last-child`, {
  marginBottom: 0,
});
globalStyle(`${FormattedBody} code`, {
  fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace',
  fontSize: '0.92em',
});
globalStyle(`${FormattedBody} :not(pre) > code`, {
  padding: `0 ${toRem(4)}`,
  borderRadius: config.radii.R300,
  background: color.SurfaceVariant.Container,
  whiteSpace: 'break-spaces',
});
globalStyle(`${FormattedBody} pre code`, {
  display: 'block',
  flex: 1,
  minWidth: 0,
  padding: 0,
  background: 'transparent',
  border: 'none',
  whiteSpace: 'pre',
  overflowWrap: 'normal',
  wordBreak: 'normal',
  lineHeight: 1.5,
  tabSize: 4,
});
globalStyle(`${FormattedBody} a`, {
  color: 'var(--tc-link)',
});
globalStyle(`${FormattedBody} hr`, {
  margin: `${config.space.S300} 0`,
  border: 'none',
  borderTop: `1px solid ${color.SurfaceVariant.ContainerLine}`,
});
globalStyle(`${FormattedBody} table`, {
  borderCollapse: 'collapse',
  margin: `${config.space.S200} 0`,
});
globalStyle(`${FormattedBody} th, ${FormattedBody} td`, {
  padding: `${config.space.S100} ${config.space.S200}`,
  border: `1px solid ${color.SurfaceVariant.ContainerLine}`,
});
globalStyle(`${FormattedBody} img`, {
  maxWidth: toRem(296),
  height: 'auto',
  borderRadius: config.radii.R300,
});
