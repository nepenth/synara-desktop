import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { loginPassword, mxIdLoginBaseUrl, type PasswordLoginInvoke } from '../loginUtil';

// matrix-v4.example.org is an IPv4 reverse proxy for a homeserver whose
// server name is matrix.example.org (served only over IPv6). Nothing the
// client can see before sign-in names the real server.
const PROXY = 'matrix-v4.example.org';
const PROXY_BASE_URL = 'https://matrix-v4.example.org';
const SERVER_NAME = 'matrix.example.org';

const loginDir = path.join(process.cwd(), 'src/app/pages/auth/login');
const openConfig = { allowCustomHomeservers: true };

const recordingInvoke = () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: PasswordLoginInvoke = async (command, args) => {
    calls.push({ command, args });
    return {
      available: true,
      value: {
        userId: `@pete:${SERVER_NAME}`,
        deviceId: 'DEVICEID',
        homeserverUrl: PROXY_BASE_URL,
      },
    };
  };
  return { calls, invoke };
};

test('a bare username is sent as typed, with the chosen server only as a store hint', async () => {
  const { calls, invoke } = recordingInvoke();
  const result = await loginPassword(
    PROXY_BASE_URL,
    {
      type: 'm.login.password',
      identifier: { type: 'm.id.user', user: 'Pete' },
      password: 's3cret',
    },
    { serverName: PROXY, isDesktop: () => true, invoke }
  );
  assert.equal(calls[0]?.args?.user, 'Pete');
  assert.equal(calls[0]?.args?.serverName, PROXY);
  assert.equal(calls[0]?.args?.homeserverUrl, PROXY_BASE_URL);
  // The account is whatever the host reports back from the server.
  assert.equal(result.identity.userId, `@pete:${SERVER_NAME}`);
});

test('the login form never completes a username with the chosen server', () => {
  const form = readFileSync(path.join(loginDir, 'PasswordLoginForm.tsx'), 'utf8');
  assert.doesNotMatch(form, /user:\s*`@\$\{username\}:\$\{server\}`/);
  assert.match(form, /user:\s*username,/);
});

test('a Matrix ID on the chosen server uses its base URL without discovery', () => {
  const resolved = mxIdLoginBaseUrl(openConfig, PROXY.toUpperCase(), PROXY, PROXY_BASE_URL, () => {
    throw new Error('no discovery expected');
  });
  assert.equal(resolved, PROXY_BASE_URL);
});

test('a Matrix ID whose server is unreachable signs in through the chosen proxy', async () => {
  const requested: string[] = [];
  const unreachable: typeof fetch = async (input) => {
    requested.push(String(input));
    throw new TypeError('connect ETIMEDOUT');
  };
  const resolved = mxIdLoginBaseUrl(openConfig, SERVER_NAME, PROXY, PROXY_BASE_URL, unreachable);
  assert.equal(typeof resolved, 'function');
  assert.equal(await (resolved as () => Promise<string>)(), PROXY_BASE_URL);
  assert.ok(requested.some((url) => url.startsWith(`https://${SERVER_NAME}/`)));
});

test('a Matrix ID on a reachable other server still signs in there', async () => {
  const reachable: typeof fetch = async (input) => {
    const url = String(input);
    if (url === 'https://example.org/.well-known/matrix/client') {
      return new Response(
        JSON.stringify({ 'm.homeserver': { base_url: 'https://matrix-client.example.org' } }),
        { status: 200 }
      );
    }
    if (url === 'https://matrix-client.example.org/_matrix/client/versions') {
      return new Response(JSON.stringify({ versions: ['v1.11'] }), { status: 200 });
    }
    return new Response(null, { status: 404 });
  };
  const resolved = mxIdLoginBaseUrl(openConfig, 'example.org', PROXY, PROXY_BASE_URL, reachable);
  assert.equal(await (resolved as () => Promise<string>)(), 'https://matrix-client.example.org');
});

test('a Matrix ID on a server the client config forbids is still refused', async () => {
  const resolved = mxIdLoginBaseUrl(
    { allowCustomHomeservers: false, homeserverList: [PROXY] },
    SERVER_NAME,
    PROXY,
    PROXY_BASE_URL,
    async () => {
      throw new Error('no discovery expected');
    }
  );
  await assert.rejects((resolved as () => Promise<string>)(), /NotAllow/);
});
