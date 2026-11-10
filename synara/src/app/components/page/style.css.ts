import { style } from '@vanilla-extract/css';
import { recipe, RecipeVariants } from '@vanilla-extract/recipes';
import { DefaultReset, color, config, toRem } from 'folds';

export const PageNav = recipe({
  base: {
    position: 'relative',
  },
  variants: {
    size: {
      '400': {
        width: toRem(256),
      },
      '300': {
        width: toRem(222),
      },
    },
    resizable: {
      true: {
        minWidth: toRem(196),
        maxWidth: toRem(360),
      },
      false: {},
    },
  },
  defaultVariants: {
    size: '400',
    resizable: false,
  },
});
export type PageNavVariants = RecipeVariants<typeof PageNav>;

/** Collapse timing: the pane's width and the slide share it. */
const NAV_COLLAPSE = '200ms cubic-bezier(0.2, 0, 0, 1)';

/** The list between the pane's pinned header and footer; it alone scrolls. */
export const PageNavBody = style({
  minHeight: 0,
  position: 'relative',
});

/**
 * The client navigation pane can collapse to the window's edge. Its width
 * animates to 0 while the content, pinned to the pane's right edge, slides
 * left out of view. Once collapsed the pane (and its divider) stops painting.
 */
export const PageNavCollapsible = style({
  transition: `width ${NAV_COLLAPSE}, visibility 0s`,
  selectors: {
    '&[data-collapsed]': {
      minWidth: 0,
      visibility: 'hidden',
      transition: `width ${NAV_COLLAPSE}, visibility 0s linear 200ms`,
    },
    // A drag follows the pointer, not an animation.
    '&[data-resizing]': {
      transition: 'none',
    },
  },
  '@media': {
    '(prefers-reduced-motion: reduce)': {
      transition: 'none',
      selectors: {
        '&[data-collapsed]': {
          transition: 'none',
        },
      },
    },
  },
});

/** Clips the sliding content; `clip` also keeps focus() from scrolling it sideways. */
export const PageNavClip = style({
  position: 'absolute',
  inset: 0,
  display: 'flex',
  justifyContent: 'flex-end',
  overflow: 'clip',
});

/** Keeps the expanded width while the pane narrows, so rows never re-wrap. */
export const PageNavSlide = style({
  display: 'flex',
  flexDirection: 'column',
  flexShrink: 0,
});

export const PageRootLine = style({
  selectors: {
    '&[data-collapsed]': {
      display: 'none',
    },
  },
});

export const PageNavDivider = style([
  DefaultReset,
  {
    position: 'absolute',
    zIndex: 3,
    top: 0,
    right: toRem(-5),
    bottom: 0,
    width: toRem(10),
    padding: 0,
    border: 0,
    background: 'transparent',
    cursor: 'col-resize',
    selectors: {
      '&::after': {
        content: '',
        position: 'absolute',
        top: 0,
        bottom: 0,
        left: '50%',
        width: toRem(1),
        background: color.Background.ContainerLine,
        transform: 'translateX(-50%)',
        transition: 'width 120ms ease, background-color 120ms ease',
      },
      '&:hover::after, &:focus-visible::after': {
        width: toRem(3),
        background: color.Primary.Main,
      },
      '&:focus-visible': {
        outline: `2px solid ${color.Primary.Main}`,
        outlineOffset: toRem(-2),
      },
    },
    '@media': {
      '(prefers-reduced-motion: reduce)': {
        selectors: {
          '&::after': {
            transition: 'none',
          },
        },
      },
    },
  },
]);

export const PageNavHeader = recipe({
  base: {
    padding: `0 ${config.space.S200} 0 ${config.space.S300}`,
    flexShrink: 0,
    selectors: {
      'button&': {
        cursor: 'pointer',
      },
      'button&[aria-pressed=true]': {
        backgroundColor: color.Surface.ContainerActive,
      },
      'button&:hover, button&:focus-visible': {
        backgroundColor: color.Surface.ContainerHover,
      },
      'button&:active': {
        backgroundColor: color.Surface.ContainerActive,
      },
    },
  },

  variants: {
    outlined: {
      true: {
        borderBottomWidth: 1,
      },
    },
  },
  defaultVariants: {
    outlined: true,
  },
});
export type PageNavHeaderVariants = RecipeVariants<typeof PageNavHeader>;

export const PageNavContent = style({
  minHeight: '100%',
  padding: config.space.S200,
  paddingRight: 0,
  paddingBottom: config.space.S700,
});

export const PageHeader = recipe({
  base: {
    paddingLeft: config.space.S400,
    paddingRight: config.space.S200,
  },
  variants: {
    balance: {
      true: {
        paddingLeft: config.space.S200,
      },
    },
    outlined: {
      true: {
        borderBottomWidth: config.borderWidth.B300,
      },
    },
  },
  defaultVariants: {
    outlined: true,
  },
});
export type PageHeaderVariants = RecipeVariants<typeof PageHeader>;

export const PageContent = style([
  DefaultReset,
  {
    paddingTop: config.space.S400,
    paddingLeft: config.space.S400,
    paddingRight: 0,
    paddingBottom: toRem(100),
  },
]);

export const PageHeroEmpty = style([
  DefaultReset,
  {
    padding: config.space.S400,
    borderRadius: config.radii.R400,
    minHeight: toRem(450),
  },
]);

export const PageHeroSection = style([
  DefaultReset,
  {
    padding: '40px 0',
    maxWidth: toRem(466),
    width: '100%',
    margin: 'auto',
  },
]);

export const PageContentCenter = style([
  DefaultReset,
  {
    maxWidth: toRem(964),
    width: '100%',
    margin: 'auto',
  },
]);
