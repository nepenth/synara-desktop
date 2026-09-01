import React, { createContext, useContext } from 'react';
import {
  resolveMatrixAuthoredColorStyle,
  useMatrixColorContext,
} from '../../utils/matrixAuthoredColor';

type MatrixColorPresentationContextValue = {
  backgrounds: string[];
  foreground: string;
};

const MatrixColorPresentationContext = createContext<
  MatrixColorPresentationContextValue | undefined
>(undefined);

export type MatrixColorSurfaceKind = 'inlineCode' | 'codeBlock' | 'spoiler' | 'table';

export function MatrixColorSurface({
  surface,
  children,
}: {
  surface: MatrixColorSurfaceKind;
  children: React.ReactNode;
}) {
  const inherited = useContext(MatrixColorPresentationContext);
  const root = useMatrixColorContext();
  const surfacesByKind: Record<MatrixColorSurfaceKind, string[]> = {
    inlineCode: root.inlineCodeSurfaces,
    codeBlock: root.codeBlockSurfaces,
    spoiler: root.spoilerSurfaces,
    table: root.tableSurfaces,
  };
  const backgrounds = surfacesByKind[surface];
  return (
    <MatrixColorPresentationContext.Provider
      value={{ backgrounds, foreground: inherited?.foreground ?? root.semanticForeground }}
    >
      {children}
    </MatrixColorPresentationContext.Provider>
  );
}

export function MatrixColorSpan({
  foreground,
  background,
  children,
}: {
  foreground?: string;
  background?: string;
  children: React.ReactNode;
}) {
  const inherited = useContext(MatrixColorPresentationContext);
  const root = useMatrixColorContext();
  const backgrounds = inherited?.backgrounds ?? root.readingSurfaces;
  const semanticForeground = inherited?.foreground ?? root.semanticForeground;
  const style = resolveMatrixAuthoredColorStyle(
    foreground,
    background,
    backgrounds,
    semanticForeground,
    root.minimumContrast
  );
  const effectiveContext = {
    backgrounds: style.backgroundColor ? [style.backgroundColor] : backgrounds,
    foreground: style.color ?? semanticForeground,
  };

  return (
    <MatrixColorPresentationContext.Provider value={effectiveContext}>
      <span data-mx-color={foreground} data-mx-bg-color={background} style={style}>
        {children}
      </span>
    </MatrixColorPresentationContext.Provider>
  );
}
