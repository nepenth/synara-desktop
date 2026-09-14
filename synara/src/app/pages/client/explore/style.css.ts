import { style } from '@vanilla-extract/css';
import { color, config } from 'folds';
import { restingInnerEdge } from '../../../styles/Depth.css';
import { ContainerColor } from '../../../styles/ContainerColor.css';

export const RoomsInfoCard = style([
  ContainerColor({ variant: 'SurfaceVariant' }),
  {
    padding: `${config.space.S700} ${config.space.S300}`,
    borderRadius: config.radii.R400,
    border: `${config.borderWidth.B300} solid ${color.SurfaceVariant.ContainerLine}`,
    boxShadow: restingInnerEdge,
  },
]);

/**
 * Error card keeps Critical semantics through its border while resting flat
 * on the reading plane like the settings sequence cards — no saturated
 * filled red block.
 */
export const PublicRoomsError = style([
  ContainerColor({ variant: 'SurfaceVariant' }),
  {
    padding: config.space.S300,
    borderRadius: config.radii.R400,
    border: `${config.borderWidth.B300} solid ${color.Critical.Main}`,
    boxShadow: restingInnerEdge,
  },
]);
