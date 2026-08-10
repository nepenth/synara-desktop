export const shouldRetrySyncOnResume = (state: string | null): boolean =>
  state === 'RECONNECTING' || state === 'ERROR';
