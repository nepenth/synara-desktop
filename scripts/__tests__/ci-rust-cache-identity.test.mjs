import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const repository = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../..",
);

function withFixture(run) {
  const directory = mkdtempSync(
    path.join(os.tmpdir(), "synara-rust-cache-identity-"),
  );
  const bin = path.join(directory, "bin");
  mkdirSync(bin);
  mkdirSync(path.join(directory, "scripts/lib"), { recursive: true });
  writeFileSync(
    path.join(directory, "scripts/ci-rust-cache-identity.sh"),
    readFileSync(path.join(repository, "scripts/ci-rust-cache-identity.sh")),
  );
  const localHelper = readFileSync(
    path.join(repository, "scripts/lib/rust-cache.sh"),
    "utf8",
  );
  const commands = {
    rustc: 'printf "%s\\n" "${FAKE_RUSTC_VERSION:-rustc 1.96.1}"',
    "xcode-select":
      'printf "%s\\n" "/Applications/Xcode_26.6.app/Contents/Developer"',
    xcodebuild:
      'printf "%s\\n" "${FAKE_XCODE_VERSION:-Xcode 26.6 Build version 17F113}"',
    xcrun:
      'case "$2" in macosx) printf "%s\\n" "${FAKE_MACOS_SDK:-26.6}" ;; iphoneos) printf "%s\\n" "${FAKE_IOS_SDK:-26.6}" ;; iphonesimulator) printf "%s\\n" "${FAKE_SIMULATOR_SDK:-26.6}" ;; *) exit 1 ;; esac',
  };
  for (const [name, command] of Object.entries(commands)) {
    writeFileSync(path.join(bin, name), `#!/bin/sh\n${command}\n`, {
      mode: 0o755,
    });
  }
  const identify = (overrides = {}) => {
    writeFileSync(
      path.join(directory, "scripts/lib/rust-cache.sh"),
      localHelper.replace(
        /^SYNARA_KACHE_VERSION=.*$/m,
        `SYNARA_KACHE_VERSION=${overrides.FAKE_KACHE_VERSION ?? "1.0.0"}`,
      ),
    );
    const output = path.join(directory, "output");
    const environment = path.join(directory, "environment");
    writeFileSync(output, "");
    writeFileSync(environment, "");
    const result = spawnSync("bash", ["scripts/ci-rust-cache-identity.sh"], {
      cwd: directory,
      encoding: "utf8",
      env: {
        ...process.env,
        PATH: `${bin}${path.delimiter}${process.env.PATH}`,
        SYNARA_CACHE_FAMILY: "validate-rust-desktop",
        SYNARA_CACHE_WRITER: "false",
        GITHUB_REF: "refs/pull/123/merge",
        GITHUB_OUTPUT: output,
        GITHUB_ENV: environment,
        GITHUB_WORKSPACE: repository,
        RUNNER_OS: "Linux",
        RUNNER_ARCH: "X64",
        DEVELOPER_DIR: "/Applications/Xcode_26.6.app/Contents/Developer",
        ...overrides,
      },
    });
    return {
      ...result,
      prefix: readFileSync(output, "utf8")
        .trim()
        .replace(/^compiler-prefix=/, ""),
      environment: readFileSync(environment, "utf8"),
    };
  };
  try {
    run(identify);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

test("compiler identity allows PR readers and main writers without archiving target paths", () => {
  withFixture((identify) => {
    const reader = identify();
    assert.equal(reader.status, 0, reader.stderr);
    assert.match(
      reader.prefix,
      /^synara-kache-v1-validate-rust-desktop-Linux-X64-[0-9a-f]{64}$/,
    );
    assert.ok(reader.environment.includes("CARGO_INCREMENTAL=0\n"));
    assert.ok(
      reader.environment.includes(`KACHE_CONFIG=${repository}/.kache.toml\n`),
    );
    assert.ok(reader.environment.includes("KACHE_HOST_CONFIG=\n"));
    assert.ok(
      reader.environment.includes("KACHE_AUTO_CLEAN_ORPHANED_TARGETS=0\n"),
    );
    const writer = identify({
      SYNARA_CACHE_WRITER: "true",
      GITHUB_REF: "refs/heads/main",
    });
    assert.equal(writer.status, 0, writer.stderr);
    assert.equal(writer.prefix, reader.prefix);
  });
});

test("compiler identity rejects non-main writers and malformed contracts before publishing environment", () => {
  withFixture((identify) => {
    for (const overrides of [
      { SYNARA_CACHE_WRITER: "true" },
      { SYNARA_CACHE_WRITER: "true", GITHUB_REF: "refs/tags/v1.0.0" },
      { SYNARA_CACHE_WRITER: "maybe" },
      { SYNARA_CACHE_FAMILY: "../target" },
    ]) {
      const result = identify(overrides);
      assert.notEqual(result.status, 0);
      assert.equal(result.prefix, "");
      assert.equal(result.environment, "");
    }
  });
});

test("compiler cache restore namespaces isolate compiler, platform and graph changes", () => {
  withFixture((identify) => {
    const baseline = identify().prefix;
    for (const overrides of [
      { FAKE_KACHE_VERSION: "1.1.0" },
      { FAKE_RUSTC_VERSION: "rustc 1.97.0" },
      { RUNNER_ARCH: "ARM64" },
      { SYNARA_CACHE_FAMILY: "release-linux-arch" },
    ]) {
      const result = identify(overrides);
      assert.equal(result.status, 0, result.stderr);
      assert.notEqual(result.prefix, baseline);
    }
    assert.equal(identify({ FAKE_IOS_SDK: "27.0" }).prefix, baseline);
  });
});

test("Apple restore namespaces isolate selected Xcode and every Rust target SDK", () => {
  withFixture((identify) => {
    const apple = { RUNNER_OS: "macOS", RUNNER_ARCH: "ARM64" };
    const baseline = identify(apple);
    assert.equal(baseline.status, 0, baseline.stderr);
    for (const overrides of [
      { DEVELOPER_DIR: "/Applications/Xcode_27.app/Contents/Developer" },
      { FAKE_XCODE_VERSION: "Xcode 27.0 Build version 18A123" },
      { FAKE_MACOS_SDK: "27.0" },
      { FAKE_IOS_SDK: "27.0" },
      { FAKE_SIMULATOR_SDK: "27.0" },
    ]) {
      const result = identify({ ...apple, ...overrides });
      assert.equal(result.status, 0, result.stderr);
      assert.notEqual(result.prefix, baseline.prefix);
    }
  });
});
