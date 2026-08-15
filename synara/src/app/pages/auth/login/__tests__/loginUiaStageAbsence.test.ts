import test from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';

// Work from the package root (synara/) the same way modernization tests do.
const packageRoot = process.cwd();
const loginDir = path.join(packageRoot, 'src/app/pages/auth/login');
const authDir = path.join(packageRoot, 'src/app/pages/auth');
const componentsDir = path.join(packageRoot, 'src/app/components');
const repoRoot = path.resolve(packageRoot, '..');

const read = (p: string) => readFileSync(p, 'utf8');

test('V-AUTH.3b: login route has no multi-stage UIA overlay or stage owners', () => {
  const loginTsx = read(path.join(loginDir, 'Login.tsx'));
  assert.match(loginTsx, /PasswordLoginForm/);
  assert.doesNotMatch(
    loginTsx,
    /UIAFlowOverlay|useUIAFlow|uia-stages|SupportedUIAFlowsLoader|matrix_uia_|submitAuthDict/
  );

  const passwordForm = read(path.join(loginDir, 'PasswordLoginForm.tsx'));
  assert.match(passwordForm, /loginPassword/);
  assert.doesNotMatch(
    passwordForm,
    /UIAFlowOverlay|useUIAFlow|uia-stages|SupportedUIAFlowsLoader|matrix_uia_|submitAuthDict|AuthStageType/
  );

  // Login util must not drive a multi-stage interactive-auth loop.
  // (Password fail-closed native-only residual is owned by open #279; do not
  // re-home that surface here.)
  const loginUtil = read(path.join(loginDir, 'loginUtil.ts'));
  assert.match(loginUtil, /matrix_login_password/);
  assert.doesNotMatch(
    loginUtil,
    /matrix_uia_|interactiveAuth|InteractiveAuth|UIAFlowOverlay|submitAuthDict|auth\.login/
  );
});

test('V-AUTH.3b: product has no generic matrix_uia_* login-stage IPC', () => {
  const libRs = read(path.join(repoRoot, 'src-tauri/src/lib.rs'));
  assert.match(libRs, /matrix_login_password/);
  assert.doesNotMatch(libRs, /matrix_uia_/);

  // Auth product commands were extracted from product.rs. Keep this guard
  // pointed at the actual native command owner so it proves the product
  // remains native-only instead of relying on the module wrapper.
  const productCommandsRs = read(
    path.join(repoRoot, 'src-tauri/src/matrix/auth/product_commands.rs')
  );
  const productCommandsProd = productCommandsRs.split('#[cfg(test)]')[0] ?? productCommandsRs;
  const deviceCommandsRs = read(
    path.join(repoRoot, 'src-tauri/src/matrix/devices/product_commands.rs')
  );
  const deviceCommandsProd = deviceCommandsRs.split('#[cfg(test)]')[0] ?? deviceCommandsRs;
  assert.doesNotMatch(productCommandsProd, /pub async fn matrix_uia_/);
  assert.match(productCommandsProd, /pub async fn matrix_login_password/);
  // Specialized native multi-stage / UIAA owners remain elsewhere.
  assert.match(productCommandsProd, /pub async fn matrix_register/);
  assert.match(productCommandsProd, /pub async fn matrix_password_reset_complete/);
  assert.match(deviceCommandsProd, /pub async fn matrix_device_delete_password/);
});

test('V-AUTH.3b: UIA stage UI consumers are register/reset only (not login)', () => {
  const authFiles: string[] = [];
  const walk = (dir: string) => {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        if (entry.name === '__tests__' || entry.name === 'node_modules') continue;
        walk(full);
        continue;
      }
      if (entry.name.endsWith('.ts') || entry.name.endsWith('.tsx')) {
        authFiles.push(full);
      }
    }
  };
  walk(authDir);

  const stageImporters = authFiles.filter((file) => {
    const src = read(file);
    return (
      src.includes('UIAFlowOverlay') ||
      src.includes('useUIAFlow') ||
      src.includes("from '../../../components/uia-stages'") ||
      src.includes('from "../../components/uia-stages"') ||
      src.includes("from '../../../hooks/useUIAFlows'") ||
      src.includes('from "../../hooks/useUIAFlows"')
    );
  });

  const relative = stageImporters.map((f) => path.relative(packageRoot, f).replaceAll('\\', '/'));
  for (const rel of relative) {
    assert.doesNotMatch(rel, /pages\/auth\/login\//, `login must not import UIA stages: ${rel}`);
  }

  assert.ok(
    relative.some((rel) => rel.includes('pages/auth/register/')),
    'register must remain the multi-stage UIA product consumer'
  );
  assert.ok(
    relative.some((rel) => rel.includes('pages/auth/reset-password/')),
    'password-reset may use UIAFlowOverlay chrome'
  );

  // Stage components themselves stay SDK-neutral shared UI for register.
  const stagesIndex = read(path.join(componentsDir, 'uia-stages/index.ts'));
  assert.doesNotMatch(stagesIndex, /from ['"]matrix-js-sdk/);
});

test('V-AUTH.3b: native login maps UIAA to fail-closed InteractiveAuthRequired (no stage IPC)', () => {
  const loginRs = read(path.join(repoRoot, 'crates/synara-core/src/app/auth/login.rs'));
  const loginProd = loginRs.split('#[cfg(test)]')[0] ?? loginRs;
  assert.match(loginProd, /InteractiveAuthRequired/);
  assert.match(loginProd, /p3\.2-login-uiaa-required/);
  assert.doesNotMatch(loginProd, /matrix_uia_|UiaSession::begin|begin_submit/);
});
