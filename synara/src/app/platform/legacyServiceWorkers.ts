type LegacyWorker = Pick<ServiceWorker, 'scriptURL'>;
type LegacyRegistration = Pick<ServiceWorkerRegistration, 'scope' | 'unregister'> & {
  active: LegacyWorker | null;
  waiting: LegacyWorker | null;
  installing: LegacyWorker | null;
};

type WorkerNavigator = {
  serviceWorker?: { getRegistrations: () => Promise<readonly LegacyRegistration[]> };
};

/** Retire only the workers previously registered by this application's entrypoint. */
export const retireLegacyServiceWorkers = async (
  browser: WorkerNavigator,
  baseUrl: string,
  pageUrl: string
): Promise<void> => {
  try {
    if (!browser.serviceWorker) return;
    const base = new URL(baseUrl, pageUrl);
    const productionScript = new URL('sw.js', base).href;
    const devScript = new URL('/dev-sw.js?dev-sw', pageUrl).href;
    const devScope = new URL('/', pageUrl).href;
    const registrations = await browser.serviceWorker.getRegistrations();
    await Promise.allSettled(
      registrations.map(async (registration) => {
        const workers = [registration.active, registration.waiting, registration.installing].filter(
          (worker): worker is LegacyWorker => worker !== null
        );
        if (
          workers.length > 0 &&
          workers.every(
            (worker) =>
              (worker.scriptURL === productionScript && registration.scope === base.href) ||
              (worker.scriptURL === devScript && registration.scope === devScope)
          )
        ) {
          await registration.unregister();
        }
      })
    );
  } catch {
    // Browser worker APIs can be disabled. Retirement must never prevent native app startup.
  }
};
