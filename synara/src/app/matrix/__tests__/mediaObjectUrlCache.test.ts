import assert from 'node:assert/strict';
import test from 'node:test';
import {
  clearMediaObjectUrlCaches,
  createMediaObjectUrlCache,
  getClientMediaObjectUrlCache,
} from '../mediaObjectUrlCache';

const deferred = <T>() => {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
};
const fixture = (options: Parameters<typeof createMediaObjectUrlCache>[0] = {}) => {
  let counter = 0;
  const revoked: string[] = [];
  const cache = createMediaObjectUrlCache({
    ...options,
    createUrl: () => `blob:${++counter}`,
    revokeUrl: (url) => revoked.push(url),
  });
  return { cache, revoked };
};

test('simultaneous avatars share one request and warm remount has a synchronous src', async () => {
  const { cache, revoked } = fixture();
  try {
    const download = deferred<Blob>();
    let requests = 0;
    const load = () => {
      requests += 1;
      return download.promise;
    };
    const first = cache.acquire('avatar', load);
    const second = cache.acquire('avatar', load);
    assert.equal(first.promise, second.promise);
    download.resolve(new Blob(['image']));
    const url = await first.promise;
    first.release();
    first.release(); // Release is idempotent; another mounted avatar still owns it.
    assert.deepEqual(revoked, []);
    second.release();
    assert.equal(cache.peek('avatar'), url);
    const returned = cache.acquire('avatar', load);
    assert.equal(returned.url, url);
    assert.equal(await returned.promise, url);
    assert.equal(requests, 1);
    returned.release();
  } finally {
    cache.clear();
  }
  assert.deepEqual(revoked, ['blob:1']);
});

test('idle entry and byte bounds evict least recent media without revoking active images', async () => {
  const { cache, revoked } = fixture({ maxIdleEntries: 1, maxIdleBytes: 4 });
  try {
    const active = cache.acquire('active', async () => new Blob(['123456']));
    await active.promise;
    const old = cache.acquire('old', async () => new Blob(['12']));
    await old.promise;
    old.release();
    const recent = cache.acquire('recent', async () => new Blob(['12']));
    await recent.promise;
    recent.release();
    await Promise.resolve();
    assert.equal(cache.peek('old'), undefined);
    assert.equal(cache.peek('active'), 'blob:1');
    assert.deepEqual(revoked, ['blob:2']);
    active.release();
    await Promise.resolve();
    assert.equal(cache.peek('active'), undefined); // Too large to retain while idle.
    assert.ok(revoked.includes('blob:1'));
  } finally {
    cache.clear();
  }
});

test('idle URLs expire and pending downloads cannot resurrect cleared or evicted entries', async (t) => {
  t.mock.timers.enable({ apis: ['setTimeout'] });
  const { cache, revoked } = fixture({ idleMs: 100 });
  const loaded = cache.acquire('ready', async () => new Blob(['image']));
  await loaded.promise;
  loaded.release();
  await Promise.resolve();
  t.mock.timers.tick(100);
  assert.equal(cache.peek('ready'), undefined);
  assert.deepEqual(revoked, ['blob:1']);

  const download = deferred<Blob>();
  const pending = cache.acquire('pending', () => download.promise);
  await Promise.resolve(); // Native IPC is already in flight at logout.
  cache.clear();
  download.resolve(new Blob(['late']));
  await assert.rejects(pending.promise, /expired/);
  assert.equal(cache.peek('pending'), undefined);
  assert.deepEqual(revoked, ['blob:1']);
  pending.release();
});

test('a failed download retries and source keys never reuse another avatar', async () => {
  const { cache } = fixture();
  try {
    const failed = cache.acquire('old', async () => {
      throw new Error('offline');
    });
    await assert.rejects(failed.promise, /offline/);
    failed.release();
    assert.equal(cache.peek('old'), undefined);
    const retry = cache.acquire('old', async () => new Blob(['old']));
    await retry.promise;
    assert.equal(cache.peek('new'), undefined);
    retry.release();
  } finally {
    cache.clear();
  }
});

test('client caches isolate accounts and session clearing revokes and replaces them', async () => {
  const clientA = {};
  const clientB = {};
  const first = getClientMediaObjectUrlCache(clientA);
  const second = getClientMediaObjectUrlCache(clientB);
  assert.notEqual(first, second);
  const image = first.acquire('same-uri', async () => new Blob(['image']));
  await image.promise;
  assert.equal(second.peek('same-uri'), undefined);
  clearMediaObjectUrlCaches();
  assert.equal(first.peek('same-uri'), undefined);
  assert.notEqual(getClientMediaObjectUrlCache(clientA), first);
  image.release();
  clearMediaObjectUrlCaches();
});

test('session clearing before dispatch prevents queued media IPC from starting', async () => {
  const { cache } = fixture();
  let downloads = 0;
  const queued = cache.acquire('queued', async () => {
    downloads += 1;
    return new Blob(['image']);
  });
  cache.clear();
  await assert.rejects(queued.promise, /expired/);
  assert.equal(downloads, 0);
  queued.release();
});

test('opaque handle leases release on last unmount and resolve their native source again', async () => {
  const { cache, revoked } = fixture();
  try {
    let downloads = 0;
    const load = async () => {
      downloads += 1;
      return new Blob(['image']);
    };
    const first = cache.acquire('opaque', load, false);
    const second = cache.acquire('opaque', load, false);
    const oldUrl = await first.promise;
    first.release();
    assert.equal(cache.peek('opaque'), oldUrl);
    second.release();
    await Promise.resolve();
    assert.equal(cache.peek('opaque'), undefined);
    assert.deepEqual(revoked, [oldUrl]);
    const remounted = cache.acquire('opaque', load, false);
    assert.notEqual(await remounted.promise, oldUrl);
    assert.equal(downloads, 2);
    remounted.release();
  } finally {
    cache.clear();
  }
});

test('outgoing cleanup cannot evict an incoming warm lease during the same commit', async () => {
  const { cache, revoked } = fixture({ maxIdleEntries: 1 });
  try {
    let warmLoads = 0;
    const loadWarm = async () => {
      warmLoads += 1;
      return new Blob(['warm']);
    };
    const warm = cache.acquire('warm', loadWarm);
    const warmUrl = await warm.promise;
    warm.release();
    await Promise.resolve();
    const outgoing = cache.acquire('outgoing', async () => new Blob(['outgoing']));
    await outgoing.promise;
    // Render reads the incoming snapshot; commit first cleans up outgoing rows.
    assert.equal(cache.peek('warm'), warmUrl);
    outgoing.release();
    const incoming = cache.acquire('warm', loadWarm);
    const incomingUrl = await incoming.promise;
    assert.equal(incoming.url, warmUrl);
    assert.equal(incomingUrl, warmUrl);
    assert.equal(warmLoads, 1);
    assert.ok(!revoked.includes(warmUrl));
    incoming.release();
  } finally {
    cache.clear();
  }
});

test('subscribers observe downloads and invalidation, and unsubscribe detaches', async () => {
  const { cache } = fixture();
  const snapshots: (string | undefined)[] = [];
  const unsubscribe = cache.subscribe('image', () => snapshots.push(cache.peek('image')));
  const image = cache.acquire('image', async () => new Blob(['image']));
  await image.promise;
  cache.clear();
  assert.deepEqual(snapshots, ['blob:1', undefined]);
  unsubscribe();
  const next = cache.acquire('image', async () => new Blob(['next']));
  await next.promise;
  assert.deepEqual(snapshots, ['blob:1', undefined]);
  image.release();
  next.release();
  cache.clear();
});
