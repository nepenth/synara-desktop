import { VerificationPhase } from 'matrix-js-sdk/lib/crypto-api';

export const shouldCancelActiveVerificationRequest = (phase: VerificationPhase): boolean =>
  phase !== VerificationPhase.Done && phase !== VerificationPhase.Cancelled;

export const phaseFromVerifierCancellation = (
  phase: VerificationPhase,
  verifierCancelled: boolean
): VerificationPhase => {
  if (!verifierCancelled || phase === VerificationPhase.Done) {
    return phase;
  }

  return VerificationPhase.Cancelled;
};

export const verificationErrorMessage = (error: unknown): string => {
  if (error instanceof Error && error.message) {
    return error.message;
  }

  if (typeof error === 'string' && error) {
    return error;
  }

  return 'Verification failed.';
};
