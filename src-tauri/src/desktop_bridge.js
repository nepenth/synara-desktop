(() => {
  window.__CINNY_DESKTOP__ = Object.freeze({
    platform: "tauri",
    supportsTray: true,
    supportsGlobalShortcuts: true,
    supportsUpdater: true,
    supportsMediaPermissions: true,
    routes: Object.freeze({
      later: "/inbox/later/",
      notifications: "/inbox/notifications/",
    }),
  });
})();
