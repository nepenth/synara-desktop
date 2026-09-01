import { globalStyle, style } from '@vanilla-extract/css';
import { recipe } from '@vanilla-extract/recipes';
import { color, config, DefaultReset, toRem } from 'folds';
import { ContainerColor } from './ContainerColor.css';

export const MarginSpaced = style({
  marginBottom: config.space.S200,
  marginTop: config.space.S200,
  selectors: {
    '&:first-child': {
      marginTop: 0,
    },
    '&:last-child': {
      marginBottom: 0,
    },
  },
});

export const Paragraph = style([DefaultReset]);

export const Heading = style([
  DefaultReset,
  MarginSpaced,
  {
    marginTop: config.space.S400,
    selectors: {
      '&:first-child': {
        marginTop: 0,
      },
    },
  },
]);

export const BlockQuote = style([
  DefaultReset,
  MarginSpaced,
  {
    paddingLeft: config.space.S200,
    borderLeft: `${config.borderWidth.B700} solid ${color.SurfaceVariant.ContainerLine}`,
    fontStyle: 'italic',
  },
]);

const BaseCode = style({
  color: 'var(--synara-rich-text-inline-code-foreground)',
  background: 'var(--synara-rich-text-inline-code-background)',
  border: `${config.borderWidth.B300} solid var(--synara-rich-text-inline-code-border)`,
  borderRadius: config.radii.R300,
  '@media': {
    '(prefers-contrast: more)': {
      color: 'var(--synara-rich-text-contrast-foreground)',
      borderColor: 'var(--synara-rich-text-contrast-border)',
    },
  },
});
const CodeFont = style({
  fontFamily:
    'ui-monospace, SFMono-Regular, Menlo, Consolas, "Liberation Mono", "DejaVu Sans Mono", "Noto Sans Mono", monospace',
  fontSize: '0.94em',
});

export const Code = style([
  DefaultReset,
  BaseCode,
  CodeFont,
  {
    padding: `0 ${config.space.S100}`,
  },
]);

export const Spoiler = recipe({
  base: [
    DefaultReset,
    {
      padding: `0 ${config.space.S100}`,
      backgroundColor: 'var(--synara-rich-text-spoiler-background)',
      border: `${config.borderWidth.B300} solid var(--synara-rich-text-spoiler-border)`,
      borderRadius: config.radii.R300,
      selectors: {
        '&:hover': {
          backgroundColor: 'var(--synara-rich-text-spoiler-hover)',
        },
        '&[aria-pressed=true]': {
          color: 'transparent',
        },
      },
      '@media': {
        '(prefers-contrast: more)': {
          borderColor: 'var(--synara-rich-text-contrast-border)',
        },
      },
    },
  ],
  variants: {
    active: {
      true: {
        color: 'transparent',
      },
    },
  },
});

globalStyle(`${Spoiler()}[aria-pressed=true] [aria-hidden=true]`, {
  color: 'transparent !important',
  backgroundColor: 'transparent !important',
});

export const CodeBlock = style([
  DefaultReset,
  BaseCode,
  MarginSpaced,
  {
    contain: 'layout paint style',
    fontStyle: 'normal',
    position: 'relative',
    overflow: 'hidden',
    overflowAnchor: 'none',
    color: 'var(--synara-rich-text-inline-code-foreground)',
    background: 'var(--synara-rich-text-code-block-background)',
    border: `${config.borderWidth.B300} solid var(--synara-rich-text-code-block-border)`,
    '@media': {
      '(prefers-contrast: more)': {
        color: 'var(--synara-rich-text-contrast-foreground)',
        borderColor: 'var(--synara-rich-text-contrast-border)',
      },
    },
  },
]);
export const CodeBlockHeader = style([
  ContainerColor({ variant: 'Surface' }),
  {
    padding: `0 ${config.space.S200} 0 ${config.space.S300}`,
    borderBottomWidth: config.borderWidth.B300,
    gap: config.space.S200,
  },
]);
export const CodeBlockInternal = style([
  CodeFont,
  {
    overflowAnchor: 'none',
    padding: `${config.space.S200} ${config.space.S200} 0`,
    minWidth: toRem(200),
  },
]);

export const Strong = style({
  fontWeight: 600,
  '@media': {
    '(prefers-contrast: more)': {
      fontWeight: 700,
    },
  },
});

export const TableScroll = style({
  width: '100%',
  maxWidth: '100%',
  margin: `${config.space.S400} 0`,
  overflowX: 'auto',
  border: `${config.borderWidth.B300} solid var(--synara-content-separator)`,
  borderRadius: config.radii.R400,
  background: 'var(--synara-rich-text-table-canvas)',
  '@media': {
    '(prefers-contrast: more)': {
      borderColor: 'var(--synara-rich-text-contrast-border)',
    },
  },
});

