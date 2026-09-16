/**
 * Privacy-safe MatrixRTC transport snapshot parser and IPC.
 *
 * Core `discover_rtc_transports` is the product owner. Autodiscovery
 * well-known foci are typed separately and are not used here.
 */

import {
  invokeDesktopWithAvailability,
  isSynaraDesktop,
  type DesktopInvokeResult,
} from '../../utils/desktop';
import { getSessionBootstrapResult } from '../../state/sessionBootstrap';

export type NativeRtcTransportKind = 'livekit' | 'custom';
export type NativeRtcTransportsStatus = 'ready' | 'unsupported' | 'unavailable';

export type NativeRtcTransport = {
  kind: NativeRtcTransportKind;
  serviceUrl?: string;
};

export type NativeRtcTransportsSnapshot = {
  sessionGeneration: number;
  status: NativeRtcTransportsStatus;
  transports: NativeRtcTransport[];
};

export type NativeRtcTransportsInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

const MAX_RTC_TRANSPORTS = 8;
const MAX_SERVICE_URL_CHARS = 2048;
const SNAPSHOT_KEYS = ['sessionGeneration', 'status', 'transports'];
const TRANSPORT_KEYS = ['kind', 'serviceUrl'];

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

const onlyKnownKeys = (value: Record<string, unknown>, keys: string[]): boolean =>
  Object.keys(value).every((key) => keys.includes(key));

const isSafeGeneration = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;

const isStatus = (value: unknown): value is NativeRtcTransportsStatus =>
  value === 'ready' || value === 'unsupported' || value === 'unavailable';

const isKind = (value: unknown): value is NativeRtcTransportKind =>
  value === 'livekit' || value === 'custom';

const isSafeServiceUrl = (value: unknown): value is string => {
  if (typeof value !== 'string' || value.length === 0 || value.length > MAX_SERVICE_URL_CHARS) {
    return false;
  }
  try {
    const parsed = new URL(value);
    return (
      (parsed.protocol === 'https:' || parsed.protocol === 'http:') &&
      parsed.username === '' &&
      parsed.password === ''
    );
  } catch {
    return false;
  }
};

export function parseRtcTransport(value: unknown): NativeRtcTransport | null {
  if (!isRecord(value) || !onlyKnownKeys(value, TRANSPORT_KEYS) || !isKind(value.kind)) {
    return null;
  }
  if (value.serviceUrl === undefined) {
    return { kind: value.kind };
  }
  if (!isSafeServiceUrl(value.serviceUrl)) return null;
  return { kind: value.kind, serviceUrl: value.serviceUrl };
}

export function parseRtcTransportsSnapshot(value: unknown): NativeRtcTransportsSnapshot | null {
  if (!isRecord(value) || !onlyKnownKeys(value, SNAPSHOT_KEYS)) return null;
  if (!isSafeGeneration(value.sessionGeneration) || !isStatus(value.status)) return null;
  if (!Array.isArray(value.transports) || value.transports.length > MAX_RTC_TRANSPORTS) {
    return null;
  }
  const transports: NativeRtcTransport[] = [];
  for (const row of value.transports) {
    const parsed = parseRtcTransport(row);
    if (!parsed) return null;
    transports.push(parsed);
  }
  if (value.status !== 'ready' && transports.length > 0) return null;
  return {
    sessionGeneration: value.sessionGeneration,
    status: value.status,
    transports,
  };
}

export type NativeRtcTransportsDependencies = {
  desktopNativeSession: boolean;
  invoke: NativeRtcTransportsInvoke;
};

function resolveDependencies(
  deps: Partial<NativeRtcTransportsDependencies>
): NativeRtcTransportsDependencies {
  return {
    desktopNativeSession:
      deps.desktopNativeSession ??
      (isSynaraDesktop() && getSessionBootstrapResult().source === 'native'),
    invoke: deps.invoke ?? ((command, args) => invokeDesktopWithAvailability(command, args)),
  };
}

async function invokeSnapshot(
  command: 'matrix_rtc_transports_snapshot' | 'matrix_rtc_transports_refresh',
  deps: NativeRtcTransportsDependencies
): Promise<NativeRtcTransportsSnapshot | null> {
  if (!deps.desktopNativeSession) return null;
  const result = await deps.invoke(command);
  if (!result.available) return null;
  return parseRtcTransportsSnapshot(result.value);
}

export async function snapshotRtcTransportsNative(
  deps: Partial<NativeRtcTransportsDependencies> = {}
): Promise<NativeRtcTransportsSnapshot | null> {
  return invokeSnapshot('matrix_rtc_transports_snapshot', resolveDependencies(deps));
}

export async function refreshRtcTransportsNative(
  deps: Partial<NativeRtcTransportsDependencies> = {}
): Promise<NativeRtcTransportsSnapshot | null> {
  return invokeSnapshot('matrix_rtc_transports_refresh', resolveDependencies(deps));
}
