const MAX_DESKTOP_DIAGNOSTIC_ENTRIES = 50;

const desktopDiagnosticEntries: string[] = [];

export const sanitizeDiagnosticDetail = (detail: string): string =>
  detail
    .replace(
      /(access[_-]?token|refresh[_-]?token|authorization|password)(["']?\s*[:=]\s*)(?:bearer\s+[a-z0-9._~+/=-]+|"[^"]*"|'[^']*'|[^\s,;}]+)/gi,
      '$1$2[redacted]'
    )
    .replace(/\bbearer\s+[a-z0-9._~+/=-]+/gi, 'Bearer [redacted]')
    .slice(0, 240);

const appendDesktopLog = (entry: string): void => {
  if (typeof window === 'undefined') return;
  const invoke = window.__SYNARA_DESKTOP__?.invoke ?? window.__TAURI_INTERNALS__?.invoke;
  if (!invoke) return;

  void invoke('desktop_append_log', {
    source: 'frontend',
    message: entry,
  }).catch(() => undefined);
};

export const recordDesktopDiagnostic = (entry: string): void => {
  const normalized = sanitizeDiagnosticDetail(entry.trim());
  if (!normalized) return;
  desktopDiagnosticEntries.push(normalized);
  appendDesktopLog(normalized);
  if (desktopDiagnosticEntries.length > MAX_DESKTOP_DIAGNOSTIC_ENTRIES) {
    desktopDiagnosticEntries.shift();
  }
};

export const getDesktopDiagnosticEntries = (): readonly string[] => desktopDiagnosticEntries;

export const clearDesktopDiagnostics = (): void => {
  desktopDiagnosticEntries.length = 0;
};

export const formatDesktopDiagnosticsSection = (): string => {
  if (desktopDiagnosticEntries.length === 0) return '';
  return [
    'Recent desktop IPC events:',
    ...desktopDiagnosticEntries.map((entry) => `- ${entry}`),
  ].join('\n');
};
