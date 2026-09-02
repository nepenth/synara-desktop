import assert from "node:assert/strict";
import {
  chmodSync,
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("../..", import.meta.url)));
const checker = resolve(root, "scripts/check-xcode-local-package-graph.mjs");

const localProject = (path = "Root") => `
/* Begin XCLocalSwiftPackageReference section */
  ABC /* XCLocalSwiftPackageReference */ = {
    isa = XCLocalSwiftPackageReference;
    relativePath = ${path};
  };
/* End XCLocalSwiftPackageReference section */
`;

function fixture() {
  const directory = mkdtempSync(join(tmpdir(), "synara-package-graph-test."));
  const bin = join(directory, "bin");
  mkdirSync(bin);
  const mockSwift = join(bin, "swift");
  writeFileSync(
    mockSwift,
    `#!/usr/bin/env node
const packagePath = process.argv.at(-1);
const entry = JSON.parse(process.env.SYNARA_PACKAGE_GRAPH_FIXTURES)[packagePath];
if (entry === undefined) process.exit(86);
process.stdout.write(entry);
`,
  );
  chmodSync(mockSwift, 0o755);
  return {
    directory,
    cleanup: () => rmSync(directory, { recursive: true, force: true }),
  };
}

function run(fixtureRoot, project, manifests = {}) {
  const projectPath = join(fixtureRoot, "project.pbxproj");
  writeFileSync(projectPath, project);
  return spawnSync(process.execPath, [checker, projectPath, fixtureRoot], {
    encoding: "utf8",
    env: {
      ...process.env,
      PATH: `${join(fixtureRoot, "bin")}:${process.env.PATH}`,
      SYNARA_PACKAGE_GRAPH_FIXTURES: JSON.stringify(manifests),
    },
  });
}

test("classifies a direct Xcode remote reference without inspecting unrelated files", () => {
  const context = fixture();
  try {
    const result = run(
      context.directory,
      "isa = XCRemoteSwiftPackageReference; repositoryURL = https://example.invalid/repo;",
    );
    assert.equal(result.status, 0, result.stderr);
    assert.equal(result.stdout.trim(), "remote");
  } finally {
    context.cleanup();
  }
});

test("walks reachable local dependencies and ignores an unrelated remote spike", () => {
  const context = fixture();
  try {
    const rootPackage = join(context.directory, "Root");
    const nestedPackage = join(context.directory, "Nested");
    const unrelatedSpike = join(context.directory, "spikes", "RemoteProbe");
    for (const path of [rootPackage, nestedPackage, unrelatedSpike]) mkdirSync(path, { recursive: true });
    const result = run(context.directory, localProject(), {
      [realpathSync(rootPackage)]: JSON.stringify({
        dependencies: [{ fileSystem: [{ path: "../Nested" }] }],
      }),
      [realpathSync(nestedPackage)]: JSON.stringify({ dependencies: [] }),
      [realpathSync(unrelatedSpike)]: JSON.stringify({ dependencies: [{ sourceControl: [{}] }] }),
    });
    assert.equal(result.status, 0, result.stderr);
    assert.equal(result.stdout.trim(), "local");
  } finally {
    context.cleanup();
  }
});

test("detects a remote dependency nested behind a local package", () => {
  const context = fixture();
  try {
    const rootPackage = join(context.directory, "Root");
    const nestedPackage = join(context.directory, "Nested");
    for (const path of [rootPackage, nestedPackage]) mkdirSync(path);
    const result = run(context.directory, localProject(), {
      [realpathSync(rootPackage)]: JSON.stringify({
        dependencies: [{ fileSystem: [{ path: "../Nested" }] }],
      }),
      [realpathSync(nestedPackage)]: JSON.stringify({ dependencies: [{ sourceControl: [{}] }] }),
    });
    assert.equal(result.status, 0, result.stderr);
    assert.equal(result.stdout.trim(), "remote");
  } finally {
    context.cleanup();
  }
});

test("fails closed for missing packages and malformed manifest output", () => {
  const missing = fixture();
  try {
    const missingResult = run(missing.directory, localProject());
    assert.notEqual(missingResult.status, 0);
  } finally {
    missing.cleanup();
  }

  const malformed = fixture();
  try {
    const rootPackage = join(malformed.directory, "Root");
    mkdirSync(rootPackage);
    const malformedResult = run(malformed.directory, localProject(), {
      [realpathSync(rootPackage)]: "not-json",
    });
    assert.notEqual(malformedResult.status, 0);
  } finally {
    malformed.cleanup();
  }
});

test("fails closed for an unknown SwiftPM dependency schema", () => {
  const context = fixture();
  try {
    const rootPackage = join(context.directory, "Root");
    mkdirSync(rootPackage);
    const result = run(context.directory, localProject(), {
      [realpathSync(rootPackage)]: JSON.stringify({
        dependencies: [{ futureDependencyKind: [{ location: "unknown" }] }],
      }),
    });
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /unrecognized Swift dependency variant/);
  } finally {
    context.cleanup();
  }
});

for (const dependency of [
  { sourceControl: null },
  { registry: [] },
  { fileSystem: null },
  { fileSystem: [] },
  { fileSystem: [{ path: "" }] },
]) {
  test(`fails closed for malformed known SwiftPM dependency ${JSON.stringify(dependency)}`, () => {
    const context = fixture();
    try {
      const rootPackage = join(context.directory, "Root");
      mkdirSync(rootPackage);
      const result = run(context.directory, localProject(), {
        [realpathSync(rootPackage)]: JSON.stringify({ dependencies: [dependency] }),
      });
      assert.notEqual(result.status, 0);
      assert.match(result.stderr, /malformed|unrecognized/);
    } finally {
      context.cleanup();
    }
  });
}

test("classifies the generated Synara app package graph as all-local", () => {
  const result = spawnSync(
    process.execPath,
    [checker, resolve(root, "synara-ios/Synara.xcodeproj/project.pbxproj"), resolve(root, "synara-ios")],
    { encoding: "utf8" },
  );
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout.trim(), "local");
});