globalStyle(`${TableScroll} table`, {
  width: 'max-content',
  minWidth: '100%',
  borderCollapse: 'separate',
  borderSpacing: 0,
  background: 'var(--synara-rich-text-table-canvas)',
});
globalStyle(`${TableScroll} th, ${TableScroll} td`, {
  minWidth: toRem(112),
  maxWidth: toRem(360),
  padding: `${toRem(10)} ${toRem(14)}`,
  borderRight: `${config.borderWidth.B300} solid var(--synara-content-separator)`,
  borderBottom: `${config.borderWidth.B300} solid var(--synara-content-separator)`,
  textAlign: 'left',
  verticalAlign: 'top',
});
globalStyle(`${TableScroll} th`, {
  background: 'var(--synara-rich-text-table-header)',
  fontWeight: 600,
});
globalStyle(`${TableScroll} tbody tr:nth-child(odd) td`, {
  background: 'var(--synara-rich-text-table-odd)',
});
globalStyle(`${TableScroll} tbody tr:nth-child(even) td`, {
  background: 'var(--synara-rich-text-table-even)',
});
globalStyle(`${TableScroll} tbody tr:hover td`, {
  background: 'var(--synara-rich-text-table-hover)',
});
globalStyle(`${TableScroll} th:last-child, ${TableScroll} td:last-child`, {
  borderRight: 'none',
});
globalStyle(`${TableScroll} tr:last-child td`, {
  borderBottom: 'none',
});

export const CodeBlockBottomShadow = style({
  position: 'absolute',
  bottom: 0,
  left: 0,
  right: 0,
  pointerEvents: 'none',

  height: config.space.S400,
  background: `linear-gradient(to top, #00000022, #00000000)`,
});

export const List = style([
  DefaultReset,
  MarginSpaced,
  {
    paddingBlock: 0,
    paddingInlineEnd: config.space.S100,
    paddingInlineStart: '2.75em',
    listStylePosition: 'outside',
    selectors: {
      'ol ol&': {
        listStyleType: 'lower-alpha',
      },
      'ol ol ol&': {
        listStyleType: 'lower-roman',
      },
      'ul ul&': {
        listStyleType: 'circle',
      },
      'ul ul ul&': {
        listStyleType: 'square',
      },
    },
  },
]);

export const Img = style([
  DefaultReset,
  MarginSpaced,
  {
    maxWidth: toRem(296),
    borderRadius: config.radii.R300,
  },
]);

export const InlineChromiumBugfix = style({
  fontSize: 0,
  lineHeight: 0,
});

export const Mention = recipe({
  base: [
    DefaultReset,
    {
      backgroundColor: color.SurfaceVariant.Container,
      color: color.SurfaceVariant.OnContainer,
      boxShadow: `0 0 0 ${config.borderWidth.B300} ${color.SurfaceVariant.ContainerLine}`,
      padding: `0 ${toRem(2)}`,
      borderRadius: config.radii.R300,
      fontWeight: config.fontWeight.W500,
    },
  ],
  variants: {
    highlight: {
      true: {
        backgroundColor: color.Success.Container,
        color: color.Success.OnContainer,
        boxShadow: `0 0 0 ${config.borderWidth.B300} ${color.Success.ContainerLine}`,
      },
    },
    focus: {
      true: {
        boxShadow: `0 0 0 ${config.borderWidth.B300} ${color.SurfaceVariant.OnContainer}`,
      },
    },
  },
});

export const Command = recipe({
  base: [
    DefaultReset,
    {
      padding: `0 ${toRem(2)}`,
      borderRadius: config.radii.R300,
      fontWeight: config.fontWeight.W500,
    },
  ],
  variants: {
    focus: {
      true: {
        boxShadow: `0 0 0 ${config.borderWidth.B300} ${color.Warning.OnContainer}`,
      },
    },
    active: {
      true: {
        backgroundColor: color.Warning.Container,
        color: color.Warning.OnContainer,
        boxShadow: `0 0 0 ${config.borderWidth.B300} ${color.Warning.ContainerLine}`,
      },
    },
  },
});

export const EmoticonBase = style([
  DefaultReset,
  {
    display: 'inline-block',
    padding: '0.05rem',
    height: '1em',
    verticalAlign: 'middle',
  },
]);

export const Emoticon = recipe({
  base: [
    DefaultReset,
    {
      display: 'inline-flex',
      justifyContent: 'center',
      alignItems: 'center',

      height: '1em',
      minWidth: '1em',
      fontSize: '1.33em',
      lineHeight: '1em',
      verticalAlign: 'middle',
      position: 'relative',
      top: '-0.35em',
      borderRadius: config.radii.R300,
    },
  ],
  variants: {
    focus: {
      true: {
        boxShadow: `0 0 0 ${config.borderWidth.B300} ${color.SurfaceVariant.OnContainer}`,
      },
    },
  },
});

export const EmoticonImg = style([
  DefaultReset,
  {
    height: '1em',
    cursor: 'default',
  },
]);

export const highlightText = style([
  DefaultReset,
  {
    backgroundColor: 'yellow',
    color: 'black',
  },
]);
