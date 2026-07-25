/**
 * PROHIBITED FIXTURE — P1.6 guardrail must reject this file.
 * New production path importing matrix-js-sdk outside the allowlist.
 */
import { createClient } from 'matrix-js-sdk';

export function makeClient(baseUrl: string) {
  return createClient({ baseUrl });
}
