import { useCallback, DragEventHandler, RefObject, useState, useEffect, useRef } from 'react';
import { getDataTransferFiles } from '../utils/dom';
import {
  readPlatformDroppedFiles,
  supportsPlatformNativeFileDrop,
  type PlatformNativeFileDropPayload,
} from '../platform';

export const useFileDropHandler = (onDrop: (file: File[]) => void): DragEventHandler =>
  useCallback(
    (evt) => {
      const files = getDataTransferFiles(evt.dataTransfer);
      if (files) onDrop(files);
    },
    [onDrop]
  );

export const useFileDropZone = (
  zoneRef: RefObject<HTMLElement | null>,
  onDrop: (file: File[]) => void
): boolean => {
  const dragStateRef = useRef<'start' | 'leave' | 'over' | undefined>(undefined);
  const [active, setActive] = useState(false);

  useEffect(() => {
    const target = zoneRef.current;
    const handleDrop = (evt: DragEvent) => {
      evt.preventDefault();
      dragStateRef.current = undefined;
      setActive(false);
      if (!evt.dataTransfer) return;
      const files = getDataTransferFiles(evt.dataTransfer);
      if (files) onDrop(files);
    };

    target?.addEventListener('drop', handleDrop);
    return () => {
      target?.removeEventListener('drop', handleDrop);
    };
  }, [zoneRef, onDrop]);

  useEffect(() => {
    const target = zoneRef.current;
    const handleDragEnter = (evt: DragEvent) => {
      if (evt.dataTransfer?.types.includes('Files')) {
        dragStateRef.current = 'start';
        setActive(true);
      }
    };
    const handleDragLeave = () => {
      if (dragStateRef.current !== 'over') return;
      dragStateRef.current = 'leave';
      setActive(false);
    };
    const handleDragOver = (evt: DragEvent) => {
      evt.preventDefault();
      dragStateRef.current = 'over';
    };

    target?.addEventListener('dragenter', handleDragEnter);
    target?.addEventListener('dragleave', handleDragLeave);
    target?.addEventListener('dragover', handleDragOver);
    return () => {
      target?.removeEventListener('dragenter', handleDragEnter);
      target?.removeEventListener('dragleave', handleDragLeave);
      target?.removeEventListener('dragover', handleDragOver);
    };
  }, [zoneRef]);

  useEffect(() => {
    if (!supportsPlatformNativeFileDrop()) return undefined;

    const handleNativeFileDrop = (evt: Event) => {
      const detail = (evt as CustomEvent<PlatformNativeFileDropPayload>).detail;
      if (!detail) return;

      if (detail.phase === 'enter' || detail.phase === 'over') {
        dragStateRef.current = 'over';
        setActive(true);
        return;
      }

      if (detail.phase === 'leave') {
        dragStateRef.current = undefined;
        setActive(false);
        return;
      }

      if (detail.phase === 'drop') {
        dragStateRef.current = undefined;
        setActive(false);
        if (detail.paths.length === 0) return;

        void readPlatformDroppedFiles(detail.paths)
          .then((files) => {
            if (files.length > 0) onDrop(files);
          })
          .catch(() => {
            setActive(false);
          });
      }
    };

    window.addEventListener('synara-native-file-drop', handleNativeFileDrop);
    return () => {
      window.removeEventListener('synara-native-file-drop', handleNativeFileDrop);
    };
  }, [onDrop]);

  return active;
};
