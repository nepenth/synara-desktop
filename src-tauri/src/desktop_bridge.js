(() => {
  const invoke = (command, args) =>
    window.__TAURI_INTERNALS__?.invoke?.(command, args);

  window.__SYNARA_DESKTOP__ = Object.freeze({
    platform: "tauri",
    supportsTray: true,
    supportsGlobalShortcuts: true,
    supportsUpdater: false,
    supportsMediaPermissions: true,
    routes: Object.freeze({
      later: "/inbox/later/",
      notifications: "/inbox/notifications/",
    }),
    invoke,
  });
})();
