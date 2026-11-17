import { keyframes } from '@vanilla-extract/css';
import { recipe } from '@vanilla-extract/recipes';
import { DefaultReset, toRem } from 'folds';
import { continuousMotion } from '../../utils/motion';

const TypingDotAnime = keyframes({
  to: {
    opacity: '0.4',
    transform: 'translateY(-15%)',
  },
});

// Safe rendering mode: the same up-and-down, as two jumps per 1.2 s cycle
// instead of a smooth glide (see utils/motion.ts). With the per-dot delays
// that is five paints a second for the row. No keyframe sets its own timing
// function, so steps(1) holds each pose for its whole interval.
const TypingDotStepAnime = keyframes({
  '0%': {
    opacity: '1',
    transform: 'translateY(15%)',
  },
  '50%': {
    opacity: '0.4',
    transform: 'translateY(-15%)',
  },
  '100%': {
    opacity: '1',
    transform: 'translateY(15%)',
  },
});

export const TypingDot = recipe({
  base: [
    DefaultReset,
    {
      display: 'inline-block',
      backgroundColor: 'currentColor',
      borderRadius: '50%',
      transform: 'translateY(15%)',
    },
  ],
  variants: {
    animated: {
      true: continuousMotion(
        {
          animation: `${TypingDotAnime} 0.6s infinite alternate`,
          '@media': {
            // Three still dots still read as "typing".
            '(prefers-reduced-motion: reduce)': {
              animation: 'none',
            },
          },
        },
        {
          animationName: TypingDotStepAnime,
          animationDuration: '1.2s',
          animationTimingFunction: 'steps(1)',
          animationDirection: 'normal',
        }
      ),
    },
    size: {
      '300': {
        width: toRem(4),
        height: toRem(4),
      },
      '400': {
        width: toRem(8),
        height: toRem(8),
      },
    },
    index: {
      '0': {
        animationDelay: '0s',
      },
      '1': {
        animationDelay: '0.2s',
      },
      '2': {
        animationDelay: '0.4s',
      },
    },
  },
  defaultVariants: {
    size: '400',
    animated: true,
  },
});
