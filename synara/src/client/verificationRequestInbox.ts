import type { MatrixClient } from 'matrix-js-sdk';
import { CryptoEvent, type VerificationRequest } from 'matrix-js-sdk/lib/crypto-api';

export type VerificationRequestInbox = {
  getSnapshot: () => VerificationRequest[];
  dismiss: (request: VerificationRequest) => void;
  hydrate: (requests: VerificationRequest[]) => void;
  subscribe: (listener: () => void) => () => void;
};

const inboxes = new WeakMap<MatrixClient, VerificationRequestInbox>();

const requestsMatch = (left: VerificationRequest, right: VerificationRequest): boolean => {
  if (left === right) return true;
  return Boolean(left.transactionId && left.transactionId === right.transactionId);
};

export const mergeSelfVerificationRequests = (
  existing: VerificationRequest[],
  incoming: VerificationRequest[]
): VerificationRequest[] => {
  const merged = [...existing];
  incoming.forEach((request) => {
    if (!request.isSelfVerification) return;
    if (!merged.some((current) => requestsMatch(current, request))) {
      merged.push(request);
    }
  });
  return merged;
};

/**
 * Install this before startClient so to-device verification requests received
 * by the first sync cannot be lost while the application is still on its splash
 * screen. The inbox is client-scoped, ordered, and deduplicated by transaction.
 */
export const ensureVerificationRequestInbox = (mx: MatrixClient): VerificationRequestInbox => {
  const existingInbox = inboxes.get(mx);
  if (existingInbox) return existingInbox;

  let requests: VerificationRequest[] = [];
  const listeners = new Set<() => void>();
  const notify = () => listeners.forEach((listener) => listener());
  const hydrate = (incoming: VerificationRequest[]) => {
    const merged = mergeSelfVerificationRequests(requests, incoming);
    if (merged.length === requests.length) return;
    requests = merged;
    notify();
  };

  const inbox: VerificationRequestInbox = {
    getSnapshot: () => [...requests],
    dismiss: (request) => {
      const next = requests.filter((current) => !requestsMatch(current, request));
      if (next.length === requests.length) return;
      requests = next;
      notify();
    },
    hydrate,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };

  mx.on(CryptoEvent.VerificationRequestReceived, (request) => hydrate([request]));
  inboxes.set(mx, inbox);
  return inbox;
};
