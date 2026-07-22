import React, { useEffect, useState } from 'react';
import { useAtomValue } from 'jotai';
import { desktopPlatformSettingsAtom } from '../../state/settings';
import {
  recordClientDiagnostic,
  refreshDesktopDiagnosticsConfig,
} from '../../utils/clientDiagnostics';
import { isLegacyPerformanceDebugEnabled } from '../../utils/performance';

type PerformanceSnapshot = {
  fps: number;
  longTasks: number;
  lastLongTaskMs: number;
  renderedTimelineRows: number;
  memoryMb?: number;
};

const readRenderedTimelineRows = (): number =>
  document.querySelectorAll('[data-timeline-row-index]').length;

const readMemoryMb = (): number | undefined => {
  const { memory } = performance as Performance & {
    memory?: { usedJSHeapSize?: number };
  };
  if (!memory?.usedJSHeapSize) return undefined;
  const { usedJSHeapSize } = memory;
  return Math.round(usedJSHeapSize / 1024 / 1024);
};

export function PerformanceDebugOverlay() {
  const platformSettings = useAtomValue(desktopPlatformSettingsAtom);
  const diagnosticsEnabled =
    platformSettings.desktopDiagnosticsEnabled && platformSettings.desktopDiagnosticsPerformance;
  const legacyEnabled = isLegacyPerformanceDebugEnabled();
  const enabled = diagnosticsEnabled || legacyEnabled;
  const showOverlay =
    legacyEnabled || (diagnosticsEnabled && platformSettings.desktopDiagnosticsOverlay);
  const [snapshot, setSnapshot] = useState<PerformanceSnapshot>({
    fps: 0,
    longTasks: 0,
    lastLongTaskMs: 0,
    renderedTimelineRows: 0,
  });

  useEffect(() => {
    refreshDesktopDiagnosticsConfig(platformSettings);
  }, [platformSettings]);

  useEffect(() => {
    if (!enabled) return undefined;

    let frameCount = 0;
    let lastFpsTs = performance.now();
    let animationFrame = 0;
    let longTasks = 0;
    let lastLongTaskMs = 0;
    let maxLongTaskMs = 0;
    let lastDiagnosticTs = 0;
    let observer: PerformanceObserver | undefined;

    const tick = (now: number) => {
      frameCount += 1;
      if (now - lastFpsTs >= 1000) {
        const fps = Math.round((frameCount * 1000) / (now - lastFpsTs));
        const nextSnapshot = {
          fps,
          longTasks,
          lastLongTaskMs,
          renderedTimelineRows: readRenderedTimelineRows(),
          memoryMb: readMemoryMb(),
        };
        if (showOverlay) setSnapshot(nextSnapshot);
        if (now - lastDiagnosticTs >= 5_000) {
          recordClientDiagnostic('performance', 'runtime.sample', {
            fps: nextSnapshot.fps,
            renderedRowCount: nextSnapshot.renderedTimelineRows,
            longTaskCount: nextSnapshot.longTasks,
            lastLongTaskMs: nextSnapshot.lastLongTaskMs,
            maxLongTaskMs,
            memoryMb: nextSnapshot.memoryMb,
            documentVisible: document.visibilityState === 'visible',
          });
          lastDiagnosticTs = now;
        }
        frameCount = 0;
        lastFpsTs = now;
      }
      animationFrame = window.requestAnimationFrame(tick);
    };

    if (typeof PerformanceObserver !== 'undefined') {
      try {
        observer = new PerformanceObserver((list) => {
          list.getEntries().forEach((entry) => {
            longTasks += 1;
            lastLongTaskMs = Math.round(entry.duration);
            maxLongTaskMs = Math.max(maxLongTaskMs, lastLongTaskMs);
          });
        });
        observer.observe({ entryTypes: ['longtask'] });
      } catch {
        observer = undefined;
      }
    }

    animationFrame = window.requestAnimationFrame(tick);
    return () => {
      window.cancelAnimationFrame(animationFrame);
      observer?.disconnect();
    };
  }, [enabled, showOverlay]);

  if (!showOverlay) return null;

  return (
    <div
      style={{
        position: 'fixed',
        right: 12,
        bottom: 12,
        zIndex: 10000,
        padding: '8px 10px',
        borderRadius: 8,
        background: 'rgba(0, 0, 0, 0.78)',
        color: '#fff',
        font: '12px/1.45 ui-monospace, SFMono-Regular, Menlo, monospace',
        pointerEvents: 'none',
        boxShadow: '0 8px 24px rgba(0, 0, 0, 0.25)',
      }}
    >
      <div>fps {snapshot.fps}</div>
      <div>rows {snapshot.renderedTimelineRows}</div>
      <div>
        long tasks {snapshot.longTasks}
        {snapshot.lastLongTaskMs ? ` (${snapshot.lastLongTaskMs}ms)` : ''}
      </div>
      {snapshot.memoryMb && <div>heap {snapshot.memoryMb}MB</div>}
    </div>
  );
}
