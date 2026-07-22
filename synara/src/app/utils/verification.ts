import {
  type ShowSasCallbacks,
  VerificationPhase,
  type VerificationRequest,
  type Verifier,
} from 'matrix-js-sdk/lib/crypto-api';

export const getInitialSasCallbacks = (verifier: Verifier): ShowSasCallbacks | undefined =>
  verifier.getShowSasCallbacks() ?? undefined;

export const shouldCancelActiveVerificationRequest = (phase: VerificationPhase): boolean =>
  phase !== VerificationPhase.Done && phase !== VerificationPhase.Cancelled;

/** Cancel before dismissing so a transport failure remains actionable in the UI. */
export const cancelVerificationRequestForExit = async (
  request: VerificationRequest
): Promise<void> => {
  if (shouldCancelActiveVerificationRequest(request.phase)) {
    await request.cancel();
  }
};

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
