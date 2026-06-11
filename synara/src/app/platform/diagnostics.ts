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

export type PlatformPerformanceCapabilities = DesktopPerformanceCapabilities;
export type PlatformIntegrationStatus = DesktopIntegrationStatus;

export const getPlatformPerformanceCapabilities = getDesktopPerformanceCapabilities;
export const getPlatformIntegrationStatus = getDesktopIntegrationStatus;
export const getPlatformDesktopDiagnosticEntries = getDesktopDiagnosticEntries;
export const formatPlatformDesktopDiagnosticsSection = formatDesktopDiagnosticsSection;
