import { style } from '@vanilla-extract/css';
import { config, toRem } from 'folds';

export const SettingTile = style({
  width: '100%',
  minWidth: 0,
  flexWrap: 'wrap',
  color: 'var(--synara-content-primary)',
});

export const SettingCopy = style({
  minWidth: toRem(220),
});

export const SettingControl = style({
  marginLeft: 'auto',
  '@media': {
    'screen and (max-width: 720px)': {
      width: '100%',
      marginLeft: 0,
      paddingTop: config.space.S100,
      justifyContent: 'flex-start',
    },
  },
});
