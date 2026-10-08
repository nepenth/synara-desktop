import assert from "node:assert/strict";
import path from "node:path";
import test from "node:test";
import { readRustModuleSources } from "../lib/rust-module-sources.mjs";
import { inspectTypedRecoveryBoundaries } from "../lib/typed-recovery-boundaries.mjs";
import { readUdlSurface } from "../lib/ffi-surface.mjs";

const root = path.resolve(import.meta.dirname, "../..");
const inputs = {
  udl: readUdlSurface(root),
  ffi: readRustModuleSources(
    path.join(root, "crates/synara-core/src/shared_core_ffi.rs")
  ).source,
};
test("accepted recovery APIs use narrow typed secret transport", () => {
  assert.deepEqual(inspectTypedRecoveryBoundaries(inputs), {
    ok: true,
    errors: [],
  });
});
test("recovery inputs cannot be routed through generic command envelopes", () => {
  const ffi = inputs.ffi.replace(
    ".backup_setup(secret.as_str())",
    '.command(serde_json::json!({"passphrase": secret.as_str()}))'
  );
  assert.match(
    inspectTypedRecoveryBoundaries({ ...inputs, ffi }).errors.join("\n"),
    /generic envelopes/
  );
});
test("recovery adapters retain zeroization and exact typed UDL signatures", () => {
  const ffi = inputs.ffi.replaceAll(
    "Zeroizing::new(recovery_secret)",
    "recovery_secret"
  );
  assert.match(
    inspectTypedRecoveryBoundaries({ ...inputs, ffi }).errors.join("\n"),
    /zeroizing buffer/
  );
  const udl = inputs.udl.replace(
    "SecretStorageStatusDto secret_storage_unlock(string recovery_secret)",
    "SecretStorageStatusDto secret_storage_unlock(string payload_json)"
  );
  assert.match(
    inspectTypedRecoveryBoundaries({ ...inputs, udl }).errors.join("\n"),
    /dedicated typed UDL signature/
  );
});
test("the setup response cannot acquire a stored password or arbitrary secret fields", () => {
  const udl = inputs.udl.replace(
    "dictionary SecretStorageSetupDto {",
    "dictionary SecretStorageSetupDto {\n  string password;"
  );
  assert.match(
    inspectTypedRecoveryBoundaries({ ...inputs, udl }).errors.join("\n"),
    /only status/
  );
});

test("commented or literal recovery code cannot satisfy native secret boundaries", () => {
  const ffi = inputs.ffi.replaceAll(
    "Zeroizing::new(passphrase)",
    "/* Zeroizing::new(passphrase) */ passphrase"
  );
  assert.match(
    inspectTypedRecoveryBoundaries({ ...inputs, ffi }).errors.join("\n"),
    /zeroizing buffer/
  );
});
