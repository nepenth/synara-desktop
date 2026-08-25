import React, {
  ComponentProps,
  KeyboardEventHandler,
  MutableRefObject,
  PointerEventHandler,
  ReactNode,
  useRef,
  useState,
} from 'react';
import { Box, Header, Line, Scroll, Text, as } from 'folds';
import classNames from 'classnames';
import { ContainerColor } from '../../styles/ContainerColor.css';
import * as css from './style.css';
import { ScreenSize, useScreenSizeContext } from '../../hooks/useScreenSize';

type PageRootProps = {
  nav: ReactNode;
  children: ReactNode;
};

export function PageRoot({ nav, children }: PageRootProps) {
  const screenSize = useScreenSizeContext();

  return (
    <Box grow="Yes" className={ContainerColor({ variant: 'Background' })}>
      {nav}
      {screenSize !== ScreenSize.Mobile && (
        <Line variant="Background" size="300" direction="Vertical" />
      )}
      {children}
    </Box>
  );
}

type ClientDrawerLayoutProps = {
  children: ReactNode;
};

const PAGE_NAV_WIDTH_KEY = 'synara.navigationPaneWidth';
const PAGE_NAV_MIN_WIDTH = 196;
const PAGE_NAV_MAX_WIDTH = 360;

const clampPageNavWidth = (width: number): number =>
  Math.min(PAGE_NAV_MAX_WIDTH, Math.max(PAGE_NAV_MIN_WIDTH, width));

const readPageNavWidth = (fallback: number): number => {
  try {
    const savedWidth = Number(globalThis.localStorage?.getItem(PAGE_NAV_WIDTH_KEY));
    return Number.isFinite(savedWidth) && savedWidth > 0 ? clampPageNavWidth(savedWidth) : fallback;
  } catch {
    return fallback;
  }
};

const writePageNavWidth = (width: number): void => {
  try {
    globalThis.localStorage?.setItem(PAGE_NAV_WIDTH_KEY, String(width));
  } catch {
    // Device-local layout preferences are best effort.
  }
};

export function PageNav({ size, children }: ClientDrawerLayoutProps & css.PageNavVariants) {
  const screenSize = useScreenSizeContext();
  const isMobile = screenSize === ScreenSize.Mobile;
  const defaultWidth = size === '300' ? 222 : 256;
  const [navWidth, setNavWidth] = useState(() => readPageNavWidth(defaultWidth));
  const navWidthRef = useRef(navWidth);
  const dragStartRef = useRef<{ pointerX: number; width: number } | undefined>(undefined);

  const updateNavWidth = (width: number) => {
    const nextWidth = clampPageNavWidth(width);
    navWidthRef.current = nextWidth;
    setNavWidth(nextWidth);
    return nextWidth;
  };

  const handleResizeStart: PointerEventHandler<HTMLButtonElement> = (event) => {
    event.currentTarget.setPointerCapture(event.pointerId);
    dragStartRef.current = { pointerX: event.clientX, width: navWidthRef.current };
  };

  const handleResizeMove: PointerEventHandler<HTMLButtonElement> = (event) => {
    const dragStart = dragStartRef.current;
    if (!dragStart) return;
    updateNavWidth(dragStart.width + event.clientX - dragStart.pointerX);
  };

  const handleResizeEnd: PointerEventHandler<HTMLButtonElement> = (event) => {
    if (!dragStartRef.current) return;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    dragStartRef.current = undefined;
    writePageNavWidth(navWidthRef.current);
  };

  const handleResizeKeyDown: KeyboardEventHandler<HTMLButtonElement> = (event) => {
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
    event.preventDefault();
    const direction = event.key === 'ArrowLeft' ? -1 : 1;
    const nextWidth = updateNavWidth(navWidthRef.current + direction * 12);
    writePageNavWidth(nextWidth);
  };

  return (
    <Box
      grow={isMobile ? 'Yes' : undefined}
      className={css.PageNav({ size, resizable: !isMobile })}
      shrink={isMobile ? 'Yes' : 'No'}
      style={isMobile ? undefined : { width: navWidth }}
      data-compact={!isMobile && navWidth < 232 ? 'true' : undefined}
    >
      <Box grow="Yes" direction="Column">
        {children}
      </Box>
      {!isMobile && (
        <button
          type="button"
          className={css.PageNavDivider}
          aria-label={`Resize navigation pane. ${Math.round(
            navWidth
          )} pixels. Use Left and Right Arrow keys.`}
          onPointerDown={handleResizeStart}
          onPointerMove={handleResizeMove}
          onPointerUp={handleResizeEnd}
          onPointerCancel={handleResizeEnd}
          onKeyDown={handleResizeKeyDown}
        />
      )}
    </Box>
  );
}

export const PageNavHeader = as<'header', css.PageNavHeaderVariants>(
  ({ className, outlined, ...props }, ref) => (
    <Header
      className={classNames(css.PageNavHeader({ outlined }), className)}
      variant="Surface"
      size="600"
      {...props}
      ref={ref}
    />
  )
);

export function PageNavContent({
  scrollRef,
  children,
}: {
  children: ReactNode;
  scrollRef?: MutableRefObject<HTMLDivElement | null>;
}) {
  return (
    <Box grow="Yes" direction="Column">
      <Scroll
        ref={scrollRef}
        variant="Surface"
        direction="Vertical"
        size="300"
        hideTrack
        visibility="Hover"
      >
        <div className={css.PageNavContent}>{children}</div>
      </Scroll>
    </Box>
  );
}

export const Page = as<'div'>(({ className, ...props }, ref) => (
  <Box
    grow="Yes"
    direction="Column"
    className={classNames(ContainerColor({ variant: 'SurfaceVariant' }), className)}
    {...props}
    ref={ref}
  />
));

export const PageHeader = as<'div', css.PageHeaderVariants>(
  ({ className, outlined, balance, ...props }, ref) => (
    <Header
      as="header"
      size="600"
      className={classNames(css.PageHeader({ balance, outlined }), className)}
      {...props}
      ref={ref}
    />
  )
);

export const PageContent = as<'div'>(({ className, ...props }, ref) => (
  <div className={classNames(css.PageContent, className)} {...props} ref={ref} />
));

export function PageHeroEmpty({ children }: { children: ReactNode }) {
  return (
    <Box
      className={classNames(ContainerColor({ variant: 'SurfaceVariant' }), css.PageHeroEmpty)}
      direction="Column"
      alignItems="Center"
      justifyContent="Center"
      gap="200"
    >
      {children}
    </Box>
  );
}

export const PageHeroSection = as<'div', ComponentProps<typeof Box>>(
  ({ className, ...props }, ref) => (
    <Box
      direction="Column"
      className={classNames(css.PageHeroSection, className)}
      {...props}
      ref={ref}
    />
  )
);

export function PageHero({
  icon,
  title,
  subTitle,
  children,
}: {
  icon: ReactNode;
  title: ReactNode;
  subTitle: ReactNode;
  children?: ReactNode;
}) {
  return (
    <Box direction="Column" gap="400">
      <Box direction="Column" alignItems="Center" gap="200">
        {icon}
      </Box>
      <Box as="h2" direction="Column" gap="200" alignItems="Center">
        <Text align="Center" size="H2">
          {title}
        </Text>
        <Text align="Center" priority="400">
          {subTitle}
        </Text>
      </Box>
      {children}
    </Box>
  );
}

export const PageContentCenter = as<'div'>(({ className, ...props }, ref) => (
  <div className={classNames(css.PageContentCenter, className)} {...props} ref={ref} />
));
