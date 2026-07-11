(() => {
  const invoke = (command, args) => {
    if (typeof window.__TAURI_INTERNALS__?.invoke !== "function") {
      throw new Error("Tauri IPC bridge is unavailable");
    }
    return window.__TAURI_INTERNALS__.invoke(command, args ?? {});
  };

  window.__SYNARA_DESKTOP__ = {
    platform: "tauri",
    supportsTray: true,
    supportsGlobalShortcuts: true,
    supportsIntegrationStatus: true,
    supportsTrayState: true,
    supportsUpdater: false,
    supportsMediaPermissions: true,
    supportsSecureSecretStore: false,
    supportsSpellcheck: true,
    desktopEnvironment: "unknown",
    sessionType: "unknown",
    routes: Object.freeze({
      later: "/inbox/later/",
      notifications: "/inbox/notifications/",
      settings: "/settings/",
    }),
    invoke,
  };
})();
