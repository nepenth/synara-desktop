import test from 'node:test';
import assert from 'node:assert/strict';
import { autoDiscovery, AutoDiscoveryAction } from '../cs-api';

test('auto discovery rejects remote plaintext homeservers before fetching', async () => {
  let requestCount = 0;
  const request = (async () => {
    requestCount += 1;
    return new Response('', { status: 404 });
  }) as typeof fetch;

  const [error, discovery] = await autoDiscovery(request, 'http://matrix.example.org');
  assert.equal(error?.action, AutoDiscoveryAction.FAIL_ERROR);
  assert.equal(discovery, undefined);
  assert.equal(requestCount, 0);
});

test('auto discovery retains exact loopback HTTP for local development', async () => {
  const request = (async () => new Response('', { status: 404 })) as typeof fetch;
  const [error, discovery] = await autoDiscovery(request, 'http://127.0.0.1:8008');

  assert.equal(error, undefined);
  assert.equal(discovery?.['m.homeserver'].base_url, 'http://127.0.0.1:8008');
});

test('well-known discovery rejects a remote plaintext base URL', async () => {
  const request = (async () =>
    new Response(JSON.stringify({ 'm.homeserver': { base_url: 'http://matrix.example.org' } }), {
      status: 200,
      headers: { 'content-type': 'application/json' },
    })) as typeof fetch;

  const [error, discovery] = await autoDiscovery(request, 'example.org');
  assert.equal(error?.action, AutoDiscoveryAction.FAIL_ERROR);
  assert.equal(discovery, undefined);
});

test('well-known discovery drops an insecure optional identity server', async () => {
  const request = (async () =>
    new Response(
      JSON.stringify({
        'm.homeserver': { base_url: 'https://matrix.example.org' },
        'm.identity_server': { base_url: 'http://identity.example.org' },
      }),
      { status: 200, headers: { 'content-type': 'application/json' } }
    )) as typeof fetch;

  const [error, discovery] = await autoDiscovery(request, 'example.org');
  assert.equal(error, undefined);
  assert.equal(discovery?.['m.identity_server'], undefined);
});

test('well-known types accept either rtc_foci key and non-livekit transports', () => {
  const msc4143: import('../cs-api').AutoDiscoveryInfo = {
    'm.homeserver': { base_url: 'https://matrix.example.org' },
    'org.matrix.msc4143.rtc_foci': [
      { type: 'livekit', livekit_service_url: 'https://livekit.example.org' },
      { type: 'local.custom.sfu' },
    ],
  };
  const stable: import('../cs-api').AutoDiscoveryInfo = {
    'm.homeserver': { base_url: 'https://matrix.example.org' },
    'm.rtc_foci': [{ type: 'livekit', livekit_service_url: 'https://livekit.example.org' }],
  };
  assert.equal(msc4143['org.matrix.msc4143.rtc_foci']?.[0]?.type, 'livekit');
  assert.equal(msc4143['org.matrix.msc4143.rtc_foci']?.[1]?.type, 'local.custom.sfu');
  assert.equal(stable['m.rtc_foci']?.[0]?.type, 'livekit');
  assert.equal('m.rtc_foci' in msc4143 && 'org.matrix.msc4143.rtc_foci' in stable, false);
});
