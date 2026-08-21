import { globalStyle, style } from '@vanilla-extract/css';
import { recipe } from '@vanilla-extract/recipes';
import { color, config, toRem } from 'folds';

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
    paddingTop: config.space.S300,
    paddingBottom: config.space.S300,
    paddingLeft: toRem(32),
    paddingRight: toRem(88),
    marginLeft: config.space.S400,
    marginRight: config.space.S400,
    marginBottom: config.space.S200,
    boxSizing: 'border-box',
    color: color.SurfaceVariant.OnContainer,
  },
  variants: {
    surface: {
      true: {
        backgroundColor: color.SurfaceVariant.ContainerHover,
        borderRadius: config.radii.R400,
        selectors: {
          [`${MessageActionSurface}:hover &`]: {
            backgroundColor: color.SurfaceVariant.ContainerActive,
          },
          [`${MessageActionSurface}:focus-within &`]: {
            backgroundColor: color.SurfaceVariant.ContainerActive,
          },
          '&:hover': {
            backgroundColor: color.SurfaceVariant.ContainerActive,
          },
        },
      },
      false: {},
    },
    grouped: {
      true: {
        paddingTop: config.space.S100,
        borderTopLeftRadius: 0,
        borderTopRightRadius: 0,
      },
      false: {},
    },
    groupsNext: {
      true: {
        paddingBottom: config.space.S100,
        borderBottomLeftRadius: 0,
        borderBottomRightRadius: 0,
        marginBottom: 0,
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
  minWidth: 0,
  fontSize: toRem(14),
  fontWeight: 400,
  lineHeight: 1.55,
  color: color.SurfaceVariant.OnContainer,
});

export const FormattedBody = style({
  overflowWrap: 'anywhere',
  wordBreak: 'break-word',
  fontSize: toRem(14),
  fontWeight: 400,
  lineHeight: 1.55,
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
  color: color.Surface.OnContainer,
  fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace',
  fontSize: '0.78em',
  letterSpacing: '0.02em',
  textTransform: 'lowercase',
  userSelect: 'none',
  opacity: 0.78,
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
  color: color.Surface.OnContainer,
  background: color.Surface.Container,
  opacity: 0.48,
  fontVariantNumeric: 'tabular-nums',
  fontFamily: 'inherit',
  fontSize: 'inherit',
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
globalStyle(
  `${FormattedBody} code .token.comment, ${FormattedBody} code .token.prolog, ${FormattedBody} code .token.doctype, ${FormattedBody} code .token.cdata`,
  {
    color: '#7a8478',
  }
);
globalStyle(`${FormattedBody} code .token.punctuation`, {
  color: '#9aa0a6',
});
globalStyle(
  `${FormattedBody} code .token.property, ${FormattedBody} code .token.tag, ${FormattedBody} code .token.constant, ${FormattedBody} code .token.symbol, ${FormattedBody} code .token.deleted`,
  {
    color: '#e06c75',
  }
);
globalStyle(`${FormattedBody} code .token.boolean, ${FormattedBody} code .token.number`, {
  color: '#d19a66',
});
globalStyle(
  `${FormattedBody} code .token.selector, ${FormattedBody} code .token.attr-name, ${FormattedBody} code .token.string, ${FormattedBody} code .token.char, ${FormattedBody} code .token.builtin, ${FormattedBody} code .token.inserted`,
  {
    color: '#98c379',
  }
);
globalStyle(
  `${FormattedBody} code .token.operator, ${FormattedBody} code .token.entity, ${FormattedBody} code .token.url, ${FormattedBody} code .token.variable`,
  {
    color: '#56b6c2',
  }
);
globalStyle(
  `${FormattedBody} code .token.atrule, ${FormattedBody} code .token.attr-value, ${FormattedBody} code .token.function, ${FormattedBody} code .token.class-name`,
  {
    color: '#61afef',
  }
);
globalStyle(`${FormattedBody} code .token.keyword`, {
  color: '#c678dd',
});
globalStyle(`${FormattedBody} code .token.regex, ${FormattedBody} code .token.important`, {
  color: '#e5c07b',
});
