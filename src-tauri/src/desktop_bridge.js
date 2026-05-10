(() => {
  const invoke = (command, args) => window.__TAURI_INTERNALS__?.invoke?.(command, args);

  window.__CINNY_DESKTOP__ = Object.freeze({
    platform: "tauri",
    supportsTray: true,
    supportsGlobalShortcuts: true,
    supportsUpdater: true,
    supportsMediaPermissions: true,
    supportsHighRefreshRate: true,
    routes: Object.freeze({
      later: "/inbox/later/",
      notifications: "/inbox/notifications/",
    }),
    invoke,
  });
})();
