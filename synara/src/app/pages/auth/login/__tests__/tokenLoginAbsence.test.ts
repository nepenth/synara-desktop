import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';

// Work from the package root (synara/) the same way modernization tests do.
const packageRoot = process.cwd();
const loginDir = path.join(packageRoot, 'src/app/pages/auth/login');
const authDir = path.join(packageRoot, 'src/app/pages/auth');
const repoRoot = path.resolve(packageRoot, '..');

test('V-AUTH.2: desktop product has no TokenLogin surface or loginToken route', () => {
  assert.equal(
    existsSync(path.join(loginDir, 'TokenLogin.tsx')),
    false,
    'TokenLogin.tsx must remain deleted'
  );
  assert.equal(
    existsSync(path.join(authDir, 'SSOLogin.tsx')),
    false,
    'SSOLogin.tsx must remain deleted (token completion had no SSO entry)'
  );

  const loginTsx = readFileSync(path.join(loginDir, 'Login.tsx'), 'utf8');
  assert.match(loginTsx, /PasswordLoginForm/);
  assert.doesNotMatch(loginTsx, /TokenLogin|loginToken|m\.login\.token/);

  const loginUtil = readFileSync(path.join(loginDir, 'loginUtil.ts'), 'utf8');
  assert.match(loginUtil, /matrix_login_password/);
  assert.doesNotMatch(loginUtil, /matrix_login_token|m\.login\.token/);

  const parsedFlows = readFileSync(
    path.join(packageRoot, 'src/app/hooks/useParsedLoginFlows.ts'),
    'utf8'
  );
  assert.doesNotMatch(parsedFlows, /m\.login\.token|token\?:/);
  assert.match(parsedFlows, /export type ParsedLoginFlows = \{\s*password\?: LoginFlowDto;\s*\}/s);

  // Password matrix type matching lives on the native login-flow owner (V-AUTH.3).
  const nativeFlows = readFileSync(
    path.join(packageRoot, 'src/app/pages/auth/login/nativeLoginFlows.ts'),
    'utf8'
  );
  assert.match(nativeFlows, /m\.login\.password/);
  assert.doesNotMatch(nativeFlows, /matrix_login_token/);

  const libRs = readFileSync(path.join(repoRoot, 'src-tauri/src/lib.rs'), 'utf8');
  assert.match(libRs, /matrix_login_password/);
  assert.doesNotMatch(libRs, /matrix_login_token/);

  const loginRs = readFileSync(path.join(repoRoot, 'crates/synara-core/src/app/auth/login.rs'), 'utf8');
  const loginProd = loginRs.split('#[cfg(test)]')[0] ?? loginRs;
  assert.doesNotMatch(loginProd, /fn login_with_token/);
  assert.doesNotMatch(loginProd, /\.login_token\(/);
});

test('V-AUTH.2: password-only login message is the only non-password product fallback', () => {
  const loginTsx = readFileSync(path.join(loginDir, 'Login.tsx'), 'utf8');
  assert.match(loginTsx, /Password based login method not found/);
  assert.doesNotMatch(loginTsx, /token login|Token login|SSO/i);
});
