import { createVar, globalStyle, style } from '@vanilla-extract/css';
import { recipe } from '@vanilla-extract/recipes';
import { color, config, toRem } from 'folds';
import { NATIVE_SYNTAX_PALETTES, type NativeSyntaxPalette } from './nativeTimelineSyntaxPalette';
import {
  avatarSurface,
  floatingSurface,
  quietEdgeLight,
  raisedShadow,
  tactileSurface,
} from '../../styles/Depth.css';

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

export const MessageSwipeSurface = style({
  position: 'relative',
});

export const MessageSwipeContent = style({
  position: 'relative',
  zIndex: 1,
});

export const GroupedTimestampReveal = style({
  position: 'absolute',
  zIndex: 2,
  top: '50%',
  right: config.space.S300,
  transform: 'translateY(-50%)',
  color: 'var(--synara-content-secondary)',
  opacity: 0,
  transition: 'opacity 120ms ease-out',
  pointerEvents: 'none',
  whiteSpace: 'nowrap',
  selectors: {
    [`${MessageActionSurface}:hover &`]: {
      opacity: 1,
    },
    [`${MessageActionSurface}:focus-within &`]: {
      opacity: 1,
    },
  },
  '@media': {
    '(prefers-reduced-motion: reduce)': {
      transition: 'none',
    },
  },
});

export const MessageActionRail = style([
  floatingSurface,
  tactileSurface,
  {
    position: 'absolute',
    top: config.space.S100,
    right: `calc(${config.space.S400} + ${config.space.S200})`,
    zIndex: 2,
    borderRadius: config.radii.R400,
    overflow: 'hidden',
  },
]);

// Popouts use the same floating plane as the reaction picker and action rail.
export const MessageActionMenu = style([
  floatingSurface,
  {
    borderRadius: config.radii.R400,
    maxWidth: 'calc(100vw - 24px)',
    maxHeight: 'calc(100dvh - 24px)',
    overflowY: 'auto',
  },
]);

/**
 * A notice above the timeline: an action that failed (`error`, same surface
 * as the composer's send error) or why a routed message is not shown (`info`).
 */
export const TimelineNotice = recipe({
  base: {
    flexShrink: 0,
    margin: `${config.space.S200} ${config.space.S300} 0`,
    padding: `${config.space.S200} ${config.space.S300}`,
    border: `${config.borderWidth.B300} solid`,
    borderRadius: config.radii.R300,
  },
  variants: {
    tone: {
      error: {
        borderColor: color.Critical.Main,
        background: color.Critical.Container,
        color: color.Critical.OnContainer,
      },
      info: {
        borderColor: color.SurfaceVariant.ContainerLine,
        background: color.SurfaceVariant.Container,
        color: color.SurfaceVariant.OnContainer,
      },
    },
  },
  defaultVariants: { tone: 'error' },
});

// The rail clips its children (`overflow: hidden`), which hid the folds
// outline on its buttons. An inset ring in Primary stays inside the clip and
// reaches 3:1 on both themes.
globalStyle(`${MessageActionRail} button:focus-visible`, {
  outline: `${config.borderWidth.B500} solid ${color.Primary.Main}`,
  outlineOffset: `calc(-1 * ${config.borderWidth.B500})`,
});

// The row itself takes focus so keyboard users can reach its actions.
globalStyle(`${MessageActionSurface}:focus-visible`, {
  outline: `${config.borderWidth.B500} solid ${color.Primary.Main}`,
  outlineOffset: `calc(-1 * ${config.borderWidth.B500})`,
  borderRadius: config.radii.R400,
});

/** Image or video that waits for a click while "Load Media Automatically" is off. */
export const MediaLoadPlaceholder = style({
  display: 'inline-flex',
  alignItems: 'center',
  justifyContent: 'center',
  gap: config.space.S200,
  maxWidth: '100%',
  boxSizing: 'border-box',
  border: `${config.borderWidth.B300} dashed ${color.SurfaceVariant.ContainerLine}`,
  borderRadius: config.radii.R400,
  background: color.SurfaceVariant.Container,
  color: color.SurfaceVariant.OnContainer,
  cursor: 'pointer',
  selectors: {
    '&:hover': { background: color.SurfaceVariant.ContainerHover },
    '&:focus-visible': {
      outline: `${config.borderWidth.B500} solid ${color.Primary.Main}`,
      outlineOffset: config.borderWidth.B300,
    },
  },
});

