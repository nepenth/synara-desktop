(() => {
  const invoke = (command, args) =>
    window.__TAURI_INTERNALS__?.invoke?.(command, args);

  window.__SYNARA_DESKTOP__ = Object.freeze({
    platform: "tauri",
    supportsTray: true,
    supportsGlobalShortcuts: true,
    supportsIntegrationStatus: true,
    supportsTrayState: true,
    supportsUpdater: false,
    supportsMediaPermissions: true,
    supportsSecureSecretStore: false,
    desktopEnvironment: "unknown",
    sessionType: "unknown",
    routes: Object.freeze({
      later: "/inbox/later/",
      notifications: "/inbox/notifications/",
      settings: "/settings/",
    }),
    invoke,
  });
})();
