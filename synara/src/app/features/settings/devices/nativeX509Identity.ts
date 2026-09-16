import { invokeDesktopWithAvailability } from '../../../utils/desktop';

export type NativeX509CaSummary = {
  fingerprint: string;
  label: string;
};

export type NativeX509IdentityStatus = {
  enabled: boolean;
  hasCa: boolean;
  cas: NativeX509CaSummary[];
  signerImported: boolean;
  verifierConfigured: boolean;
  reloadRequired: boolean;
  certificateVerifiedIdentities: string[];
};

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

const MAX_FINGERPRINT_CHARS = 80;
const MAX_LABEL_CHARS = 64;
const MAX_MXID_CHARS = 255;
const MAX_IDENTITIES = 64;
const MAX_CAS = 16;

const optionalBounded = (value: unknown, maxChars: number): string | undefined => {
  if (typeof value !== 'string') return undefined;
  const trimmed = value.trim();
  if (!trimmed || [...trimmed].length > maxChars) return undefined;
  return trimmed;
};

const parseCa = (value: unknown): NativeX509CaSummary | undefined => {
  if (!isRecord(value)) return undefined;
  const fingerprint = optionalBounded(value.fingerprint, MAX_FINGERPRINT_CHARS);
  const label = optionalBounded(value.label, MAX_LABEL_CHARS);
  if (!fingerprint || !label) return undefined;
  if (!/^sha256:[a-f0-9]+$/i.test(fingerprint)) return undefined;
  return { fingerprint, label };
};

export const parseNativeX509IdentityStatus = (
  value: unknown
): NativeX509IdentityStatus | undefined => {
  if (!isRecord(value)) return undefined;
  if (typeof value.enabled !== 'boolean') return undefined;
  if (typeof value.hasCa !== 'boolean') return undefined;
  if (typeof value.signerImported !== 'boolean') return undefined;
  if (typeof value.verifierConfigured !== 'boolean') return undefined;
  if (typeof value.reloadRequired !== 'boolean') return undefined;
  if (!Array.isArray(value.cas)) return undefined;
  const cas: NativeX509CaSummary[] = [];
  for (const entry of value.cas.slice(0, MAX_CAS)) {
    const parsed = parseCa(entry);
    if (!parsed) return undefined;
    cas.push(parsed);
  }
  const identities: string[] = [];
  const rawIdentities = Array.isArray(value.certificateVerifiedIdentities)
    ? value.certificateVerifiedIdentities
    : [];
  for (const entry of rawIdentities.slice(0, MAX_IDENTITIES)) {
    const mxid = optionalBounded(entry, MAX_MXID_CHARS);
    if (!mxid || !mxid.startsWith('@')) return undefined;
    identities.push(mxid);
  }
  return {
    enabled: value.enabled,
    hasCa: value.hasCa,
    cas,
    signerImported: value.signerImported,
    verifierConfigured: value.verifierConfigured,
    reloadRequired: value.reloadRequired,
    certificateVerifiedIdentities: identities,
  };
};

const invokeX509 = async (command: string, args?: Record<string, unknown>) => {
  const result = await invokeDesktopWithAvailability<unknown>(command, args);
  if (!result.available || result.value === undefined) {
    throw new Error('X.509 identity settings are unavailable.');
  }
  const parsed = parseNativeX509IdentityStatus(result.value);
  if (!parsed) {
    throw new Error('X.509 identity settings are unavailable.');
  }
  return parsed;
};

export const getNativeX509IdentityStatus = (): Promise<NativeX509IdentityStatus> =>
  invokeX509('matrix_x509_identity_status');

export const setNativeX509IdentityEnabled = (enabled: boolean): Promise<NativeX509IdentityStatus> =>
  invokeX509('matrix_x509_identity_set_enabled', { enabled });

export const importNativeX509Ca = (): Promise<NativeX509IdentityStatus> =>
  invokeX509('matrix_x509_identity_import_ca');

export const removeNativeX509Ca = (fingerprint: string): Promise<NativeX509IdentityStatus> =>
  invokeX509('matrix_x509_identity_remove_ca', { fingerprint });

export const importNativeX509Signer = (): Promise<NativeX509IdentityStatus> =>
  invokeX509('matrix_x509_identity_import_signer');

export const reloadNativeSession = (): void => {
  if (typeof window !== 'undefined') window.location.reload();
};
