import assert from 'node:assert/strict';
import test from 'node:test';
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import {
  discoverLoginFlows,
  getPasswordFlow,
  NativeLoginFlowsError,
  type NativeLoginFlowsInvoke,
} from '../nativeLoginFlows';

const packageRoot = process.cwd();
const repoRoot = path.resolve(packageRoot, '..');

test('discoverLoginFlows invokes matrix_login_flows with homeserverUrl', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: NativeLoginFlowsInvoke = async (command, args) => {
    calls.push({ command, args });
    return {
      available: true,
      value: {
        flows: [
          { kind: 'password', matrixType: 'm.login.password' },
          { kind: 'token', matrixType: 'm.login.token', getLoginToken: true },
        ],
      },
    };
  };
  const result = await discoverLoginFlows('https://hs.example.org', invoke);
  assert.equal(calls[0]?.command, 'matrix_login_flows');
  assert.equal(calls[0]?.args?.homeserverUrl, 'https://hs.example.org');
  assert.equal(result.flows.length, 2);
  assert.equal(result.flows[0]?.kind, 'password');
  assert.equal(result.flows[0]?.matrixType, 'm.login.password');
  assert.equal('accessToken' in result.flows[0], false);
});

test('discoverLoginFlows fails closed when native command is unavailable', async () => {
  const invoke: NativeLoginFlowsInvoke = async () => ({ available: false });
  await assert.rejects(
    () => discoverLoginFlows('https://hs.example.org', invoke),
    (err: unknown) => {
      assert.ok(err instanceof NativeLoginFlowsError);
      assert.match(err.message, /unavailable/i);
      return true;
    }
  );
});

test('getPasswordFlow selects password kind from native DTOs', () => {
  const password = getPasswordFlow([
    { kind: 'token', matrixType: 'm.login.token', getLoginToken: false },
    { kind: 'password', matrixType: 'm.login.password' },
  ]);
  assert.equal(password?.kind, 'password');
  assert.equal(password?.matrixType, 'm.login.password');
  assert.equal(getPasswordFlow([{ kind: 'token', matrixType: 'm.login.token' }]), undefined);
});

test('V-AUTH.3: AuthFlowsLoader has no live matrix-js-sdk client', () => {
  const loader = readFileSync(
    path.join(packageRoot, 'src/app/components/AuthFlowsLoader.tsx'),
    'utf8'
  );
  assert.doesNotMatch(loader, /from ['"]matrix-js-sdk['"]/);
  assert.doesNotMatch(loader, /createClient/);
  assert.doesNotMatch(loader, /\.loginFlows\s*\(/);
  assert.match(loader, /discoverLoginFlows|matrix_login_flows/);

  const useAuthFlows = readFileSync(
    path.join(packageRoot, 'src/app/hooks/useAuthFlows.ts'),
    'utf8'
  );
  assert.doesNotMatch(useAuthFlows, /from ['"]matrix-js-sdk/);

  const parsed = readFileSync(
    path.join(packageRoot, 'src/app/hooks/useParsedLoginFlows.ts'),
    'utf8'
  );
  assert.doesNotMatch(parsed, /from ['"]matrix-js-sdk/);
  assert.match(parsed, /matrixType|LoginFlowDto/);

  const libRs = readFileSync(path.join(repoRoot, 'src-tauri/src/lib.rs'), 'utf8');
  assert.match(libRs, /matrix_login_flows/);

  assert.equal(
    existsSync(path.join(packageRoot, 'src/app/pages/auth/login/nativeLoginFlows.ts')),
    true
  );
});