/** An inline image or video that opens the media viewer; it keeps the media's own box. */
export const MediaOpenButton = style({
  display: 'block',
  alignSelf: 'flex-start',
  maxWidth: '100%',
  lineHeight: 0,
  padding: 0,
  margin: 0,
  border: 'none',
  borderRadius: config.radii.R400,
  background: 'none',
  overflow: 'hidden',
  cursor: 'zoom-in',
  selectors: {
    '&:focus-visible': {
      outline: `${config.borderWidth.B500} solid ${color.Primary.Main}`,
      outlineOffset: config.borderWidth.B300,
    },
  },
});

/** Inline video keeps its own controls; the viewer opens from this corner button. */
export const MediaVideoFrame = style({
  position: 'relative',
  alignSelf: 'flex-start',
  maxWidth: '100%',
  lineHeight: 0,
});

export const MediaExpandButton = style({
  position: 'absolute',
  top: config.space.S200,
  right: config.space.S200,
});

export const MediaViewerModal = style({
  width: 'min(72rem, 94vw)',
  maxHeight: '92vh',
  display: 'flex',
  flexDirection: 'column',
  backgroundColor: color.Surface.Container,
  color: color.Surface.OnContainer,
});

export const MediaViewerBody = style({
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  minHeight: toRem(160),
  padding: config.space.S300,
  overflow: 'auto',
});

export const MediaViewerMedia = style({
  display: 'block',
  maxWidth: '100%',
  maxHeight: 'calc(92vh - 5rem)',
  objectFit: 'contain',
});

/** Read by screen readers, not drawn. */
export const VisuallyHidden = style({
  position: 'absolute',
  width: 1,
  height: 1,
  padding: 0,
  margin: -1,
  overflow: 'hidden',
  clip: 'rect(0 0 0 0)',
  whiteSpace: 'nowrap',
  border: 0,
});

export const MessageActionDivider = style({
  margin: `${config.space.S100} ${config.space.S200}`,
  flexShrink: 0,
});

export const TimelineAvatar = style([avatarSurface]);

const messageHoverTint = `color-mix(in srgb, ${color.SurfaceVariant.ContainerHover} 35%, transparent)`;

