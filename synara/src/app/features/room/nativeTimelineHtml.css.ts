import { createVar, globalStyle, style } from '@vanilla-extract/css';
import { recipe } from '@vanilla-extract/recipes';
import { color, config, toRem } from 'folds';
import { NATIVE_SYNTAX_PALETTES, type NativeSyntaxPalette } from './nativeTimelineSyntaxPalette';

const syntaxMeta = createVar();
const syntaxComment = createVar();
const syntaxPunctuation = createVar();
const syntaxProperty = createVar();
const syntaxNumber = createVar();
const syntaxString = createVar();
const syntaxOperator = createVar();
const syntaxFunction = createVar();
const syntaxKeyword = createVar();
const syntaxRegex = createVar();

const syntaxPaletteVars = (palette: NativeSyntaxPalette) => ({
  [syntaxMeta]: palette.meta,
  [syntaxComment]: palette.comment,
  [syntaxPunctuation]: palette.punctuation,
  [syntaxProperty]: palette.property,
  [syntaxNumber]: palette.number,
  [syntaxString]: palette.string,
  [syntaxOperator]: palette.operator,
  [syntaxFunction]: palette.function,
  [syntaxKeyword]: palette.keyword,
  [syntaxRegex]: palette.regex,
});

export const MessageActionSurface = style({
  position: 'relative',
});

export const MessageActionRail = style({
  position: 'absolute',
  top: config.space.S100,
  right: `calc(${config.space.S400} + ${config.space.S200})`,
  zIndex: 2,
});

export const MessageRow = recipe({
  base: {
    paddingTop: config.space.S200,
    paddingBottom: config.space.S200,
    paddingLeft: toRem(16),
    paddingRight: toRem(72),
    marginLeft: config.space.S200,
    marginRight: config.space.S200,
    boxSizing: 'border-box',
    color: 'var(--synara-message-foreground)',
  },
  variants: {
    surface: {
      true: {
        borderRadius: config.radii.R400,
        selectors: {
          [`${MessageActionSurface}:hover &`]: {
            backgroundColor: color.SurfaceVariant.ContainerHover,
          },
          [`${MessageActionSurface}:focus-within &`]: {
            backgroundColor: color.SurfaceVariant.ContainerHover,
          },
        },
      },
      false: {},
    },
    grouped: {
      true: {
        paddingTop: config.space.S100,
      },
      false: {
        paddingTop: config.space.S400,
      },
    },
    groupsNext: {
      true: {
        paddingBottom: config.space.S100,
      },
      false: {},
    },
  },
  defaultVariants: {
    surface: true,
    grouped: false,
    groupsNext: false,
  },
});

export const MessageBody = style({
  background: 'transparent',
  padding: 0,
  width: '100%',
  maxWidth: toRem(672),
  minWidth: 0,
  fontSize: toRem(16),
  fontWeight: 400,
  lineHeight: 1.55,
  color: 'var(--synara-message-foreground)',
});

export const FormattedBody = style({
  overflowWrap: 'break-word',
  wordBreak: 'normal',
  fontSize: toRem(16),
  fontWeight: 400,
  lineHeight: 1.55,
  color: 'var(--synara-message-foreground)',
  vars: syntaxPaletteVars(NATIVE_SYNTAX_PALETTES.light),
});

export const SpoilerButton = style({
  display: 'inline',
  padding: `0 ${toRem(5)}`,
  border: `1px solid var(--synara-content-separator)`,
  borderRadius: config.radii.R300,
  background: color.SurfaceVariant.Container,
  color: 'var(--synara-content-secondary)',
  font: 'inherit',
  lineHeight: 'inherit',
  cursor: 'pointer',
  selectors: {
    '&:hover': {
      background: color.SurfaceVariant.ContainerHover,
    },
    '&:focus-visible': {
      outline: `2px solid ${color.Primary.Main}`,
      outlineOffset: toRem(2),
    },
  },
});

