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

for (const cfg of [
  "test",
  "all(test)",
  "any(test)",
  'all(test, feature = "fixture")',
  'any(test, all(test, feature = "fixture"))',
  "all(\n test,\n)",
]) {
  test(`production-false outer cfg ${JSON.stringify(
    cfg
  )} cannot supply a boundary`, () => {
    fixture((entry, root) => {
      writeFileSync(
        entry,
        `// 🦀 Unicode keeps source offsets honest.\n#[derive(Debug)] #[cfg(${cfg})] mod fixture;\nmod recovery;\n`
      );
      writeFileSync(
        join(root, "facade/fixture.rs"),
        "pub fn TEST_SECRET() {}\n"
      );
      assert.doesNotMatch(readRustModuleSources(entry).source, /TEST_SECRET/);
      assert.match(
        readRustModuleSources(entry, { includeTests: true }).source,
        /TEST_SECRET/
      );
    });
  });
}
for (const cfg of ["test", "all(test)", "any(test)", "all(\n test,\n)"]) {
  test(`production-false inner cfg ${JSON.stringify(
    cfg
  )} excludes the entire child`, () => {
    fixture((entry, root) => {
      writeFileSync(
        join(root, "facade/recovery.rs"),
        `#![cfg(${cfg})]\npub fn TEST_SECRET() {}\nmod missing;\n`
      );
      assert.doesNotMatch(readRustModuleSources(entry).source, /TEST_SECRET/);
    });
  });
}
for (const cfg of [
  "not(test)",
  'any(test, feature = "full")',
  'all(not(test), feature = "full")',
]) {
  test(`potential production cfg ${JSON.stringify(
    cfg
  )} remains in scope`, () => {
    fixture((entry) => {
      writeFileSync(entry, `#[cfg(${cfg})] mod recovery;\n`);
      assert.match(readRustModuleSources(entry).source, /typed_recovery/);
    });
  });
}
test("attached and multiline path overrides fail closed wherever they appear", () => {
  for (const prefix of [
    '#[path = "real.rs"]',
    '#[cfg(feature = "x")] #[path = "real.rs"]',
    '#[cfg(test)]\n#[path\n= "real.rs"]',
  ]) {
    fixture((entry) => {
      writeFileSync(entry, `${prefix} mod recovery;\n`);
      assert.throws(() => readRustModuleSources(entry), /path override/);
    });
  }
});
test("compiled include macros fail explicitly while comments and literal examples stay inert", () => {
  fixture((entry) => {
    for (const macro of [
      'include!("hidden.rs");',
      'include!{"hidden.rs"}',
      'include!["hidden.rs"];',
    ]) {
      writeFileSync(entry, `${macro}\n`);
      assert.throws(() => readRustModuleSources(entry), /include macro/);
    }
    writeFileSync(
      entry,
      '// include!{"hidden.rs"}\nconst EXAMPLE: &str = r#"include!["hidden.rs"];"#;\nmod recovery;\n'
    );
    assert.equal(readRustModuleSources(entry).files.length, 2);
  });
});
test("ambiguous module layouts and unsupported nested external declarations fail closed", () => {
  fixture((entry, root) => {
    mkdirSync(join(root, "facade/recovery"));
    writeFileSync(
      join(root, "facade/recovery/mod.rs"),
      "pub fn alternate() {}\n"
    );
    assert.throws(() => readRustModuleSources(entry), /Expected one source/);
    writeFileSync(entry, "mod inline { mod recovery; }\n");
    assert.throws(() => readRustModuleSources(entry), /nested external/);
  });
});
test("conditional attributes which can add cfg fail explicitly rather than supplying test evidence", () => {
  fixture((entry) => {
    writeFileSync(entry, "#[cfg_attr(not(test), cfg(test))] mod recovery;\n");
    assert.throws(() => readRustModuleSources(entry), /conditional cfg/);
  });
});

test("a production module named tests cannot hide compiled source", () => {
  fixture((entry) => {
    writeFileSync(entry, "mod tests;\n");
    assert.match(readRustModuleSources(entry).source, /TEST_SECRET/);
  });
});
test("nested inner cfg cannot blank unrelated production declarations", () => {
  fixture((entry) => {
    writeFileSync(
      entry,
      "pub fn secret_transport() {}\nmod inner { #![cfg(test)] fn fake() {} }\n"
    );
    assert.throws(() => readRustModuleSources(entry), /nested inner cfg/);
  });
});

test("conditional path overrides cannot redirect production source silently", () => {
  fixture((entry) => {
    writeFileSync(
      entry,
      '#[cfg_attr(feature = "full", path = "hidden.rs")] mod recovery;\n'
    );
    assert.throws(() => readRustModuleSources(entry), /path override/);
  });
});

test("raw ASCII module identifiers visit their actual filename", () => {
  fixture((entry, root) => {
    writeFileSync(entry, "pub mod r#async;\n");
    writeFileSync(
      join(root, "facade/async.rs"),
      "pub fn authoritative_source() {}\n"
    );
    const result = readRustModuleSources(entry);
    assert.equal(result.files.length, 2);
    assert.match(result.source, /authoritative_source/);
    rmSync(join(root, "facade/async.rs"));
    assert.throws(() => readRustModuleSources(entry), /Expected one source/);
  });
});
test("unsupported Unicode module identifiers cannot omit compiled source silently", () => {
  fixture((entry, root) => {
    for (const identifier of ["café", "r#café", "來源"]) {
      writeFileSync(entry, `mod ${identifier};\n`);
      writeFileSync(
        join(root, `facade/${identifier.replace(/^r#/, "")}.rs`),
        "pub fn hidden_transport() {}\n"
      );
      assert.throws(
        () => readRustModuleSources(entry),
        /Unsupported Rust module identifier/
      );
    }
  });
});
