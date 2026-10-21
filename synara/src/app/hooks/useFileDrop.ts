import { useCallback, DragEventHandler, useState, useEffect, useRef, type RefObject } from 'react';
import { dataTransferHasFiles, getDataTransferFiles } from '../utils/dom';
import {
  readPlatformDroppedFiles,
  supportsPlatformNativeFileDrop,
  type PlatformNativeFileDropPayload,
} from '../platform';
import type { DesktopDroppedFileRejection } from '../utils/desktop';
import { listen } from '../utils/desktop';

export const useFileDropHandler = (onDrop: (file: File[]) => void): DragEventHandler =>
  useCallback(
    (evt) => {
      const files = getDataTransferFiles(evt.dataTransfer);
      if (files) onDrop(files);
    },
    [onDrop]
  );

/** Marks the area (room column, thread pane) whose composer takes drops on it. */
export const FILE_DROP_REGION_ATTRIBUTE = 'data-file-drop-region';

export type FileDropZone = {
  /** The drop area of this composer, or null while unmounted. */
  region: () => Element | null;
  /** Takes drops that land outside every region (the room composer). */
  primary: boolean;
};

/**
 * The one composer a drop at `hit` belongs to: the zone whose region
 * contains it, else the primary zone, else the first. With a thread pane
 * open two composers are mounted; each event must reach exactly one.
 */
export const pickFileDropZone = <Z extends FileDropZone>(
  zones: readonly Z[],
  hit: Element | null
): Z | undefined => {
  const containing = hit ? zones.find((zone) => zone.region()?.contains(hit)) : undefined;
  return containing ?? zones.find((zone) => zone.primary) ?? zones[0];
};

/**
 * Whether a paste outside any editor belongs to this composer: the one whose
 * region holds the focus, and the primary one when focus is in no region
 * or in its own. Each mounted composer listens on the window.
 */
export const windowPasteTargetsComposer = (
  ownRegion: Element | null | undefined,
  focused: Element | null | undefined,
  primary: boolean
): boolean => {
  const focusedRegion = focused?.closest(`[${FILE_DROP_REGION_ATTRIBUTE}]`) ?? null;
  if (focusedRegion && ownRegion) return focusedRegion === ownRegion;
  return primary;
};

/** Mounted composers, in mount order; shared so every hook picks the same one. */
const fileDropZones: FileDropZone[] = [];
/** The zone of the drag in progress; kept while the pointer is over an overlay. */
let draggingOver: FileDropZone | undefined;

const hitElement = (x: number, y: number): Element | null =>
  typeof document === 'undefined' ? null : document.elementFromPoint(x, y);

const targetZoneAt = (hit: Element | null): FileDropZone | undefined => {
  // The drop dialog and other overlays live in the portal, outside every
  // region; the pointer being over one must not move the drop elsewhere.
  if (draggingOver && hit?.closest('#portalContainer')) return draggingOver;
  draggingOver = pickFileDropZone(fileDropZones, hit);
  return draggingOver;
};

export type FileDropZoneOptions = {
  /** Any element inside this composer's drop region (the composer root). */
  anchorRef: RefObject<HTMLElement | null>;
  primary?: boolean;
  /** Upload limit: larger dropped files are reported here, never read. */
  maxFileBytes?: number;
  onTooLarge?: (files: DesktopDroppedFileRejection[]) => void;
};