export const MessageRow = recipe({
  base: {
    paddingTop: config.space.S200,
    paddingBottom: config.space.S200,
    paddingLeft: toRem(16),
    paddingRight: toRem(96),
    marginLeft: config.space.S200,
    marginRight: config.space.S200,
    boxSizing: 'border-box',
    color: 'var(--synara-message-foreground)',
  },
  variants: {
    surface: {
      true: {
        backgroundColor: 'transparent',
        border: `${config.borderWidth.B300} solid transparent`,
        boxShadow: 'none',
        borderRadius: config.radii.R400,
        // Reading comes first: hover is a steady, clearly visible tint (about
        // a third of the old surface change) with no border, lift or shadow.
        // It switches instantly: a fade on a faint tint reads as flicker as
        // the pointer crosses rows.
        selectors: {
          [`${MessageActionSurface}:hover &, ${MessageActionSurface}:focus-within &`]: {
            backgroundColor: messageHoverTint,
          },
        },
        transition: 'none',
        '@media': {
          '(prefers-contrast: more)': {
            boxShadow: 'none',
            borderColor: 'var(--synara-depth-contrast-edge)',
            selectors: {
              [`${MessageActionSurface}:hover &, ${MessageActionSurface}:focus-within &`]: {
                boxShadow: 'none',
                borderColor: 'var(--synara-depth-contrast-strong-edge)',
              },
            },
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

/** A message that mentions the signed-in user: an accent edge and a light tint. */
export const MentionsMeRow = style({
  boxShadow: `inset ${toRem(3)} 0 0 ${color.Primary.Main}`,
  backgroundColor: `color-mix(in srgb, ${color.Primary.Container} 45%, transparent)`,
});

export const UnsentDelivery = style({
  display: 'flex',
  flexWrap: 'wrap',
  alignItems: 'center',
  gap: config.space.S200,
  marginTop: config.space.S100,
  color: 'var(--synara-message-foreground)',
});

/** Authenticity shield. Grey is informational; red flags a trust problem. */
export const EncryptionShield = style({
  display: 'inline-flex',
  alignItems: 'center',
  float: 'right',
  marginLeft: config.space.S100,
  color: color.Secondary.Main,
  opacity: 0.75,
  selectors: {
    '&[data-native-timeline-shield="red"]': {
      color: color.Critical.Main,
      opacity: 1,
    },
  },
});

export const UnsentAction = style({
  border: `${config.borderWidth.B300} solid ${color.SurfaceVariant.ContainerLine}`,
  borderRadius: config.radii.R300,
  background: color.Surface.Container,
  color: 'var(--synara-message-foreground)',
  padding: `${config.space.S100} ${config.space.S200}`,
  cursor: 'pointer',
});

export const MessageBody = style({
  background: 'transparent',
  padding: 0,
  width: '100%',
  maxWidth: '100%',
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
  border: `1px solid var(--synara-rich-text-spoiler-border)`,
  borderRadius: config.radii.R300,
  background: 'var(--synara-rich-text-spoiler-background)',
  // Primary, not secondary: secondary was 3.8:1 on the spoiler fill.
  color: 'var(--synara-content-primary)',
  font: 'inherit',
  lineHeight: 'inherit',
  cursor: 'pointer',
  selectors: {
    '&:hover': {
      background: 'var(--synara-rich-text-spoiler-hover)',
    },
    '&:focus-visible': {
      outline: `2px solid ${color.Primary.Main}`,
      outlineOffset: toRem(2),
    },
  },
  '@media': {
    '(prefers-contrast: more)': {
      borderColor: 'var(--synara-rich-text-contrast-border)',
    },
  },
});

export const SpoilerContent = style({
  padding: `0 ${toRem(4)}`,
  borderRadius: config.radii.R300,
  background: 'var(--synara-rich-text-spoiler-background)',
  border: `1px solid var(--synara-rich-text-spoiler-border)`,
  '@media': {
    '(prefers-contrast: more)': {
      borderColor: 'var(--synara-rich-text-contrast-border)',
    },
  },
});

/** Custom emoji sit on the text line, capped to about one line high. */
export const CustomEmoji = style({
  display: 'inline-block',
  height: '1.5em',
  width: 'auto',
  maxWidth: '4.5em',
  verticalAlign: 'middle',
  objectFit: 'contain',
});

/** The address of a link Synara cannot open (ftp, magnet), shown for copying. */
export const LinkAddress = style({
  color: 'var(--synara-content-secondary)',
  wordBreak: 'break-all',
});

/** Where a link really goes when its text shows a different address. */
export const LinkHostWarning = style({
  // Critical text passes AA on both chat backgrounds; Warning does not in light.
  color: color.Critical.Main,
  fontWeight: config.fontWeight.W500,
  whiteSpace: 'nowrap',
});

export const InlineImageFallback = style({
  display: 'inline-block',
  padding: `0 ${toRem(4)}`,
  borderRadius: config.radii.R300,
  background: 'var(--synara-rich-text-inline-code-background)',
  border: `1px solid var(--synara-rich-text-inline-code-border)`,
  color: 'var(--synara-content-secondary)',
  fontStyle: 'italic',
  '@media': {
    '(prefers-contrast: more)': {
      borderColor: 'var(--synara-rich-text-contrast-border)',
    },
  },
});

export const SenderName = style({
  color: 'var(--synara-content-heading)',
  fontWeight: 600,
});

export const Metadata = style({
  color: 'var(--synara-content-secondary)',
});

export const FileDownload = style({
  appearance: 'none',
  background: 'none',
  border: 0,
  padding: 0,
  color: color.Primary.Main,
  cursor: 'pointer',
  textAlign: 'left',
  textDecoration: 'underline',
  font: 'inherit',
  selectors: {
    '&:disabled': {
      cursor: 'progress',
      textDecoration: 'none',
    },
    '&:focus-visible': {
      outline: `2px solid ${color.Primary.Main}`,
      outlineOffset: toRem(2),
    },
  },
});

export const FilePreviewModal = style({
  width: 'min(52rem, 92vw)',
  maxHeight: '88vh',
  display: 'flex',
  flexDirection: 'column',
  backgroundColor: color.Surface.Container,
  color: color.Surface.OnContainer,
});

export const FilePreviewHeader = style({
  paddingLeft: config.space.S200,
  paddingRight: config.space.S200,
  borderBottomWidth: config.borderWidth.B300,
  flexShrink: 0,
  gap: config.space.S200,
});

export const FilePreviewBody = style({
  padding: `${config.space.S500} ${config.space.S600}`,
  minHeight: toRem(160),
  maxWidth: toRem(760),
  marginInline: 'auto',
  color: color.Surface.OnContainer,
  backgroundColor: color.Surface.Container,
});

globalStyle(`${FilePreviewBody} ${FormattedBody}`, {
  fontSize: toRem(16),
  lineHeight: 1.65,
});

/** Inline thread replies sit under their root, indented behind a thread rule. */
export const InlineThreadReplies = style({
  marginTop: config.space.S100,
  paddingLeft: config.space.S300,
  borderLeft: `2px solid var(--synara-content-separator)`,
});

export const InlineThreadReply = style({
  color: 'var(--synara-message-foreground)',
});

export const InlineThreadStatus = style({
  paddingLeft: config.space.S300,
});

export const ReplySurface = style({
  width: '100%',
  maxWidth: '100%',
  padding: `${config.space.S200} ${config.space.S300}`,
  margin: `${config.space.S100} 0`,
  border: 'none',
  borderLeft: `3px solid var(--synara-content-separator)`,
  borderRadius: `0 ${config.radii.R300} ${config.radii.R300} 0`,
  background: 'var(--synara-table-even)',
  boxShadow: `inset 0 1px 0 ${quietEdgeLight}, 0 1px 2px color-mix(in srgb, ${color.Other.Shadow} 18%, transparent)`,
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
  '@media': {
    '(prefers-reduced-transparency: reduce)': {
      boxShadow: `inset 0 1px 0 ${quietEdgeLight}`,
    },
    '(prefers-contrast: more)': {
      boxShadow: 'none',
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
  fontFamily:
    'ui-monospace, SFMono-Regular, Menlo, Consolas, "Liberation Mono", "DejaVu Sans Mono", "Noto Sans Mono", monospace',
  fontSize: '0.92em',
  lineHeight: 1.5,
} as const;

export const CodeLanguage = style({
  flex: 'none',
  padding: `${config.space.S100} ${config.space.S300}`,
  color: syntaxMeta,
  fontFamily:
    'ui-monospace, SFMono-Regular, Menlo, Consolas, "Liberation Mono", "DejaVu Sans Mono", "Noto Sans Mono", monospace',
  fontSize: '0.78em',
  letterSpacing: '0.02em',
  textTransform: 'lowercase',
  userSelect: 'none',
});

export const CodeToolbar = style({
  display: 'flex',
  flexDirection: 'row',
  alignItems: 'center',
  justifyContent: 'space-between',
  gap: config.space.S200,
  borderBottom: `1px solid var(--synara-rich-text-code-block-border)`,
});

export const CodeCopyButton = style({
  flex: 'none',
  margin: `0 ${config.space.S200} 0 0`,
  padding: `${config.space.S100} ${config.space.S200}`,
  border: 0,
  borderRadius: config.radii.R400,
  background: 'transparent',
  color: 'var(--synara-content-secondary)',
  fontSize: '0.78em',
  letterSpacing: '0.02em',
  cursor: 'pointer',
  userSelect: 'none',
  ':hover': {
    color: 'var(--synara-content-primary)',
  },
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
  borderRight: `1px solid var(--synara-rich-text-code-block-border)`,
  color: syntaxMeta,
  background: 'var(--synara-rich-text-code-block-background)',
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
globalStyle(`${FormattedBody} strong, ${FormattedBody} b`, {
  fontWeight: 600,
  '@media': {
    '(prefers-contrast: more)': {
      fontWeight: 700,
    },
  },
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
  background: 'var(--synara-rich-text-code-block-background)',
  color: 'var(--synara-rich-text-inline-code-foreground)',
  border: `1px solid var(--synara-rich-text-code-block-border)`,
  '@media': {
    '(prefers-contrast: more)': {
      color: 'var(--synara-rich-text-contrast-foreground)',
      borderColor: 'var(--synara-rich-text-contrast-border)',
    },
  },
});
globalStyle(`${FormattedBody} pre:last-child`, {
  marginBottom: 0,
});
globalStyle(`${FormattedBody} code`, {
  fontFamily:
    'ui-monospace, SFMono-Regular, Menlo, Consolas, "Liberation Mono", "DejaVu Sans Mono", "Noto Sans Mono", monospace',
  fontSize: '0.92em',
});
globalStyle(`${FormattedBody} :not(pre) > code`, {
  padding: `0 ${toRem(4)}`,
  borderRadius: config.radii.R300,
  color: 'var(--synara-rich-text-inline-code-foreground)',
  background: 'var(--synara-rich-text-inline-code-background)',
  border: `1px solid var(--synara-rich-text-inline-code-border)`,
  whiteSpace: 'break-spaces',
  '@media': {
    '(prefers-contrast: more)': {
      color: 'var(--synara-rich-text-contrast-foreground)',
      borderColor: 'var(--synara-rich-text-contrast-border)',
    },
  },
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
  background: 'var(--synara-rich-text-table-canvas)',
  boxShadow: 'inset -14px 0 16px -18px rgba(0, 0, 0, 0.75)',
  selectors: {
    '&:focus-visible': {
      outline: `2px solid ${color.Primary.Main}`,
      outlineOffset: toRem(2),
    },
  },
  '@media': {
    '(prefers-reduced-transparency: reduce)': {
      boxShadow: 'none',
    },
    '(prefers-contrast: more)': {
      boxShadow: 'none',
    },
  },
});

globalStyle(`${TableScroll} table`, {
  width: 'max-content',
  minWidth: '100%',
  borderCollapse: 'separate',
  borderSpacing: 0,
  margin: 0,
  background: 'var(--synara-rich-text-table-canvas)',
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
  background: 'var(--synara-rich-text-table-header)',
  color: 'var(--synara-content-heading)',
  fontWeight: 600,
});
globalStyle(`${TableScroll} tbody tr:nth-child(odd) td`, {
  background: 'var(--synara-rich-text-table-odd)',
});
globalStyle(`${TableScroll} tr:nth-child(odd) td`, {
  background: 'var(--synara-rich-text-table-odd)',
});
globalStyle(`${TableScroll} tbody tr:nth-child(even) td`, {
  background: 'var(--synara-rich-text-table-even)',
});
globalStyle(`${TableScroll} tr:nth-child(even) td`, {
  background: 'var(--synara-rich-text-table-even)',
});
globalStyle(`${TableScroll} tbody tr:hover td`, {
  background: 'var(--synara-rich-text-table-hover)',
});
globalStyle(`${TableScroll} tr:hover td`, {
  background: 'var(--synara-rich-text-table-hover)',
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

export const UnfurlCard = style({
  display: 'flex',
  alignItems: 'stretch',
  maxWidth: toRem(420),
  minWidth: 0,
  marginTop: config.space.S200,
  border: `1px solid ${color.Surface.ContainerLine}`,
  borderRadius: config.radii.R400,
  overflow: 'hidden',
  background: color.Surface.Container,
});

export const UnfurlLink = style({
  display: 'flex',
  alignItems: 'stretch',
  minWidth: 0,
  flex: 1,
  color: 'inherit',
  textDecoration: 'none',
});

// Links in plain bodies are underlined like formatted ones: colour alone is
// 1.75:1 against dark body text. Unfurl cards are whole-card links.
globalStyle(`${MessageBody} a:not(.${UnfurlLink})`, {
  textDecoration: 'underline',
  textUnderlineOffset: toRem(2),
});

export const UnfurlThumb = style({
  width: toRem(96),
  height: toRem(96),
  objectFit: 'cover',
  flexShrink: 0,
  background: color.Surface.ContainerHover,
});

/** A GIF link played in place of its text (see `NativeInlineGif`). */
export const InlineGif = style({
  display: 'block',
  maxWidth: `min(100%, ${toRem(320)})`,
  maxHeight: toRem(280),
  marginTop: config.space.S100,
  borderRadius: config.radii.R400,
  background: color.Surface.ContainerHover,
});

export const XPostCard = style({
  display: 'flex',
  flexDirection: 'column',
  gap: config.space.S200,
  maxWidth: toRem(460),
  minWidth: 0,
  marginTop: config.space.S200,
  padding: config.space.S300,
  border: `1px solid ${color.Surface.ContainerLine}`,
  borderRadius: config.radii.R400,
  background: color.Surface.Container,
});

export const XPostAvatar = style({
  width: toRem(32),
  height: toRem(32),
  borderRadius: '50%',
  flexShrink: 0,
  objectFit: 'cover',
});

export const XPostMedia = style({
  display: 'block',
  width: '100%',
  maxHeight: toRem(420),
  objectFit: 'contain',
  borderRadius: config.radii.R300,
  background: '#000',
});

export const XPostMediaGrid = style({
  display: 'grid',
  gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
  gap: config.space.S100,
});

export const XPostSource = style({
  alignSelf: 'flex-start',
  fontSize: toRem(12),
  color: 'inherit',
  opacity: 0.75,
});

const historyStatusOverlay = {
  position: 'absolute',
  top: 0,
  left: 0,
  right: 0,
  zIndex: 4,
  display: 'flex',
  justifyContent: 'center',
  padding: config.space.S200,
  pointerEvents: 'none',
} as const;

export const HistoryStatusOverlayBackward = style({
  ...historyStatusOverlay,
  top: 0,
  background: `linear-gradient(180deg, var(--synara-rich-text-reading-surface) 55%, transparent)`,
});

export const HistoryStatusOverlayForward = style({
  ...historyStatusOverlay,
  top: 'auto',
  bottom: 0,
  background: `linear-gradient(0deg, var(--synara-rich-text-reading-surface) 55%, transparent)`,
});

export const HistoryStatusHitTarget = style({
  pointerEvents: 'auto',
});

export const HistoryStatusCard = style({
  display: 'flex',
  alignItems: 'center',
  flexWrap: 'wrap',
  gap: config.space.S200,
  padding: `${config.space.S200} ${config.space.S300}`,
  borderRadius: config.radii.R400,
  maxWidth: '100%',
  pointerEvents: 'auto',
  backgroundColor: color.SurfaceVariant.Container,
  color: color.SurfaceVariant.OnContainer,
  border: `${config.borderWidth.B300} solid ${color.SurfaceVariant.ContainerLine}`,
  boxShadow: raisedShadow,
});

export const HistoryStatusCardError = style({
  backgroundColor: color.Critical.Container,
  border: `${config.borderWidth.B300} solid ${color.Critical.Main}`,
  color: color.Critical.OnContainer,
});

export const HistoryStatusDateChip = style({
  position: 'absolute',
  top: 0,
  left: 0,
  right: 0,
  zIndex: 4,
  display: 'flex',
  justifyContent: 'center',
  padding: config.space.S200,
  pointerEvents: 'none',
  contain: 'layout style paint',
});

export const HistoryStatusDateChipCard = style({
  display: 'flex',
  alignItems: 'center',
  padding: `${config.space.S100} ${config.space.S300}`,
  borderRadius: config.radii.R400,
  maxWidth: '100%',
  pointerEvents: 'none',
  backgroundColor: color.SurfaceVariant.Container,
  color: color.SurfaceVariant.OnContainer,
  border: `${config.borderWidth.B300} solid ${color.SurfaceVariant.ContainerLine}`,
  contain: 'layout style paint',
});
