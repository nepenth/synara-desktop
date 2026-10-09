import assert from 'node:assert/strict';
import test from 'node:test';
import { AutoDiscoveryAction } from '../../../cs-api';
import { resolveAuthServer, type AuthServerResolutionDeps } from '../authServerResolution';

const deps = (overrides: Partial<AuthServerResolutionDeps> = {}): AuthServerResolutionDeps => ({
  request: (async () => new Response()) as typeof fetch,
  discover: async (_request, server) => [
    undefined,
    { 'm.homeserver': { base_url: `https://${server}` } },
  ],
  versions: async () => ({ versions: ['v1.12'] }),
  loginFlows: async () => ({ flows: [{ kind: 'password', matrixType: 'm.login.password' }] }),
  ...overrides,
});

test('resolves discovery, versions and login flows in one step', async () => {
  const result = await resolveAuthServer('matrix.example.test', deps());
  assert.equal(result.ok, true);
  if (!result.ok) return;
  assert.equal(result.resolved.serverName, 'matrix.example.test');
  assert.equal(
    result.resolved.autoDiscoveryInfo['m.homeserver'].base_url,
    'https://matrix.example.test'
  );
  assert.deepEqual(result.resolved.specVersions.versions, ['v1.12']);
  assert.equal(result.resolved.authFlows.loginFlows.flows[0].kind, 'password');
});

test('each failing stage reports a short, distinct message', async () => {
  const failPrompt = await resolveAuthServer(
    'example.test',
    deps({
      discover: async () => [
        { host: 'https://example.test', action: AutoDiscoveryAction.FAIL_PROMPT },
        undefined,
      ],
    })
  );
  assert.deepEqual(failPrompt, {
    ok: false,
    message: "https://example.test advertises a homeserver that can't be used.",
  });

  const failError = await resolveAuthServer(
    'bad host',
    deps({
      discover: async () => [
        { host: 'bad host', action: AutoDiscoveryAction.FAIL_ERROR },
        undefined,
      ],
    })
  );
  assert.deepEqual(failError, {
    ok: false,
    message: 'bad host has an invalid homeserver address.',
  });

  const thrown = await resolveAuthServer(
    'x.test',
    deps({
      discover: async () => {
        throw new Error('boom');
      },
    })
  );
  assert.deepEqual(thrown, { ok: false, message: "Couldn't find a homeserver at x.test." });

  const down = await resolveAuthServer(
    'down.test',
    deps({
      versions: async () => {
        throw new Error('offline');
      },
    })
  );
  assert.deepEqual(down, { ok: false, message: "down.test isn't responding right now." });

  const noFlows = await resolveAuthServer(
    'flows.test',
    deps({
      loginFlows: async () => {
        throw new Error('ipc');
      },
    })
  );
  assert.deepEqual(noFlows, {
    ok: false,
    message: "Couldn't load sign-in options from flows.test.",
  });
});