export const SpoilerContent = style({
  padding: `0 ${toRem(4)}`,
  borderRadius: config.radii.R300,
  background: color.SurfaceVariant.Container,
});

export const InlineImageFallback = style({
  display: 'inline-block',
  padding: `0 ${toRem(4)}`,
  borderRadius: config.radii.R300,
  background: color.SurfaceVariant.Container,
  color: 'var(--synara-content-secondary)',
  fontStyle: 'italic',
});

export const SenderName = style({
  color: 'var(--synara-content-heading)',
  fontWeight: 600,
});

export const Metadata = style({
  color: 'var(--synara-content-secondary)',
});

export const ReplySurface = style({
  width: '100%',
  maxWidth: toRem(672),
  padding: `${config.space.S200} ${config.space.S300}`,
  margin: `${config.space.S100} 0`,
  border: 'none',
  borderLeft: `3px solid var(--synara-content-separator)`,
  borderRadius: `0 ${config.radii.R300} ${config.radii.R300} 0`,
  background: 'var(--synara-table-even)',
  color: 'var(--synara-message-foreground)',
  textAlign: 'left',
  cursor: 'pointer',
  selectors: {
    '&:hover, &:focus-visible': {
      background: 'var(--synara-table-hover)',
    },
    '&:focus-visible': {
      outline: `2px solid ${color.Primary.Main}`,
      outlineOffset: toRem(2),
    },
  },
});

export const SystemRow = style({
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  gap: config.space.S300,
  color: 'var(--synara-content-tertiary)',
  fontSize: toRem(13),
  lineHeight: 1.35,
  textAlign: 'center',
});

export const SystemRule = style({
  flex: 1,
  height: 1,
  maxWidth: toRem(160),
  background: 'var(--synara-content-separator)',
});

export const UnreadSystemRow = style({
  color: color.Primary.Main,
  fontWeight: 600,
});

globalStyle(`.prism-light ${FormattedBody}`, {
  vars: syntaxPaletteVars(NATIVE_SYNTAX_PALETTES.light),
  '@media': {
    '(prefers-contrast: more)': {
      vars: syntaxPaletteVars(NATIVE_SYNTAX_PALETTES.moreLight),
    },
  },
});

globalStyle(`.prism-dark ${FormattedBody}`, {
  vars: syntaxPaletteVars(NATIVE_SYNTAX_PALETTES.dark),
  '@media': {
    '(prefers-contrast: more)': {
      vars: syntaxPaletteVars(NATIVE_SYNTAX_PALETTES.moreDark),
    },
  },
});

export const CodePanel = style({
  display: 'flex',
  flexDirection: 'column',
  width: '100%',
  maxWidth: '100%',
  minWidth: 0,
  boxSizing: 'border-box',
});

const codeMono = {
  fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace',
  fontSize: '0.92em',
  lineHeight: 1.5,
} as const;

export const CodeLanguage = style({
  flex: 'none',
  padding: `${config.space.S100} ${config.space.S300}`,
  borderBottom: `1px solid ${color.Surface.ContainerLine}`,
  color: syntaxMeta,
  fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace',
  fontSize: '0.78em',
  letterSpacing: '0.02em',
  textTransform: 'lowercase',
  userSelect: 'none',
});

export const CodeRow = style({
  display: 'flex',
  flexDirection: 'row',
  alignItems: 'flex-start',
  minWidth: 0,
  padding: `${config.space.S200} ${config.space.S300}`,
  ...codeMono,
});

export const CodeScroll = style({
  flex: 1,
  minWidth: 0,
  overflowX: 'auto',
  whiteSpace: 'pre',
  lineHeight: 1.5,
});

