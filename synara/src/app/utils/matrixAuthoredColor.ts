import chroma from 'chroma-js';
import { useSyncExternalStore } from 'react';

const MATRIX_HEX_COLOR = /^#[0-9a-f]{6}$/i;
export const MATRIX_AUTHORED_TEXT_MINIMUM_CONTRAST = 4.5;
export const MATRIX_AUTHORED_INCREASED_CONTRAST = 7;
export const matrixAuthoredMinimumContrast = (prefersIncreasedContrast: boolean): number =>
  prefersIncreasedContrast
    ? MATRIX_AUTHORED_INCREASED_CONTRAST
    : MATRIX_AUTHORED_TEXT_MINIMUM_CONTRAST;

const normalizeColor = (value?: string): string | undefined => {
  if (!value || !MATRIX_HEX_COLOR.test(value)) return undefined;
  return chroma(value).hex();
};

const minimallyClampForeground = (
  foreground: string,
  backgrounds: readonly string[],
  minimumContrast: number
): string | undefined => {
  const passes = (candidate: string) =>
    backgrounds.every((background) => chroma.contrast(candidate, background) >= minimumContrast);
  if (passes(foreground)) return foreground;

  const endpoints = ['#000000', '#ffffff'] as const;
  let best: { color: string; amount: number } | undefined;
  for (const endpoint of endpoints) {
    if (!passes(endpoint)) continue;
    let low = 0;
    let high = 1;
    for (let iteration = 0; iteration < 16; iteration += 1) {
      const amount = (low + high) / 2;
      const candidate = chroma.mix(foreground, endpoint, amount, 'rgb').hex();
      if (passes(candidate)) high = amount;
      else low = amount;
    }
    const color = chroma.mix(foreground, endpoint, high, 'rgb').hex();
    if (!best || high < best.amount) best = { color, amount: high };
  }
  return best?.color;
};

export type MatrixAuthoredColorStyle = {
  color?: string;
  backgroundColor?: string;
};

/**
 * Resolves Matrix-authored colors only after the effective reading surface is
 * known. Invalid colors are dropped; valid foregrounds are preserved when
 * readable and minimally moved toward black/white otherwise. A background
 * without an authored foreground is dropped if it would make the semantic
 * message foreground unreadable.
 */
export const resolveMatrixAuthoredColorStyle = (
  authoredForeground: string | undefined,
  authoredBackground: string | undefined,
  readingSurfaces: string | readonly string[],
  semanticForeground: string,
  minimumContrast = MATRIX_AUTHORED_TEXT_MINIMUM_CONTRAST
): MatrixAuthoredColorStyle => {
  const surfaces = (typeof readingSurfaces === 'string' ? [readingSurfaces] : readingSurfaces)
    .map(normalizeColor)
    .filter((surface): surface is string => surface !== undefined);
  const fallback = normalizeColor(semanticForeground);
  if (surfaces.length === 0 || !fallback) return {};

  const foreground = normalizeColor(authoredForeground);
  let background = normalizeColor(authoredBackground);
  if (background && !foreground && chroma.contrast(fallback, background) < minimumContrast) {
    background = undefined;
  }

  const effectiveBackgrounds = background ? [background] : surfaces;
  const safeForeground = foreground
    ? minimallyClampForeground(foreground, effectiveBackgrounds, minimumContrast)
    : undefined;
  if (background && !safeForeground && chroma.contrast(fallback, background) < minimumContrast) {
    background = undefined;
  }

  return {
    color: safeForeground,
    backgroundColor: background,
  };
};

