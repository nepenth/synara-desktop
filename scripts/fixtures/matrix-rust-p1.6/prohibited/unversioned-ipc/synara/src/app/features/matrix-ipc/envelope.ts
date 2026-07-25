/**
 * PROHIBITED FIXTURE — P1.6 guardrail must reject this file.
 * Envelope type with sessionGeneration + sequence but NO protocolVersion.
 */

export type MatrixIpcEnvelope = {
  sessionGeneration: number;
  sequence: number;
  kind: string;
};