export const CodeLineNumbers = style({
  flex: 'none',
  userSelect: 'none',
  textAlign: 'right',
  whiteSpace: 'pre',
  lineHeight: 1.5,
  paddingRight: config.space.S200,
  marginRight: config.space.S200,
  borderRight: `1px solid ${color.Surface.ContainerLine}`,
  color: syntaxMeta,
  background: color.Surface.Container,
  fontVariantNumeric: 'tabular-nums',
  fontFamily: 'inherit',
  fontSize: 'inherit',
});

globalStyle(`${FormattedBody} p`, {
  margin: `0 0 ${config.space.S300}`,
});
globalStyle(`${FormattedBody} p:last-child`, {
  marginBottom: 0,
});
globalStyle(
  `${FormattedBody} h1, ${FormattedBody} h2, ${FormattedBody} h3, ${FormattedBody} h4, ${FormattedBody} h5, ${FormattedBody} h6`,
  {
    margin: `${config.space.S500} 0 ${config.space.S300}`,
    fontWeight: 600,
    lineHeight: 1.3,
    color: 'var(--synara-content-heading)',
  }
);
globalStyle(`${FormattedBody} h1`, { fontSize: toRem(24), lineHeight: 1.22 });
globalStyle(`${FormattedBody} h2`, { fontSize: toRem(21), lineHeight: 1.25 });
globalStyle(`${FormattedBody} h3`, { fontSize: toRem(18), lineHeight: 1.3 });
globalStyle(`${FormattedBody} h4, ${FormattedBody} h5, ${FormattedBody} h6`, {
  fontSize: toRem(16),
});
globalStyle(
  `${FormattedBody} h1:first-child, ${FormattedBody} h2:first-child, ${FormattedBody} h3:first-child, ${FormattedBody} h4:first-child, ${FormattedBody} h5:first-child, ${FormattedBody} h6:first-child`,
  {
    marginTop: 0,
  }
);
globalStyle(`${FormattedBody} blockquote`, {
  margin: `${config.space.S300} 0`,
  paddingLeft: config.space.S300,
  borderLeft: `3px solid var(--synara-content-separator)`,
  fontStyle: 'italic',
  color: 'var(--synara-content-secondary)',
});
globalStyle(`${FormattedBody} ul, ${FormattedBody} ol`, {
  margin: `${config.space.S300} 0`,
  paddingLeft: '1.6em',
});
globalStyle(`${FormattedBody} li`, {
  margin: `${config.space.S200} 0`,
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
  background: color.Surface.Container,
  color: color.Surface.OnContainer,
  border: `1px solid ${color.Surface.ContainerLine}`,
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
  width: 'max-content',
  minWidth: '100%',
  padding: 0,
  background: 'transparent',
  border: 'none',
  whiteSpace: 'pre',
  overflowWrap: 'normal',
  wordBreak: 'normal',
  fontFamily: 'inherit',
  fontSize: 'inherit',
  lineHeight: 1.5,
  tabSize: 4,
});
globalStyle(`${FormattedBody} a`, {
  color: 'var(--tc-link)',
  textDecoration: 'underline',
  textUnderlineOffset: toRem(2),
});
globalStyle(`${FormattedBody} hr`, {
  margin: `${config.space.S300} 0`,
  border: 'none',
  borderTop: `1px solid ${color.SurfaceVariant.ContainerLine}`,
});
export const TableScroll = style({
  width: '100%',
  maxWidth: '100%',
  margin: `${config.space.S400} 0`,
  overflowX: 'auto',
  border: `1px solid var(--synara-content-separator)`,
  borderRadius: config.radii.R400,
  background: 'var(--synara-table-canvas)',
  boxShadow: 'inset -14px 0 16px -18px rgba(0, 0, 0, 0.75)',
  selectors: {
    '&:focus-visible': {
      outline: `2px solid ${color.Primary.Main}`,
      outlineOffset: toRem(2),
    },
  },
});

