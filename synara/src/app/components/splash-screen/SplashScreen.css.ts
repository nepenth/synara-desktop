import { style } from '@vanilla-extract/css';
import { color, config } from 'folds';

export const SplashScreen = style({
  flex: '1 1 0',
  minHeight: 0,
  overflowY: 'auto',
  backgroundColor: color.Background.Container,
  color: color.Background.OnContainer,
});

export const SplashScreenFooter = style({
  padding: config.space.S400,
});
