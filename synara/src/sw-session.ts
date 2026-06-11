export type SessionPushCredentials = {
  baseUrl?: string;
  accessToken?: string;
};

export function pushSessionToSW(baseUrl?: string, accessToken?: string): void {
  if (typeof navigator === 'undefined') return;
  if (!('serviceWorker' in navigator)) return;
  if (!navigator.serviceWorker.controller) return;

  navigator.serviceWorker.controller.postMessage({
    type: 'setSession',
    accessToken,
    baseUrl,
  });
}

export function pushActiveSessionToSW(
  getSession: () => SessionPushCredentials | undefined
): void {
  const session = getSession();
  pushSessionToSW(session?.baseUrl, session?.accessToken);
}
