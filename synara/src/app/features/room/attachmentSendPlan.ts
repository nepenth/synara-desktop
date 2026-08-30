export type AttachmentSendPlan = {
  body: string;
  remainingTransactionIds: string[];
  textRole: 'caption' | 'trailing' | 'none';
};

export function makeOrReuseAttachmentSendPlan(
  existing: AttachmentSendPlan | undefined,
  transactionIds: string[],
  body: string
): AttachmentSendPlan {
  const hasSameRemainingAttachments =
    existing !== undefined &&
    existing.remainingTransactionIds.length === transactionIds.length &&
    existing.remainingTransactionIds.every(
      (transactionId, index) => transactionId === transactionIds[index]
    );
  if (hasSameRemainingAttachments) {
    if (existing.textRole === 'none' && body.trim().length > 0 && transactionIds.length > 0) {
      return {
        ...existing,
        body,
        textRole: transactionIds.length === 1 ? 'caption' : 'trailing',
      };
    }
    return existing.body === body ? existing : { ...existing, body };
  }

  return {
    body,
    remainingTransactionIds: transactionIds,
    textRole: body === '' ? 'none' : transactionIds.length === 1 ? 'caption' : 'trailing',
  };
}

export function completeAttachmentSendStep(
  plan: AttachmentSendPlan,
  transactionId: string
): AttachmentSendPlan {
  return {
    ...plan,
    remainingTransactionIds: plan.remainingTransactionIds.filter(
      (candidate) => candidate !== transactionId
    ),
  };
}

export function hasTrailingAttachmentText(plan: AttachmentSendPlan): boolean {
  return plan.textRole === 'trailing' && plan.body.trim().length > 0;
}
