import { useCallback, DragEventHandler, useState, useEffect, useRef } from 'react';
import { getDataTransferFiles } from '../utils/dom';
import {
  readPlatformDroppedFiles,
  supportsPlatformNativeFileDrop,
  type PlatformNativeFileDropPayload,
} from '../platform';
import { listen } from '../utils/desktop';

export const useFileDropHandler = (onDrop: (file: File[]) => void): DragEventHandler =>
  useCallback(
    (evt) => {
      const files = getDataTransferFiles(evt.dataTransfer);
      if (files) onDrop(files);
    },
    [onDrop]
  );

export const useFileDropZone = (onDrop: (file: File[]) => void): boolean => {
  const dragStateRef = useRef<'start' | 'leave' | 'over' | undefined>(undefined);
  const [active, setActive] = useState(false);

  useEffect(() => {
    const handleDrop = (evt: DragEvent) => {
      if (!evt.dataTransfer?.types.includes('Files')) return;
      evt.preventDefault();
      dragStateRef.current = undefined;
      setActive(false);
      const files = getDataTransferFiles(evt.dataTransfer);
      if (files) onDrop(files);
    };

    window.addEventListener('drop', handleDrop);
    return () => {
      window.removeEventListener('drop', handleDrop);
    };
  }, [onDrop]);

  useEffect(() => {
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
      if (!evt.dataTransfer?.types.includes('Files')) return;
      evt.preventDefault();
      dragStateRef.current = 'over';
    };

    window.addEventListener('dragenter', handleDragEnter);
    window.addEventListener('dragleave', handleDragLeave);
    window.addEventListener('dragover', handleDragOver);
    return () => {
      window.removeEventListener('dragenter', handleDragEnter);
      window.removeEventListener('dragleave', handleDragLeave);
      window.removeEventListener('dragover', handleDragOver);
    };
  }, []);

  useEffect(() => {
    if (!supportsPlatformNativeFileDrop()) return undefined;

    const handleNativeFileDrop = (detail: PlatformNativeFileDropPayload | undefined) => {
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

    const handleNativeFileDropEvent = (evt: Event) => {
      handleNativeFileDrop((evt as CustomEvent<PlatformNativeFileDropPayload>).detail);
    };

    window.addEventListener('synara-native-file-drop', handleNativeFileDropEvent);
    return () => {
      disposed = true;
      void unlisten?.();
      window.removeEventListener('synara-native-file-drop', handleNativeFileDropEvent);
    };
  }, [onDrop]);

  return active;
};
