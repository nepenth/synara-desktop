import { globalStyle, style } from '@vanilla-extract/css';
import { color, config, DefaultReset, toRem } from 'folds';
import * as editorCss from '../../components/editor/Editor.css';
import { quietEdgeLight } from '../../styles/Depth.css';

export const RoomComposer = style([
  DefaultReset,
  {
    width: '100%',
    minWidth: 0,
  },
]);

/** Keep every first-line affordance on one optical center without moving it as a draft grows. */
export const RoomComposerEditor = style({
  minWidth: 0,
});

globalStyle(`${RoomComposerEditor} .${editorCss.EditorOptions}`, {
  boxSizing: 'border-box',
  height: toRem(50),
  justifyContent: 'center',
  paddingBlock: 0,
  paddingInline: config.space.S200,
});

globalStyle(`${RoomComposerEditor} .${editorCss.EditorTextareaArea}`, {
  minHeight: toRem(50),
});

export const RoomComposerReply = style([
  DefaultReset,
  {
    minWidth: 0,
  },
]);

export const RoomComposerLeadingAction = style([
  DefaultReset,
  {
    display: 'flex',
    alignItems: 'center',
    minWidth: 0,
  },
]);

export const RoomComposerFloatingActions = style([
  DefaultReset,
  {
    display: 'flex',
    alignItems: 'center',
    gap: toRem(2),
    minWidth: 0,
  },
]);

export const RoomComposerToolbar = style([
  DefaultReset,
  {
    minWidth: 0,
  },
]);

export const ComposerError = style({
  margin: `0 ${config.space.S300} ${config.space.S200}`,
  padding: `${config.space.S200} ${config.space.S300}`,
  border: `1px solid ${color.Critical.Main}`,
  borderRadius: config.radii.R300,
  background: color.Critical.Container,
  color: color.Critical.OnContainer,
});

/** One shared surface with a faint edge and soft contact shadow on hover. */
export const ComposerAction = style({
  transition: 'background-color 140ms ease-out, color 140ms ease-out, box-shadow 140ms ease-out',
  selectors: {
    '&&': {
      backgroundColor: 'transparent',
      boxShadow: 'none',
    },
    '&&:not(:disabled):hover': {
      backgroundColor: 'color-mix(in srgb, currentColor 7%, transparent)',
      boxShadow: `inset 0 1px 0 ${quietEdgeLight}, 0 1px 2px color-mix(in srgb, var(--synara-depth-shadow-near) 45%, transparent)`,
    },
    '&&:not(:disabled)[aria-pressed=true], &&:not(:disabled)[aria-expanded=true], &&:not(:disabled):active':
      {
        backgroundColor: 'color-mix(in srgb, currentColor 12%, transparent)',
      },
    '&&:not(:disabled):active': {
      boxShadow: `inset 0 1px 0 ${quietEdgeLight}`,
    },
    '&&:focus-visible': {
      outline: `2px solid ${color.Primary.Main}`,
      outlineOffset: '-2px',
    },
  },
  '@media': {
    '(prefers-reduced-motion: reduce)': {
      transition: 'none',
    },
    '(prefers-contrast: more)': {
      selectors: {
        '&&:not(:disabled):hover, &&:not(:disabled):active': {
          boxShadow: 'none',
        },
        '&&[aria-pressed=true], &&[aria-expanded=true]': {
          outline: '1px solid currentColor',
          outlineOffset: '-2px',
        },
        '&&:focus-visible': {
          outlineWidth: '2px',
        },
      },
    },
  },
});
