/**
 * Synara-owned Matrix domain DTOs (P1.4).
 *
 * Product-oriented projections for Matrix IPC snapshot/delta bodies.
 * Sibling of `matrix-ipc` (transport). Not a matrix-js-sdk / Ruma clone.
 *
 * Not wired into production session bootstrap. Product Matrix runtime remains
 * matrix-js-sdk until later cutover phases.
 */

export const MATRIX_DTO_MARKER = 'matrix-domain-dtos-p1.4';
export const FORBID_MEDIA_BYTES_OVER_JSON_IPC = true;

export * from './ids';
export * from './parseUtil';
export * from './session';
export * from './room';
export * from './member';
export * from './relation';
export * from './timeline';
export * from './receipt';
export * from './typing';
export * from './upload';
export * from './media';
export * from './security';
export * from './notification';
export * from './search';
export * from './space';
export * from './thread';
export * from './roomDirectory';
export * from './roomJoinRule';
