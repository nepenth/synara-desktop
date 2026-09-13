import { style } from '@vanilla-extract/css';
import { color, config, toRem } from 'folds';
import { raisedShadow, quietInteractiveSurface } from '../../styles/Depth.css';

export const Content = style({
  width: '100%',
  maxWidth: toRem(1000),
  margin: '0 auto',
  padding: config.space.S500,
  display: 'flex',
  flexDirection: 'column',
  gap: config.space.S500,
  minWidth: 0,
  '@media': { '(max-width: 600px)': { padding: config.space.S300 } },
});
export const Toolbar = style({
  display: 'flex',
  alignItems: 'center',
  flexWrap: 'wrap',
  gap: config.space.S300,
});
export const Filters = style({ display: 'flex', flexWrap: 'wrap', gap: config.space.S200 });
export const Filter = style([
  quietInteractiveSurface,
  {
    border: `1px solid ${color.Surface.ContainerLine}`,
    borderRadius: config.radii.R400,
    padding: `${config.space.S200} ${config.space.S300}`,
    backgroundColor: color.Surface.Container,
    color: color.Surface.OnContainer,
    cursor: 'pointer',
    font: 'inherit',
    selectors: {
      '&[aria-pressed=true]': {
        backgroundColor: color.Primary.Container,
        borderColor: color.Primary.Main,
        color: color.Primary.OnContainer,
      },
    },
  },
]);
export const Search = style({
  minWidth: toRem(160),
  flex: 1,
  padding: config.space.S300,
  borderRadius: config.radii.R400,
  border: `1px solid ${color.Surface.ContainerLine}`,
  backgroundColor: color.Background.Container,
  color: color.Background.OnContainer,
  font: 'inherit',
});
export const Card = style({
  border: `1px solid ${color.Surface.ContainerLine}`,
  borderRadius: config.radii.R400,
  backgroundColor: color.Surface.Container,
  boxShadow: raisedShadow,
  overflow: 'hidden',
  '@media': {
    '(prefers-contrast: more)': { boxShadow: 'none', borderColor: color.Surface.OnContainer },
  },
});
export const CardHeader = style({
  display: 'flex',
  flexWrap: 'wrap',
  alignItems: 'center',
  justifyContent: 'space-between',
  gap: config.space.S300,
  padding: config.space.S400,
  borderBottom: `1px solid ${color.Surface.ContainerLine}`,
});
export const Empty = style({
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  gap: config.space.S300,
  textAlign: 'center',
  padding: `${config.space.S700} ${config.space.S400}`,
  border: `1px solid ${color.Surface.ContainerLine}`,
  borderRadius: config.radii.R400,
});
export const Notice = style({
  padding: config.space.S300,
  border: `1px solid ${color.Surface.ContainerLine}`,
  borderRadius: config.radii.R400,
  backgroundColor: color.SurfaceVariant.Container,
});
export const Status = style({
  color: color.Surface.OnContainer,
  fontVariantNumeric: 'tabular-nums',
  whiteSpace: 'nowrap',
});