globalStyle(`${TableScroll} table`, {
  width: 'max-content',
  minWidth: '100%',
  borderCollapse: 'separate',
  borderSpacing: 0,
  margin: 0,
  background: 'var(--synara-table-canvas)',
});
globalStyle(`${TableScroll} th, ${TableScroll} td`, {
  minWidth: toRem(112),
  maxWidth: toRem(360),
  padding: `${toRem(10)} ${toRem(14)}`,
  borderRight: `1px solid var(--synara-content-separator)`,
  borderBottom: `1px solid var(--synara-content-separator)`,
  textAlign: 'left',
  verticalAlign: 'top',
  overflowWrap: 'normal',
  wordBreak: 'normal',
});
globalStyle(`${TableScroll} th:last-child, ${TableScroll} td:last-child`, {
  borderRight: 'none',
});
globalStyle(`${TableScroll} tr:last-child td`, {
  borderBottom: 'none',
});
globalStyle(`${TableScroll} th`, {
  background: 'var(--synara-table-header)',
  color: 'var(--synara-content-heading)',
  fontWeight: 600,
});
globalStyle(`${TableScroll} tbody tr:nth-child(odd) td`, {
  background: 'var(--synara-table-odd)',
});
globalStyle(`${TableScroll} tr:nth-child(odd) td`, {
  background: 'var(--synara-table-odd)',
});
globalStyle(`${TableScroll} tbody tr:nth-child(even) td`, {
  background: 'var(--synara-table-even)',
});
globalStyle(`${TableScroll} tr:nth-child(even) td`, {
  background: 'var(--synara-table-even)',
});
globalStyle(`${TableScroll} tbody tr:hover td`, {
  background: 'var(--synara-table-hover)',
});
globalStyle(`${TableScroll} tr:hover td`, {
  background: 'var(--synara-table-hover)',
});
globalStyle(`${TableScroll} td code`, {
  whiteSpace: 'nowrap',
});
globalStyle(`${FormattedBody} img`, {
  maxWidth: toRem(296),
  height: 'auto',
  borderRadius: config.radii.R300,
});
globalStyle(
  `${FormattedBody} code .token.comment, ${FormattedBody} code .token.prolog, ${FormattedBody} code .token.doctype, ${FormattedBody} code .token.cdata`,
  {
    color: syntaxComment,
  }
);
globalStyle(`${FormattedBody} code .token.punctuation`, {
  color: syntaxPunctuation,
});
globalStyle(`${FormattedBody} code .token.namespace`, {
  color: syntaxMeta,
  opacity: 1,
});
globalStyle(
  `${FormattedBody} code .token.property, ${FormattedBody} code .token.tag, ${FormattedBody} code .token.constant, ${FormattedBody} code .token.symbol, ${FormattedBody} code .token.deleted`,
  {
    color: syntaxProperty,
  }
);
globalStyle(`${FormattedBody} code .token.boolean, ${FormattedBody} code .token.number`, {
  color: syntaxNumber,
});
globalStyle(
  `${FormattedBody} code .token.selector, ${FormattedBody} code .token.attr-name, ${FormattedBody} code .token.string, ${FormattedBody} code .token.char, ${FormattedBody} code .token.builtin, ${FormattedBody} code .token.inserted`,
  {
    color: syntaxString,
  }
);
globalStyle(
  `${FormattedBody} code .token.operator, ${FormattedBody} code .token.entity, ${FormattedBody} code .token.url, ${FormattedBody} code .token.variable`,
  {
    color: syntaxOperator,
  }
);
globalStyle(
  `${FormattedBody} code .token.atrule, ${FormattedBody} code .token.attr-value, ${FormattedBody} code .token.function, ${FormattedBody} code .token.class-name`,
  {
    color: syntaxFunction,
  }
);
globalStyle(`${FormattedBody} code .token.keyword`, {
  color: syntaxKeyword,
});
globalStyle(`${FormattedBody} code .token.regex, ${FormattedBody} code .token.important`, {
  color: syntaxRegex,
});
