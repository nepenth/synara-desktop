import React, { ReactNode, useCallback, useEffect, useRef, useState } from 'react';
import { Box, Icon, IconButton, Icons, Text, config } from 'folds';
import { NativeTimelinePresenter } from './NativeTimelinePresenter';
import { clampThreadPaneWidth } from '../../utils/threadDisplay';
import * as css from './ThreadSidePanel.css';

/**
 * A thread beside the live room timeline. The room presenter stays mounted
 * and scrollable; this pane owns the thread stream and, through `composer`,
 * a composer that sends into this root only.
 */
export function ThreadSidePanel({
  roomId,
  rootEventId,
  width,
  availableWidth,
  onWidthChange,
  onClose,
  composer,
}: {
  roomId: string;
  rootEventId: string;
  width: number;
  /** Room area width, for the 60% ceiling. */
  availableWidth?: number;
  /** Called once with the final width when a resize ends. */
  onWidthChange: (width: number) => void;
  onClose: () => void;
  composer?: ReactNode;
}) {
  const [dragWidth, setDragWidth] = useState<number>();
  const startRef = useRef<{ x: number; width: number } | undefined>(undefined);
  const currentWidth = clampThreadPaneWidth(dragWidth ?? width, availableWidth);

  const onPointerDown = useCallback(
    (event: React.PointerEvent<HTMLButtonElement>) => {
      if (event.button !== 0) return;
      event.preventDefault();
      event.currentTarget.setPointerCapture(event.pointerId);
      startRef.current = { x: event.clientX, width: currentWidth };
      setDragWidth(currentWidth);
    },
    [currentWidth]
  );
  const onPointerMove = useCallback(
    (event: React.PointerEvent<HTMLButtonElement>) => {
      const start = startRef.current;
      if (!start) return;
      // The handle is on the pane's left edge: dragging left widens it.
      setDragWidth(clampThreadPaneWidth(start.width + (start.x - event.clientX), availableWidth));
    },
    [availableWidth]
  );
  const endResize = useCallback(() => {
    if (!startRef.current) return;
    startRef.current = undefined;
    setDragWidth((value) => {
      if (value !== undefined) onWidthChange(clampThreadPaneWidth(value, availableWidth));
      return undefined;
    });
  }, [availableWidth, onWidthChange]);

  const onKeyboardResize = useCallback(
    (event: React.KeyboardEvent<HTMLButtonElement>) => {
      const step = event.shiftKey ? 64 : 16;
      if (event.key === 'ArrowLeft')
        onWidthChange(clampThreadPaneWidth(width + step, availableWidth));
      else if (event.key === 'ArrowRight')
        onWidthChange(clampThreadPaneWidth(width - step, availableWidth));
      else return;
      event.preventDefault();
    },
    [availableWidth, onWidthChange, width]
  );

  // Escape closes the pane unless a dialog or menu is open over the app.
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape' || event.defaultPrevented) return;
      const portal = document.getElementById('portalContainer');
      if (portal && portal.children.length > 0) return;
      onClose();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [onClose]);

  return (
    <Box
      className={css.Pane}
      direction="Column"
      shrink="No"
      style={{ width: currentWidth }}
      aria-label="Thread"
      role="complementary"
      data-thread-side-panel={rootEventId}
    >
      <button
        type="button"
        className={css.ResizeHandle}
        aria-label={`Resize thread panel, ${currentWidth} pixels wide`}
        title="Drag or use the arrow keys to resize"
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={endResize}
        onPointerCancel={endResize}
        onKeyDown={onKeyboardResize}
      />
      <Box className={css.Header} alignItems="Center" gap="200" shrink="No">
        <Box grow="Yes">
          <Text size="H5" truncate>
            Thread
          </Text>
        </Box>
        <IconButton
          size="300"
          radii="300"
          aria-label="Close thread"
          title="Close thread"
          onClick={onClose}
        >
          <Icon size="100" src={Icons.Cross} />
        </IconButton>
      </Box>
      <Box grow="Yes" direction="Column" style={{ minHeight: 0 }}>
        <NativeTimelinePresenter
          key={`${roomId}:${rootEventId}`}
          roomId={roomId}
          threadRootEventId={rootEventId}
          onCloseThreadRoute={onClose}
          publishThreadRoot={false}
        />
      </Box>
      {composer && (
        <Box shrink="No" direction="Column" style={{ padding: `0 ${config.space.S300}` }}>
          {composer}
        </Box>
      )}
    </Box>
  );
}
