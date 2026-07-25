/**
 * Matrix IPC schema foundation (P1.3) — transport-neutral types only.
 *
 * Not wired into production session bootstrap. Product Matrix runtime remains
 * matrix-js-sdk until later cutover phases.
 */

export * from './version';
export * from './error';
export * from './stream';
export * from './envelope';
export * from './protocol';
