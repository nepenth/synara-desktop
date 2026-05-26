import {
  getDesktopIntegrationStatus,
  getDesktopPerformanceCapabilities,
  type DesktopIntegrationStatus,
  type DesktopPerformanceCapabilities,
} from '../utils/desktop';

export type PlatformPerformanceCapabilities = DesktopPerformanceCapabilities;
export type PlatformIntegrationStatus = DesktopIntegrationStatus;

export const getPlatformPerformanceCapabilities = getDesktopPerformanceCapabilities;
export const getPlatformIntegrationStatus = getDesktopIntegrationStatus;
