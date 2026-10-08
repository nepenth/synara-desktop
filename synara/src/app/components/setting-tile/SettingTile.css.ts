import { style } from '@vanilla-extract/css';
import { config } from 'folds';

const NARROW = 'screen and (max-width: 720px)';

/**
 * One settings row: copy on the left, control on the right edge, vertically
 * centred against the copy. Long descriptions wrap inside the copy column
 * instead of pushing the control onto its own line; only narrow windows stack
 * the control under the copy.
 */
export const SettingTile = style({
  width: '100%',
  minWidth: 0,
  flexWrap: 'nowrap',
  color: 'var(--synara-content-primary)',
  selectors: {
    // Rows in separate setting cards sit two card paddings plus the card gap
    // apart. Rows sharing one card (which stacks them with a 1rem gap) get the
    // difference, so every pair of settings keeps the same vertical rhythm.
    '& + &': {
      marginTop: `calc(${config.space.S300} * 2 + ${config.space.S100} - ${config.space.S400})`,
    },
  },
  '@media': {
    [NARROW]: {
      flexWrap: 'wrap',
    },
  },
});

export const SettingCopy = style({
  flex: '1 1 0',
  minWidth: 0,
});

export const SettingControl = style({
  marginLeft: 'auto',
  alignSelf: 'center',
  '@media': {
    [NARROW]: {
      width: '100%',
      marginLeft: 0,
      paddingTop: config.space.S100,
      justifyContent: 'flex-start',
    },
  },
});
