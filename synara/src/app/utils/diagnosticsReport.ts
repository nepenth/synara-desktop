export const MAX_CLIPBOARD_REPORT_CHARS = 250_000;

export const compactDiagnosticsReport = (report: string): string => {
  if (report.length <= MAX_CLIPBOARD_REPORT_CHARS) return report;
  try {
    const parsed = JSON.parse(report) as Record<string, unknown>;
    if (!Array.isArray(parsed.entries)) throw new Error('missing diagnostic entries');

    let low = 0;
    let high = parsed.entries.length;
    let compact = '';
    while (low <= high) {
      const start = Math.floor((low + high) / 2);
      const candidate = JSON.stringify(
        {
          ...parsed,
          clipboardTruncated: start > 0,
          entries: parsed.entries.slice(start),
        },
        null,
        2
      );
      if (candidate.length <= MAX_CLIPBOARD_REPORT_CHARS) {
        compact = candidate;
        high = start - 1;
      } else {
        low = start + 1;
      }
    }
    if (compact) return compact;
  } catch {
    // Older report formats fall back to a bounded text tail.
  }

  const tail = report.slice(-MAX_CLIPBOARD_REPORT_CHARS);
  const firstLineEnd = tail.indexOf('\n');
  return firstLineEnd >= 0 ? tail.slice(firstLineEnd + 1) : tail;
};
