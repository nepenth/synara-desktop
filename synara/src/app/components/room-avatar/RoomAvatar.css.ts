import { style } from '@vanilla-extract/css';
import { color } from 'folds';
import { avatarMedia } from '../../styles/Depth.css';

export const RoomAvatar = style([
  avatarMedia,
  {
    backgroundColor: color.Secondary.Container,
    color: color.Secondary.OnContainer,
    textTransform: 'capitalize',

    selectors: {
      '&[data-image-loaded="true"]': {
        backgroundColor: 'transparent',
      },
    },
  },
]);
