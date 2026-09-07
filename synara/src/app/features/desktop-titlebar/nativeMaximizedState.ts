type NativeWindowState = {
  isMaximized(): Promise<boolean>;
  onResized(handler: () => void): Promise<() => void>;
};

/** Read native state after resize events, including window-manager and Tauri drags. */
export function observeNativeMaximizedState(
  nativeWindow: NativeWindowState,
  onChange: (maximized: boolean) => void
) {
  let disposed = false;
  let revision = 0;
  let unlisten: (() => void) | undefined;
  const refresh = async () => {
    if (disposed) return;
    const requestedRevision = ++revision;
    try {
      const maximized = await nativeWindow.isMaximized();
      if (!disposed && requestedRevision === revision) onChange(maximized);
    } catch {
      // Retain the last confirmed state if the native window is unavailable.
    }
  };
  void nativeWindow
    .onResized(() => void refresh())
    .then(
      (stop) => {
        if (disposed) {
          stop();
          return;
        }
        unlisten = stop;
        // Register before reading, so initialization cannot miss a native resize.
        void refresh();
      },
      () => void refresh()
    );
  return {
    refresh,
    dispose: () => {
      disposed = true;
      unlisten?.();
      unlisten = undefined;
    },
  };
}
