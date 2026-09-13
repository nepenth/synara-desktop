/** Renderer-only decoded media lifetime. Native remains the byte/download owner. */
type Entry = {
  users: number;
  url?: string;
  bytes: number;
  promise: Promise<string>;
  expired: boolean;
  retainWhenIdle: boolean;
  timer?: ReturnType<typeof setTimeout>;
};

type CacheOptions = {
  maxIdleEntries?: number;
  maxIdleBytes?: number;
  idleMs?: number;
  createUrl?: (blob: Blob) => string;
  revokeUrl?: (url: string) => void;
};

export const createMediaObjectUrlCache = ({
  maxIdleEntries = 128,
  maxIdleBytes = 16 * 1024 * 1024,
  idleMs = 5 * 60 * 1000,
  createUrl = (blob) => URL.createObjectURL(blob),
  revokeUrl = (url) => URL.revokeObjectURL(url),
}: CacheOptions = {}) => {
  const entries = new Map<string, Entry>();
  const idle = new Map<string, Entry>();
  const listeners = new Map<string, Set<() => void>>();
  const notify = (key: string) => listeners.get(key)?.forEach((listener) => listener());

  const discard = (key: string, entry: Entry) => {
    if (entries.get(key) === entry) entries.delete(key);
    if (idle.get(key) === entry) idle.delete(key);
    entry.expired = true;
    clearTimeout(entry.timer);
    const previousUrl = entry.url;
    entry.url = undefined;
    if (previousUrl) {
      revokeUrl(previousUrl);
      notify(key);
    }
  };

  const retainIdle = (key: string, entry: Entry) => {
    if (entry.expired || entry.users > 0) return;
    if (!entry.retainWhenIdle) {
      discard(key, entry);
      return;
    }
    idle.delete(key);
    idle.set(key, entry);
    clearTimeout(entry.timer);
    entry.timer = setTimeout(() => discard(key, entry), idleMs);
    let idleBytes = Array.from(idle.values()).reduce((sum, item) => sum + item.bytes, 0);
    for (const [oldKey, oldEntry] of idle) {
      if (idle.size <= maxIdleEntries && idleBytes <= maxIdleBytes) break;
      idleBytes -= oldEntry.bytes;
      discard(oldKey, oldEntry);
    }
  };

  return {
    peek: (key: string): string | undefined => entries.get(key)?.url,
    subscribe(key: string, listener: () => void) {
      const subscribers = listeners.get(key) ?? new Set<() => void>();
      subscribers.add(listener);
      listeners.set(key, subscribers);
      return () => {
        subscribers.delete(listener);
        if (subscribers.size === 0) listeners.delete(key);
      };
    },
    acquire(key: string, load: () => Promise<Blob>, retainWhenIdle = true) {
      let entry = entries.get(key);
      if (!entry) {
        const created: Entry = {
          users: 0,
          bytes: 0,
          expired: false,
          retainWhenIdle,
          promise: Promise.resolve(''),
        };
        created.promise = Promise.resolve()
          .then(() => {
            if (created.expired) throw new Error('Media cache entry expired');
            return load();
          })
          .then((blob) => {
            // A logout/eviction while IPC was pending must never resurrect media.
            if (created.expired) throw new Error('Media cache entry expired');
            created.url = createUrl(blob);
            created.bytes = blob.size;
            notify(key);
            queueMicrotask(() => retainIdle(key, created));
            if (!created.url) throw new Error('Media cache entry expired');
            return created.url;
          })
          .catch((error: unknown) => {
            discard(key, created);
            throw error;
          });
        entries.set(key, created);
        entry = created;
      }
      const leased = entry;
      leased.users += 1;
      clearTimeout(leased.timer);
      idle.delete(key);
      let released = false;
      return {
        url: leased.url,
        promise: leased.promise,
        release: () => {
          if (released) return;
          released = true;
          leased.users -= 1;
          // All commit-phase cleanups/setups must finish before deciding a
          // previously painted URL is idle. A returning row can lease it here.
          queueMicrotask(() => retainIdle(key, leased));
        },
      };
    },
    clear() {
      for (const [key, entry] of entries) discard(key, entry);
    },
  };
};

export type MediaObjectUrlCache = ReturnType<typeof createMediaObjectUrlCache>;

// A facade identifies one native session. Reusing an MXC URI in another account
// never shares its blob URL. Explicit session clearing also revokes active URLs.
let clientCaches = new WeakMap<object, MediaObjectUrlCache>();
const caches = new Set<WeakRef<MediaObjectUrlCache>>();
const clientFinalizer = new FinalizationRegistry<{
  cache: MediaObjectUrlCache;
  reference: WeakRef<MediaObjectUrlCache>;
}>(({ cache, reference }) => {
  cache.clear();
  caches.delete(reference);
});
export const getClientMediaObjectUrlCache = (client: object): MediaObjectUrlCache => {
  let cache = clientCaches.get(client);
  if (!cache) {
    cache = createMediaObjectUrlCache();
    clientCaches.set(client, cache);
    const reference = new WeakRef(cache);
    caches.add(reference);
    clientFinalizer.register(client, { cache, reference }, cache);
  }
  return cache;
};

export const clearMediaObjectUrlCaches = () => {
  for (const reference of caches) {
    const cache = reference.deref();
    if (cache) {
      clientFinalizer.unregister(cache);
      cache.clear();
    }
  }
  caches.clear();
  clientCaches = new WeakMap();
};
