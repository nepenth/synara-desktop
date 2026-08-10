/**
 * Shared lightweight parse helpers for Matrix domain DTOs (P1.4).
 */

export function isObject(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}

export function optString(o: Record<string, unknown>, key: string): string | undefined | null {
  if (!(key in o) || o[key] === undefined) return undefined;
  return typeof o[key] === 'string' ? (o[key] as string) : null;
}

export function reqString(o: Record<string, unknown>, key: string): string | null {
  const v = o[key];
  return typeof v === 'string' ? v : null;
}

export function reqNumber(o: Record<string, unknown>, key: string): number | null {
  const v = o[key];
  return typeof v === 'number' && Number.isFinite(v) ? v : null;
}

export function optNumber(o: Record<string, unknown>, key: string): number | undefined | null {
  if (!(key in o) || o[key] === undefined) return undefined;
  const v = o[key];
  return typeof v === 'number' && Number.isFinite(v) ? v : null;
}

export function reqBoolean(o: Record<string, unknown>, key: string): boolean | null {
  const v = o[key];
  return typeof v === 'boolean' ? v : null;
}

export function optBoolean(o: Record<string, unknown>, key: string): boolean | undefined | null {
  if (!(key in o) || o[key] === undefined) return undefined;
  const v = o[key];
  return typeof v === 'boolean' ? v : null;
}

/** Reject secret-looking / media-byte keys at the DTO boundary. */
export const FORBIDDEN_WIRE_FIELD_NAMES = [
  'accessToken',
  'access_token',
  'refreshToken',
  'refresh_token',
  'password',
  'recoveryKey',
  'recovery_key',
  'privateKey',
  'private_key',
  'mediaBytes',
  'media_bytes',
  'fileBytes',
  'file_bytes',
  'ciphertext',
  'sessionKey',
  'session_key',
] as const;

export function hasForbiddenWireFields(o: Record<string, unknown>): boolean {
  for (const key of FORBIDDEN_WIRE_FIELD_NAMES) {
    if (key in o) return true;
  }
  return false;
}

export function stringArray(value: unknown): string[] | null {
  if (!Array.isArray(value)) return null;
  const out: string[] = [];
  for (const item of value) {
    if (typeof item !== 'string') return null;
    out.push(item);
  }
  return out;
}
