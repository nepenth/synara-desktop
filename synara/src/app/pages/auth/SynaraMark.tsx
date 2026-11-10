import React, { useId } from 'react';

// The one Synara mark: a heavy orbit, a ring core, two nodes and three
// separated feathers, flat (no glow), so the same drawing holds from a 16px
// row icon to the sign-in hero. Source art: assets/branding/synara-mark.svg;
// assets/branding/synara-symbolic.svg is the single-colour version. Keep the
// three in step.
const ORBIT = 'M112 44A88 88 0 0 0 78 214';
const NODES = [
  { cx: 40, cy: 100 },
  { cx: 42, cy: 170 },
];
const FEATHERS = [
  'M136 104C162 72 205 66 238 28c-2 44-26 76-88 100-8 3-18-14-14-24Z',
  'M146 148c30-27 64-32 86-64-4 38-30 62-78 76-9 2-14-6-8-12Z',
  'M146 186c26-20 50-20 70-48-8 32-28 50-64 60-9 2-12-6-6-12Z',
];

/**
 * The Synara mark. Without `size` it fills its container. Pass `decorative`
 * where a visible "Synara" or list name sits beside it, so screen readers hear
 * the name once.
 */
export function SynaraMark({ size, decorative = false }: { size?: number; decorative?: boolean }) {
  const id = useId().replace(/:/g, '');
  const line = `${id}-line`;
  const wing = `${id}-wing`;

  return (
    <svg
      width={size ?? '100%'}
      height={size ?? '100%'}
      // Square box centered on the drawn geometry (x 13–238, y 28–223).
      viewBox="-3 4 250 250"
      data-synara-mark=""
      style={{ display: 'block' }}
      {...(decorative ? { 'aria-hidden': true } : { role: 'img', 'aria-label': 'Synara' })}
    >
      <defs>
        <linearGradient id={line} x1="40" y1="40" x2="150" y2="230" gradientUnits="userSpaceOnUse">
          <stop offset="0" stopColor="#8BF6FF" />
          <stop offset="1" stopColor="#3FA0FF" />
        </linearGradient>
        <linearGradient id={wing} x1="240" y1="28" x2="130" y2="200" gradientUnits="userSpaceOnUse">
          <stop offset="0" stopColor="#6FB0FF" />
          <stop offset="1" stopColor="#2F6BFF" />
        </linearGradient>
      </defs>
      <g fill="none" stroke={`url(#${line})`} strokeLinecap="round" strokeWidth="18">
        <path d={ORBIT} />
        <circle cx="108" cy="132" r="26" />
      </g>
      <g fill={`url(#${line})`}>
        {NODES.map((node) => (
          <circle key={`${node.cx}-${node.cy}`} cx={node.cx} cy={node.cy} r="17" />
        ))}
      </g>
      <g fill={`url(#${wing})`}>
        {FEATHERS.map((d) => (
          <path key={d} d={d} />
        ))}
      </g>
    </svg>
  );
}