export const readMatrixColorContext = (): {
  readingSurfaces: string[];
  semanticForeground: string;
  inlineCodeSurfaces: string[];
  codeBlockSurfaces: string[];
  spoilerSurfaces: string[];
  tableSurfaces: string[];
  minimumContrast: number;
} => {
  if (typeof document === 'undefined') {
    return {
      readingSurfaces: ['#ffffff'],
      semanticForeground: '#000000',
      inlineCodeSurfaces: ['#d9dde3'],
      codeBlockSurfaces: ['#eceef1'],
      spoilerSurfaces: ['#dde1e6', '#d1d6dc'],
      tableSurfaces: ['#f2f3f5', '#dfe3e8', '#ffffff', '#f0f2f4', '#e7eaee'],
      minimumContrast: MATRIX_AUTHORED_TEXT_MINIMUM_CONTRAST,
    };
  }
  const computed = getComputedStyle(document.body);
  const read = (name: string, fallback: string) =>
    computed.getPropertyValue(name).trim() || fallback;
  return {
    readingSurfaces: [
      read('--synara-rich-text-reading-surface', '#ffffff'),
      read('--synara-rich-text-reading-surface-hover', '#f8f9fa'),
    ],
    semanticForeground:
      computed.getPropertyValue('--synara-message-foreground').trim() || '#000000',
    inlineCodeSurfaces: [read('--synara-rich-text-inline-code-background', '#d9dde3')],
    codeBlockSurfaces: [read('--synara-rich-text-code-block-background', '#eceef1')],
    spoilerSurfaces: [
      read('--synara-rich-text-spoiler-background', '#dde1e6'),
      read('--synara-rich-text-spoiler-hover', '#d1d6dc'),
    ],
    tableSurfaces: [
      read('--synara-rich-text-table-canvas', '#f2f3f5'),
      read('--synara-rich-text-table-header', '#dfe3e8'),
      read('--synara-rich-text-table-odd', '#ffffff'),
      read('--synara-rich-text-table-even', '#f0f2f4'),
      read('--synara-rich-text-table-hover', '#e7eaee'),
    ],
    minimumContrast: matrixAuthoredMinimumContrast(ensureContrastMediaQuery()?.matches ?? false),
  };
};

let colorContextRevision = 0;
let colorContextObserver: MutationObserver | undefined;
const colorContextSubscribers = new Set<() => void>();
let cachedColorContext: ReturnType<typeof readMatrixColorContext> | undefined;
let cachedColorContextRevision = -1;
let contrastMediaQuery: MediaQueryList | undefined;

const ensureContrastMediaQuery = (): MediaQueryList | undefined => {
  if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return undefined;
  contrastMediaQuery ??= window.matchMedia('(prefers-contrast: more)');
  return contrastMediaQuery;
};

const refreshMatrixColorContext = () => {
  colorContextRevision += 1;
  cachedColorContext = readMatrixColorContext();
  cachedColorContextRevision = colorContextRevision;
  colorContextSubscribers.forEach((subscriber) => subscriber());
};

const getMatrixColorContextSnapshot = (): ReturnType<typeof readMatrixColorContext> => {
  // Do not retain the SSR fallback into hydration.
  if (typeof document === 'undefined') return readMatrixColorContext();
  if (!cachedColorContext || cachedColorContextRevision !== colorContextRevision) {
    cachedColorContext = readMatrixColorContext();
    cachedColorContextRevision = colorContextRevision;
  }
  return cachedColorContext;
};

const subscribeToMatrixColorContext = (listener: () => void) => {
  colorContextSubscribers.add(listener);
  if (!colorContextObserver && typeof MutationObserver !== 'undefined') {
    colorContextObserver = new MutationObserver(refreshMatrixColorContext);
    colorContextObserver.observe(document.body, {
      attributes: true,
      attributeFilter: ['class', 'style'],
    });
    colorContextObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class', 'style'],
    });
  }
  if (colorContextSubscribers.size === 1) {
    ensureContrastMediaQuery()?.addEventListener('change', refreshMatrixColorContext);
  }

  return () => {
    colorContextSubscribers.delete(listener);
    if (colorContextSubscribers.size === 0) {
      colorContextObserver?.disconnect();
      colorContextObserver = undefined;
      contrastMediaQuery?.removeEventListener('change', refreshMatrixColorContext);
      contrastMediaQuery = undefined;
      cachedColorContext = undefined;
      cachedColorContextRevision = -1;
    }
  };
};

export const useMatrixColorContext = () => {
  useSyncExternalStore(
    subscribeToMatrixColorContext,
    () => colorContextRevision,
    () => 0
  );
  return getMatrixColorContextSnapshot();
};
