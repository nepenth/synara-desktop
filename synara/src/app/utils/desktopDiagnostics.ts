const MAX_DESKTOP_DIAGNOSTIC_ENTRIES = 50;

const desktopDiagnosticEntries: string[] = [];

const sanitizeDiagnosticDetail = (detail: string): string =>
  detail.replace(/access[_-]?token/gi, '[redacted]').slice(0, 240);

export const recordDesktopDiagnostic = (entry: string): void => {
  const normalized = sanitizeDiagnosticDetail(entry.trim());
  if (!normalized) return;
  desktopDiagnosticEntries.push(normalized);
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