export const useFileDropZone = (
  onDrop: (file: File[]) => void,
  { anchorRef, primary = false, maxFileBytes, onTooLarge }: FileDropZoneOptions
): boolean => {
  const dragStateRef = useRef<'start' | 'leave' | 'over' | undefined>(undefined);
  const [active, setActive] = useState(false);
  const zoneRef = useRef<FileDropZone | undefined>(undefined);
  const limitRef = useRef(maxFileBytes);
  limitRef.current = maxFileBytes;
  const onTooLargeRef = useRef(onTooLarge);
  onTooLargeRef.current = onTooLarge;

  useEffect(() => {
    const zone: FileDropZone = {
      region: () => {
        const anchor = anchorRef.current;
        return anchor?.closest(`[${FILE_DROP_REGION_ATTRIBUTE}]`) ?? anchor ?? null;
      },
      primary,
    };
    zoneRef.current = zone;
    fileDropZones.push(zone);
    return () => {
      fileDropZones.splice(fileDropZones.indexOf(zone), 1);
      if (draggingOver === zone) draggingOver = undefined;
      zoneRef.current = undefined;
    };
  }, [anchorRef, primary]);

  const isTarget = useCallback(
    (hit: Element | null) => !!zoneRef.current && targetZoneAt(hit) === zoneRef.current,
    []
  );

  useEffect(() => {
    const handleDrop = (evt: DragEvent) => {
      if (!dataTransferHasFiles(evt.dataTransfer)) return;
      evt.preventDefault();
      dragStateRef.current = undefined;
      setActive(false);
      if (!isTarget(hitElement(evt.clientX, evt.clientY))) return;
      const files = getDataTransferFiles(evt.dataTransfer);
      if (files) onDrop(files);
    };

    window.addEventListener('drop', handleDrop);
    return () => {
      window.removeEventListener('drop', handleDrop);
    };
  }, [isTarget, onDrop]);

  useEffect(() => {
    const handleDragEnter = (evt: DragEvent) => {
      if (dataTransferHasFiles(evt.dataTransfer)) {
        dragStateRef.current = 'start';
        setActive(isTarget(hitElement(evt.clientX, evt.clientY)));
      }
    };
    const handleDragLeave = () => {
      if (dragStateRef.current !== 'over') return;
      dragStateRef.current = 'leave';
      setActive(false);
    };
    const handleDragOver = (evt: DragEvent) => {
      if (!dataTransferHasFiles(evt.dataTransfer)) return;
      evt.preventDefault();
      dragStateRef.current = 'over';
      setActive(isTarget(hitElement(evt.clientX, evt.clientY)));
    };

    window.addEventListener('dragenter', handleDragEnter);
    window.addEventListener('dragleave', handleDragLeave);
    window.addEventListener('dragover', handleDragOver);
    return () => {
      window.removeEventListener('dragenter', handleDragEnter);
      window.removeEventListener('dragleave', handleDragLeave);
      window.removeEventListener('dragover', handleDragOver);
    };
  }, [isTarget]);

  useEffect(() => {
    if (!supportsPlatformNativeFileDrop()) return undefined;

    // Rust sends each drag phase once, as a Tauri event, with the pointer in
    // CSS pixels; every mounted composer hears it and only the one under the
    // pointer acts.
    const handleNativeFileDrop = (detail: PlatformNativeFileDropPayload | undefined) => {
      if (!detail) return;

      if (detail.phase === 'enter' || detail.phase === 'over') {
        // A new drag starts with no remembered target.
        if (detail.phase === 'enter') draggingOver = undefined;
        dragStateRef.current = 'over';
        setActive(isTarget(hitElement(detail.x, detail.y)));
        return;
      }

      if (detail.phase === 'leave') {
        dragStateRef.current = undefined;
        draggingOver = undefined;
        setActive(false);
        return;
      }

      if (detail.phase === 'drop') {
        dragStateRef.current = undefined;
        setActive(false);
        // Every mounted composer runs this; the remembered target is kept
        // until the next drag so they all agree on the one that reads.
        if (!isTarget(hitElement(detail.x, detail.y))) return;
        if (detail.paths.length === 0) return;

        void readPlatformDroppedFiles(detail.paths, limitRef.current)
          .then(({ files, tooLarge }) => {
            if (tooLarge.length > 0) onTooLargeRef.current?.(tooLarge);
            if (files.length > 0) onDrop(files);
          })
          .catch(() => {
            setActive(false);
          });
      }
    };

    let unlisten: (() => void | Promise<void>) | undefined;
    let disposed = false;

    void listen<PlatformNativeFileDropPayload>('synara-native-file-drop', (evt) => {
      handleNativeFileDrop(evt.payload);
    })
      .then((unsubscribe) => {
        if (disposed) {
          void unsubscribe?.();
          return;
        }
        unlisten = unsubscribe;
      })
      .catch(() => undefined);

    return () => {
      disposed = true;
      void unlisten?.();
    };
  }, [isTarget, onDrop]);

  return active;
};
