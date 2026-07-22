import {
  getDesktopIntegrationStatus,
  getDesktopPerformanceCapabilities,
  type DesktopIntegrationStatus,
  type DesktopPerformanceCapabilities,
} from '../utils/desktop';
import {
  formatDesktopDiagnosticsSection,
  getDesktopDiagnosticEntries,
} from '../utils/desktopDiagnostics';
import {
  clearClientDiagnosticTokens,
  getDesktopDiagnosticsConfig,
  recordClientDiagnostic,
  type ClientDiagnosticDomain,
  type ClientDiagnosticIdentifiers,
} from '../utils/clientDiagnostics';
import { clearFoundationDiagnosticTokens } from '../utils/foundationDiagnostics';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../utils/desktop';

export type PlatformPerformanceCapabilities = DesktopPerformanceCapabilities;
export type PlatformIntegrationStatus = DesktopIntegrationStatus;

export const getPlatformPerformanceCapabilities = getDesktopPerformanceCapabilities;
export const getPlatformIntegrationStatus = getDesktopIntegrationStatus;
export const getPlatformDesktopDiagnosticEntries = getDesktopDiagnosticEntries;
export const formatPlatformDesktopDiagnosticsSection = formatDesktopDiagnosticsSection;

export type PlatformDiagnosticsStatus = {
  available: boolean;
  entryCount: number;
  sizeBytes: number;
  oldestTimestampMs?: number;
  newestTimestampMs?: number;
};

const unavailableDiagnosticsStatus = (): PlatformDiagnosticsStatus => ({
  available: false,
  entryCount: 0,
  sizeBytes: 0,
});

const finiteNumber = (value: unknown): number | undefined =>
  typeof value === 'number' && Number.isFinite(value) ? value : undefined;

const normalizeDiagnosticsStatus = (value: unknown): PlatformDiagnosticsStatus => {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return unavailableDiagnosticsStatus();
  }
  const status = value as Record<string, unknown>;
  return {
    available: status.available !== false,
    entryCount: finiteNumber(status.entryCount) ?? 0,
    sizeBytes: finiteNumber(status.sizeBytes) ?? finiteNumber(status.totalBytes) ?? 0,
    oldestTimestampMs: finiteNumber(status.oldestTimestampMs),
    newestTimestampMs: finiteNumber(status.newestTimestampMs),
  };
};

export const getPlatformDiagnosticsStatus = async (): Promise<PlatformDiagnosticsStatus> => {
  if (!isSynaraDesktop()) return unavailableDiagnosticsStatus();
  try {
    const result = await invokeDesktopWithAvailability<unknown>('desktop_diagnostics_status');
    if (!result.available) return unavailableDiagnosticsStatus();
    return normalizeDiagnosticsStatus(result.value);
  } catch {
    return unavailableDiagnosticsStatus();
  }
};

export const readPlatformDiagnosticsReport = async (): Promise<string | undefined> => {
  if (!isSynaraDesktop()) return undefined;
  try {
    const result = await invokeDesktopWithAvailability<unknown>('desktop_read_diagnostics');
    if (!result.available) return undefined;
    const parsed =
      typeof result.value === 'string' ? (JSON.parse(result.value) as unknown) : result.value;
    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
      return JSON.stringify(
        {
          ...(parsed as Record<string, unknown>),
          captureConfiguration: getDesktopDiagnosticsConfig(),
          handlingPolicy: {
            storage: 'local-only',
            upload: 'manual-only',
            schema: 'strict-allowlist-v1',
            reviewBeforeSharing: true,
          },
        },
        null,
        2
      );
    }
  } catch {
    // Diagnostics are best effort and must never interfere with the client.
  }
  return undefined;
};

export const clearPlatformDiagnostics = async (): Promise<boolean> => {
  if (!isSynaraDesktop()) return false;
  try {
    const result = await invokeDesktopWithAvailability<boolean>('desktop_clear_diagnostics');
    if (!result.available || result.value !== true) return false;
    clearClientDiagnosticTokens();
    clearFoundationDiagnosticTokens();
    return true;
  } catch {
    return false;
  }
};

export const recordPlatformDiagnostic = (
  domain: ClientDiagnosticDomain,
  event: string,
  fields?: Record<string, unknown>,
  identifiers?: ClientDiagnosticIdentifiers
): void => recordClientDiagnostic(domain, event, fields, identifiers);
