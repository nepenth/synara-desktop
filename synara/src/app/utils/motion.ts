/** The user asked the system to reduce motion (WCAG 2.3.3). */
export const prefersReducedMotion = (): boolean =>
  typeof window !== 'undefined' &&
  typeof window.matchMedia === 'function' &&
  window.matchMedia('(prefers-reduced-motion: reduce)').matches;

/** Smooth scrolling, unless reduced motion is on: then jump. */
export const preferredScrollBehavior = (): 'auto' | 'smooth' =>
  prefersReducedMotion() ? 'auto' : 'smooth';
