/**
 * Stable Synara Matrix IPC error categories (plan §6.4).
 *
 * Privacy: tokens, credentials, recovery keys, event plaintext, raw push
 * payloads, and decrypted media must never appear in error fields.
 */

export const MATRIX_IPC_ERROR_CATEGORIES = [
  'authentication_rejected',
  'user_deactivated',
  'interactive_auth_required',
  'forbidden',
  'rate_limited',
  'connectivity',
  'homeserver_unavailable',
  'unsupported_capability',
  'store_locked',
  'store_corrupt',
  'store_unavailable',
  'crypto_failure',
  'recovery_failure',
  'verification_failure',
  'media_too_large',
  'media_unsupported',
  'media_decrypt_failed',
  'cancellation',
  'stale_session_generation',
  'sdk_invariant',
  'unknown',
] as const;

export type MatrixIpcErrorCategory = typeof MATRIX_IPC_ERROR_CATEGORIES[number];

/**
 * Privacy-safe error payload for `kind: "error"`.
 * Explicitly excludes secret-bearing fields.
 */
export type MatrixIpcError = {
  category: MatrixIpcErrorCategory;
  /** Optional short, privacy-safe summary. */
  message?: string;
  /** Opaque diagnostic code (never secrets or event bodies). */
  diagnosticId?: string;
  /** Suggested retry delay for rate limits / transient failures. */
  retryAfterMs?: number;
  /** Correlates to the request that failed. */
  requestId?: string;
};

const ERROR_CATEGORY_SET = new Set<string>(MATRIX_IPC_ERROR_CATEGORIES);

export function isMatrixIpcErrorCategory(value: unknown): value is MatrixIpcErrorCategory {
  return typeof value === 'string' && ERROR_CATEGORY_SET.has(value);
}

export function parseMatrixIpcError(value: unknown): MatrixIpcError | null {
  if (!value || typeof value !== 'object') return null;
  const o = value as Record<string, unknown>;
  if (!isMatrixIpcErrorCategory(o.category)) return null;
  if (o.message !== undefined && typeof o.message !== 'string') return null;
  if (o.diagnosticId !== undefined && typeof o.diagnosticId !== 'string') return null;
  if (o.retryAfterMs !== undefined && typeof o.retryAfterMs !== 'number') return null;
  if (o.requestId !== undefined && typeof o.requestId !== 'string') return null;
  // Reject secret-looking keys if present (privacy guard at boundary).
  for (const forbidden of [
    'accessToken',
    'access_token',
    'password',
    'recoveryKey',
    'recovery_key',
    'refreshToken',
    'refresh_token',
    'plaintext',
    'eventBody',
    'mediaBytes',
  ]) {
    if (forbidden in o) return null;
  }
  return {
    category: o.category,
    message: typeof o.message === 'string' ? o.message : undefined,
    diagnosticId: typeof o.diagnosticId === 'string' ? o.diagnosticId : undefined,
    retryAfterMs: typeof o.retryAfterMs === 'number' ? o.retryAfterMs : undefined,
    requestId: typeof o.requestId === 'string' ? o.requestId : undefined,
  };
}
