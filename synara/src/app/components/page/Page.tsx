import React, {
  ComponentProps,
  KeyboardEventHandler,
  MutableRefObject,
  PointerEventHandler,
  ReactNode,
  createContext,
  useContext,
  useRef,
  useState,
} from 'react';
import { Box, Header, Line, Scroll, Text, as } from 'folds';
import classNames from 'classnames';
import { ContainerColor } from '../../styles/ContainerColor.css';
import * as css from './style.css';
import { ScreenSize, useScreenSizeContext } from '../../hooks/useScreenSize';
import { useRoomListHidden } from '../../features/room-list-pane/useRoomListPane';
import { ROOM_LIST_PANE_ID } from '../../features/room-list-pane/roomListPane';

/** Set by a `PageRoot` whose pane can collapse: whether it is collapsed now. */
const PageNavCollapsedContext = createContext<boolean | undefined>(undefined);

/**
 * The same value for the page beside the pane, so its header can offer
 * "Show room list" while the list (and its bottom bar) is hidden.
 */
const PageContentNavCollapsedContext = createContext<boolean | undefined>(undefined);

/** Inside a pane: `undefined` unless it can collapse (desktop), else whether it is hidden. */
export const usePageNavCollapsed = (): boolean | undefined => useContext(PageNavCollapsedContext);

/** True on a page whose collapsible room list is hidden right now (desktop only). */
export const useRoomListHiddenBesidePage = (): boolean =>
  useContext(PageContentNavCollapsedContext) === true;

/**
 * The list column's width in pixels, or `undefined` when the column fills
 * the window (Mobile). Lets the column's header and bottom bar adapt.
 */
const PageNavWidthContext = createContext<number | undefined>(undefined);
export const usePageNavWidth = (): number | undefined => useContext(PageNavWidthContext);

type PageRootProps = {
  nav: ReactNode;
  children: ReactNode;
  /** The client's room list pane, which the show/hide button and ⌘⇧D hide on desktop. */
  collapsibleNav?: boolean;
};

export function PageRoot({ nav, children, collapsibleNav }: PageRootProps) {
  const screenSize = useScreenSizeContext();
  const roomListCollapsed = useRoomListHidden();
  const desktop = screenSize !== ScreenSize.Mobile;
  const collapsed = collapsibleNav && desktop ? roomListCollapsed : undefined;

  return (
    <Box grow="Yes" className={ContainerColor({ variant: 'Background' })}>
      <PageNavCollapsedContext.Provider value={collapsed}>{nav}</PageNavCollapsedContext.Provider>
      {desktop && (
        <Line
          className={css.PageRootLine}
          data-collapsed={collapsed || undefined}
          variant="Background"
          size="300"
          direction="Vertical"
        />
      )}
      <PageContentNavCollapsedContext.Provider value={collapsed}>
        {children}
      </PageContentNavCollapsedContext.Provider>
    </Box>
  );
}

type ClientDrawerLayoutProps = {
  children: ReactNode;
  /** Names the pane as a navigation landmark (e.g. "Rooms"). */
  label?: string;
  /** Pinned above the scrolling content: the pane title (`NavPaneHeader`). */
  header?: ReactNode;
  /** Pinned below the scrolling content: the client's bottom bar. */
  footer?: ReactNode;
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

export function PageNav({
  size,
  label,
  header,
  footer,
  children,
}: ClientDrawerLayoutProps & css.PageNavVariants) {
  const screenSize = useScreenSizeContext();
  const isMobile = screenSize === ScreenSize.Mobile;
  const defaultWidth = size === '300' ? 222 : 256;
  const [navWidth, setNavWidth] = useState(() => readPageNavWidth(defaultWidth));
  const navWidthRef = useRef(navWidth);
  const dragStartRef = useRef<{ pointerX: number; width: number } | undefined>(undefined);
  const [resizing, setResizing] = useState(false);
  const collapsed = useContext(PageNavCollapsedContext);
  const collapsible = !isMobile && collapsed !== undefined;

  const updateNavWidth = (width: number) => {
    const nextWidth = clampPageNavWidth(width);
    navWidthRef.current = nextWidth;
    setNavWidth(nextWidth);
    return nextWidth;
  };

  const handleResizeStart: PointerEventHandler<HTMLButtonElement> = (event) => {
    event.currentTarget.setPointerCapture(event.pointerId);
    dragStartRef.current = { pointerX: event.clientX, width: navWidthRef.current };
    setResizing(true);
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
    setResizing(false);
    writePageNavWidth(navWidthRef.current);
  };

  const handleResizeKeyDown: KeyboardEventHandler<HTMLButtonElement> = (event) => {
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
    event.preventDefault();
    const direction = event.key === 'ArrowLeft' ? -1 : 1;
    const nextWidth = updateNavWidth(navWidthRef.current + direction * 12);
    writePageNavWidth(nextWidth);
  };

  const divider = !isMobile && (
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
  );

  // Header and footer stay put; only the list between them scrolls.
  const frame = (
    <PageNavWidthContext.Provider value={isMobile ? undefined : navWidth}>
      {header}
      <Box grow="Yes" direction="Column" className={css.PageNavBody}>
        {children}
      </Box>
      {footer}
    </PageNavWidthContext.Provider>
  );

  if (collapsible) {
    // Stays mounted while hidden, so scroll position, search and sections survive.
    return (
      <Box
        as={label ? 'nav' : 'div'}
        id={ROOM_LIST_PANE_ID}
        aria-label={label}
        className={classNames(css.PageNav({ size, resizable: true }), css.PageNavCollapsible)}
        shrink="No"
        style={{ width: collapsed ? 0 : navWidth }}
        data-compact={navWidth < 232 ? 'true' : undefined}
        data-collapsed={collapsed || undefined}
        data-resizing={resizing || undefined}
        inert={collapsed}
        aria-hidden={collapsed || undefined}
      >
        <div className={css.PageNavClip}>
          <div className={css.PageNavSlide} style={{ width: navWidth }}>
            {frame}
          </div>
        </div>
        {divider}
      </Box>
    );
  }

  return (
    <Box
      as={label ? 'nav' : 'div'}
      aria-label={label}
      grow={isMobile ? 'Yes' : undefined}
      className={css.PageNav({ size, resizable: !isMobile })}
      shrink={isMobile ? 'Yes' : 'No'}
      style={isMobile ? undefined : { width: navWidth }}
      data-compact={!isMobile && navWidth < 232 ? 'true' : undefined}
    >
      <Box grow="Yes" direction="Column">
        {frame}
      </Box>
      {divider}
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
