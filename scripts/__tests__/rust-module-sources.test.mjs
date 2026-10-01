import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { readRustModuleSources } from "../lib/rust-module-sources.mjs";

function fixture(callback) {
  const root = mkdtempSync(join(tmpdir(), "synara-rust-modules-"));
  try {
    mkdirSync(join(root, "facade"));
    writeFileSync(
      join(root, "facade.rs"),
      "mod recovery;\n#[cfg(test)]\nmod tests;\n"
    );
    writeFileSync(
      join(root, "facade/recovery.rs"),
      "pub fn typed_recovery() {}\n"
    );
    writeFileSync(
      join(root, "facade/tests.rs"),
      'const TEST_SECRET: &str = "fixture";\n'
    );
    callback(join(root, "facade.rs"), root);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}
test("a split production operation participates in facade boundary validation", () => {
  fixture((entry) => {
    const result = readRustModuleSources(entry);
    assert.equal(result.files.length, 2);
    assert.match(result.source, /typed_recovery/);
    assert.doesNotMatch(result.source, /TEST_SECRET/);
    assert.match(
      readRustModuleSources(entry, { includeTests: true }).source,
      /TEST_SECRET/
    );
  });
});
test("undeclared sibling files cannot provide false positive production evidence", () => {
  fixture((entry, root) => {
    writeFileSync(
      join(root, "facade/unwired.rs"),
      "pub fn secret_transport() {}\n"
    );
    assert.doesNotMatch(
      readRustModuleSources(entry).source,
      /secret_transport/
    );
  });
});
test("a declared but missing domain fails closed", () => {
  fixture((entry, root) => {
    rmSync(join(root, "facade/recovery.rs"));
    assert.throws(() => readRustModuleSources(entry), /Expected one source/);
  });
});

test("comments and Rust raw literals cannot declare production modules", () => {
  fixture((entry) => {
    writeFileSync(
      entry,
      '/* mod absent; */\nconst EXAMPLE: &str = r#"\nmod fake;\n"#;\nmod recovery;\n'
    );
    assert.equal(readRustModuleSources(entry).files.length, 2);
  });
});

test("inline test modules and test methods cannot satisfy production guards", () => {
  fixture((entry, root) => {
    writeFileSync(
      join(root, "facade/recovery.rs"),
      "pub fn production() {}\n#[cfg(test)]\nmod tests { fn fake_boundary() {} }\nimpl Facade { #[cfg(test)] pub fn fake_method() {} }\n"
    );
    const production = readRustModuleSources(entry).source;
    assert.match(production, /fn production/);
    assert.doesNotMatch(production, /fake_boundary|fake_method/);
    assert.match(
      readRustModuleSources(entry, { includeTests: true }).source,
      /fake_boundary|fake_method/
    );
  });
});
