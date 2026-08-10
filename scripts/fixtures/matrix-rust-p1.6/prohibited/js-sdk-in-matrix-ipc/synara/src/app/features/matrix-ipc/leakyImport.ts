/**
 * PROHIBITED FIXTURE — P1.6 guardrail must reject this file.
 * matrix-js-sdk import inside matrix-ipc hard-ban zone.
 */
import { MatrixClient } from 'matrix-js-sdk';

export function leak(client: MatrixClient): string {
  return client.getUserId() ?? '';
}
