import React, { useId } from 'react';
import * as css from './styles.css';

// Geometry from assets/branding/synara-symbolic.svg (viewBox 0 0 256 256).
const ORBIT = 'M105 48A82 82 0 0 0 84 215';
const SPOKES = 'M51 91 101 125M47 153l55-14M80 207l30-45M95 60l16 42';
const NODES = [
  { cx: 95, cy: 60 },
  { cx: 51, cy: 91 },
  { cx: 47, cy: 153 },
  { cx: 80, cy: 207 },
];
const FEATHERS = [
  'M133 111C160 75 205 66 238 28c-2 48-28 84-95 111-8 3-15-19-10-28Z',
  'M141 143c31-30 67-33 91-59-4 42-32 69-84 83-9 2-15-17-7-24Z',
  'M139 174c29-22 56-19 77-36-8 34-30 56-70 65-10 2-16-20-7-29Z',
];
const TAIL = 'm115 164 30 66-10-61Z';

/**
 * The Synara mark drawn live: neon linework with a slow pulse through the
 * network nodes and a sheen across the wing. Motion stops under
 * prefers-reduced-motion.
 */
export function SynaraMark({ size, animated = true }: { size?: number; animated?: boolean }) {
  const id = useId().replace(/:/g, '');
  const line = `${id}-line`;
  const wing = `${id}-wing`;
  const sheen = `${id}-sheen`;
  const core = `${id}-core`;
  const glow = `${id}-glow`;
  const feathers = `${id}-feathers`;

  return (
    <svg
      className={animated ? css.MarkAnimated : undefined}
      width={size ?? '100%'}
      height={size ?? '100%'}
      // Square box centered on the drawn geometry (x 5–240, y 27–232), so the
      // mark and its halo sit visually centered.
      viewBox="-3 4 250 250"
      role="img"
      aria-label="Synara"
    >
      <defs>
        <linearGradient id={line} x1="40" y1="40" x2="150" y2="230" gradientUnits="userSpaceOnUse">
          <stop offset="0" stopColor="#8BF6FF" />
          <stop offset="0.55" stopColor="#3FB8FF" />
          <stop offset="1" stopColor="#2F6BFF" />
        </linearGradient>
        <linearGradient id={wing} x1="240" y1="28" x2="130" y2="200" gradientUnits="userSpaceOnUse">
          <stop offset="0" stopColor="#2F7DFF" />
          <stop offset="0.6" stopColor="#123C9C" />
          <stop offset="1" stopColor="#0A1F5C" />
        </linearGradient>
        <linearGradient id={sheen} x1="0" y1="0" x2="1" y2="0">
          <stop offset="0" stopColor="#B8FBFF" stopOpacity="0" />
          <stop offset="0.5" stopColor="#B8FBFF" stopOpacity="0.55" />
          <stop offset="1" stopColor="#B8FBFF" stopOpacity="0" />
        </linearGradient>
        <radialGradient id={core} cx="0.4" cy="0.35" r="0.75">
          <stop offset="0" stopColor="#2B6DFF" />
          <stop offset="1" stopColor="#081A4F" />
        </radialGradient>
        <clipPath id={feathers}>
          {FEATHERS.map((d) => (
            <path key={d} d={d} />
          ))}
        </clipPath>
        <filter id={glow} x="-30%" y="-30%" width="160%" height="160%">
          <feGaussianBlur stdDeviation="5" result="blur" />
          <feMerge>
            <feMergeNode in="blur" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>

      <g filter={`url(#${glow})`}>
        <g fill={`url(#${wing})`} stroke={`url(#${line})`} strokeWidth="3" strokeLinejoin="round">
          {FEATHERS.map((d) => (
            <path key={d} d={d} />
          ))}
          <path d={TAIL} />
        </g>
        <g clipPath={`url(#${feathers})`}>
          <rect
            className={css.MarkSheen}
            x="96"
            y="0"
            width="70"
            height="256"
            fill={`url(#${sheen})`}
          />
        </g>

        <g fill="none" stroke={`url(#${line})`} strokeLinecap="round" strokeLinejoin="round">
          <path d={ORBIT} strokeWidth="11" strokeOpacity="0.85" />
          <path className={css.MarkOrbitTrace} d={ORBIT} strokeWidth="4" pathLength={100} />
          <path d={SPOKES} strokeWidth="6" />
          <circle cx="112" cy="132" r="32" strokeWidth="9" fill={`url(#${core})`} />
        </g>
        <circle className={css.MarkCore} cx="112" cy="132" r="17" fill="#3FB8FF" />

        {NODES.map((node, index) => (
          <circle
            key={`${node.cx}-${node.cy}`}
            className={css.MarkNode}
            style={{ animationDelay: `${index * 0.45}s` }}
            cx={node.cx}
            cy={node.cy}
            r="12"
            fill={`url(#${core})`}
            stroke={`url(#${line})`}
            strokeWidth="6"
          />
        ))}
      </g>
    </svg>
  );
}
