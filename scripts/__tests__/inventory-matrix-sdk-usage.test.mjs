import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
  rmSync,
  existsSync,
} from "node:fs";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  buildInventory,
  checkSnapshots,
  classifyBucket,
  classifyFileRole,
  formatJsonArtifact,
  formatMarkdownArtifact,
  isDesktopRuntimePath,
  isMatrixSdkModule,
  renderMarkdown,
  scanNetworkingIndicators,
  stableStringify,
  writeSnapshots,
} from "../inventory-matrix-sdk-usage.mjs";

const REPO_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../.."
);

/** Same binary and cwd as the required external Prettier check. */
const PRETTIER_CLI = path.join(REPO_ROOT, "synara/node_modules/.bin/prettier");

/**
 * Format text via the external Prettier CLI (not the generator's helper).
 * Matches: ./synara/node_modules/.bin/prettier --stdin-filepath <rel>
 */
function formatViaExternalPrettierCli(text, repoRelativePath) {
  assert.ok(
    existsSync(PRETTIER_CLI),
    `Prettier CLI missing at ${PRETTIER_CLI}`
  );
  return execFileSync(PRETTIER_CLI, ["--stdin-filepath", repoRelativePath], {
    cwd: REPO_ROOT,
    input: text,
    encoding: "utf8",
  });
}

function writeFixture(root, relPath, content) {
  const abs = path.join(root, relPath);
  mkdirSync(path.dirname(abs), { recursive: true });
  writeFileSync(abs, content, "utf8");
}

function makeFixtureRoot(files) {
  const root = mkdtempSync(path.join(tmpdir(), "matrix-sdk-inventory-"));
  const fileList = [];
  for (const [relPath, content] of Object.entries(files)) {
    writeFixture(root, relPath, content);
    fileList.push(relPath.split(path.sep).join("/"));
  }
  fileList.sort();
  return { root, fileList };
}

test("classifies modules, buckets, roles, and runtime paths", () => {
  assert.equal(isMatrixSdkModule("matrix-js-sdk"), true);
  assert.equal(isMatrixSdkModule("matrix-js-sdk/lib/crypto-api"), true);
  assert.equal(
    isMatrixSdkModule("matrix-js-sdk/lib/matrixrtc/CallMembership"),
    true
  );
  assert.equal(isMatrixSdkModule("@matrix-org/something"), false);
  assert.equal(isMatrixSdkModule("not-matrix-js-sdk"), false);

  assert.equal(
    classifyBucket("synara/src/app/features/room/RoomView.tsx"),
    "feature"
  );
  assert.equal(classifyBucket("synara/src/app/hooks/useCall.ts"), "hook");
  assert.equal(
    classifyBucket("synara/src/client/initMatrix.ts"),
    "client-lifecycle"
  );
  assert.equal(
    classifyBucket("synara/scripts/run-synapse-two-client-integration.mjs"),
    null
  );
  assert.equal(isDesktopRuntimePath("synara/src/app/utils/matrix.ts"), true);
  assert.equal(isDesktopRuntimePath("synara/scripts/harness.mjs"), false);

  assert.equal(
    classifyFileRole("synara/src/app/utils/matrix.ts"),
    "production"
  );
  assert.equal(
    classifyFileRole("synara/src/app/utils/__tests__/matrix.test.ts"),
    "test"
  );
  assert.equal(
    classifyFileRole("synara/src/app/utils/matrix.spec.tsx"),
    "test"
  );
  assert.equal(
    classifyFileRole("synara/scripts/run-synapse-two-client-integration.mjs"),
    "tooling"
  );
  assert.equal(
    classifyFileRole("scripts/inventory-matrix-sdk-usage.mjs"),
    "tooling"
  );
});

test("detects root, deep, type-only, value, aliased, default, namespace, and dynamic imports", () => {
  const files = {
    "synara/src/app/features/sample/imports.ts": `
import type { MatrixClient } from 'matrix-js-sdk';
import { Room as MatrixRoom, MatrixEvent } from 'matrix-js-sdk';
import { CryptoEvent, type VerificationRequest } from 'matrix-js-sdk/lib/crypto-api';
import DefaultClient from 'matrix-js-sdk';
import * as MatrixSDK from 'matrix-js-sdk';
import { CallMembership } from 'matrix-js-sdk/lib/matrixrtc/CallMembership';
`,
    "synara/scripts/dynamic-harness.mjs": `
export async function loadSdk() {
  const sdk = await import('matrix-js-sdk');
  return sdk;
}
`,
  };
  const { root, fileList } = makeFixtureRoot(files);
  try {
    const inventory = buildInventory({ root, fileList });
    assert.equal(inventory.summary.repositoryWide.productionImportFiles, 1);
    assert.equal(inventory.summary.repositoryWide.testImportFiles, 0);
    assert.equal(inventory.summary.repositoryWide.toolingImportFiles, 1);
    assert.equal(
      inventory.summary.desktopRuntimeBaseline.productionImportFiles,
      1
    );
    assert.equal(inventory.summary.desktopRuntimeBaseline.testImportFiles, 0);

    const feature = inventory.files.find((f) =>
      f.path.endsWith("sample/imports.ts")
    );
    assert.ok(feature);
    assert.equal(feature.role, "production");
    assert.equal(feature.imports.length, 6);
    assert.ok(feature.imports.every((imp) => imp.form === "static"));

    const typeOnly = feature.imports.find(
      (imp) =>
        imp.module === "matrix-js-sdk" &&
        imp.isTypeOnly &&
        imp.namedImports.some((n) => n.name === "MatrixClient")
    );
    assert.ok(typeOnly, "type-only MatrixClient import");

    const aliased = feature.imports.find((imp) =>
      imp.namedImports.some(
        (n) => n.name === "Room" && n.alias === "MatrixRoom"
      )
    );
    assert.ok(aliased, "aliased Room import");

    const deep = feature.imports.find(
      (imp) => imp.module === "matrix-js-sdk/lib/crypto-api"
    );
    assert.ok(deep);
    assert.ok(
      deep.namedImports.some((n) => n.name === "CryptoEvent" && !n.isTypeOnly)
    );
    assert.ok(
      deep.namedImports.some(
        (n) => n.name === "VerificationRequest" && n.isTypeOnly
      )
    );

    assert.ok(
      feature.imports.find((imp) => imp.defaultImport === "DefaultClient")
    );
    assert.ok(
      feature.imports.find((imp) => imp.namespaceImport === "MatrixSDK")
    );
    assert.ok(
      feature.imports.find(
        (imp) => imp.module === "matrix-js-sdk/lib/matrixrtc/CallMembership"
      )
    );

    const harness = inventory.files.find((f) =>
      f.path.endsWith("dynamic-harness.mjs")
    );
    assert.ok(harness);
    assert.equal(harness.role, "tooling");
    assert.equal(harness.desktopRuntime, false);
    assert.equal(harness.imports.length, 1);
    assert.equal(harness.imports[0].form, "dynamic");
    assert.equal(harness.imports[0].module, "matrix-js-sdk");
    assert.ok(harness.imports[0].line >= 1);

    // Aggregates must not mix roles
    assert.equal(inventory.aggregates.production.importFileCount, 1);
    assert.equal(inventory.aggregates.tooling.importFileCount, 1);
    assert.equal(inventory.aggregates.test.importFileCount, 0);
    assert.ok(
      inventory.aggregates.production.modules.some((m) =>
        m.path.includes("matrixrtc")
      )
    );
    assert.equal(
      inventory.aggregates.tooling.modules.find(
        (m) => m.path === "matrix-js-sdk"
      )?.fileCount,
      1
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("classifies production vs test vs tooling independently", () => {
  const files = {
    "synara/src/app/hooks/useX.ts": `import type { MatrixClient } from 'matrix-js-sdk';\nexport type T = MatrixClient;\n`,
    "synara/src/app/hooks/__tests__/useX.test.ts": `import { Room } from 'matrix-js-sdk';\nexport const r: typeof Room | null = null;\n`,
    "synara/src/app/utils/helper.spec.ts": `import { MatrixError } from 'matrix-js-sdk';\nexport const e = MatrixError;\n`,
    "synara/scripts/tool.mjs": `import('matrix-js-sdk');\n`,
  };
  const { root, fileList } = makeFixtureRoot(files);
  try {
    const inventory = buildInventory({ root, fileList });
    assert.equal(inventory.summary.repositoryWide.productionImportFiles, 1);
    assert.equal(inventory.summary.repositoryWide.testImportFiles, 2);
    assert.equal(inventory.summary.repositoryWide.toolingImportFiles, 1);
    assert.equal(
      inventory.summary.desktopRuntimeBaseline.productionImportFiles,
      1
    );
    assert.equal(inventory.summary.desktopRuntimeBaseline.testImportFiles, 2);
    assert.equal(inventory.summary.desktopRuntimeBaseline.buckets.hook, 1);

    const roles = inventory.files.map((f) => [f.path, f.role]).sort();
    assert.deepEqual(roles, [
      ["synara/scripts/tool.mjs", "tooling"],
      ["synara/src/app/hooks/__tests__/useX.test.ts", "test"],
      ["synara/src/app/hooks/useX.ts", "production"],
      ["synara/src/app/utils/helper.spec.ts", "test"],
    ]);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("records method/listener candidates with explicit non-proven confidence", () => {
  const files = {
    "synara/src/client/lifecycle.ts": `
import { createClient, ClientEvent, IndexedDBStore, IndexedDBCryptoStore, MatrixClient } from 'matrix-js-sdk';
import { CryptoEvent } from 'matrix-js-sdk/lib/crypto-api';

export function boot(mx: MatrixClient) {
  const store = new IndexedDBStore({ indexedDB: globalThis.indexedDB });
  const cryptoStore = new IndexedDBCryptoStore(globalThis.indexedDB, 'crypto');
  void store;
  void cryptoStore;
  mx.on(ClientEvent.Sync, () => {});
  mx.on(CryptoEvent.VerificationRequestReceived, () => {});
  mx.startClient();
  mx.getRoom('!a:example');
  mx.sendEvent('!a:example', 'm.room.message', {});
  mx.sendTyping('!a:example', true, 5000);
  mx.sendReadReceipt({} as never);
  mx.getAccountData('m.direct');
  mx.setAccountData('m.direct', {});
  mx.getPushRules();
  mx.getCrypto();
  mx.searchRoomEvents({ body: { search_categories: { room_events: { search_term: 'x' } } } });
  mx.stopClient();
}
`,
  };
  const { root, fileList } = makeFixtureRoot(files);
  try {
    const inventory = buildInventory({ root, fileList });
    const file = inventory.files[0];
    assert.ok(file.categories.includes("sync_lifecycle"));
    assert.ok(file.categories.includes("crypto_verification_recovery"));
    assert.ok(file.categories.includes("indexeddb_matrix_stores"));
    assert.ok(file.categories.includes("event_emitters_listeners"));
    assert.ok(file.categories.includes("room_lists"));
    assert.ok(file.categories.includes("custom_raw_event_sends"));
    assert.ok(file.categories.includes("typing"));
    assert.ok(file.categories.includes("receipts"));
    assert.ok(file.categories.includes("account_data"));
    assert.ok(file.categories.includes("notifications_push_rules"));
    assert.ok(file.categories.includes("searches"));

    assert.ok(
      file.methodCandidates.some((r) => r.name === "startClient" && r.line > 0)
    );
    assert.ok(file.methodCandidates.some((r) => r.name === "sendEvent"));
    assert.ok(
      file.methodCandidates.every(
        (r) =>
          r.kind.includes("candidate") &&
          typeof r.confidence === "string" &&
          r.confidence.includes("not type-proven")
      )
    );
    assert.ok(
      file.listenerCandidates.some(
        (r) => r.method === "on" && r.event?.includes("ClientEvent.Sync")
      )
    );
    assert.ok(
      file.constructorCandidates.some((r) => r.name === "IndexedDBStore")
    );
    assert.ok(file.modelCoupling.includes("MatrixClient"));
    assert.ok(file.modelCoupling.includes("IndexedDBStore"));

    const md = renderMarkdown(inventory);
    assert.match(md, /not type-proven|candidates/i);
    assert.match(md, /Method-name candidates/i);
    assert.match(md, /not verified SDK API calls/i);
    assert.doesNotMatch(md, /## Verified SDK calls/i);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("classifies direct Matrix networking indicators with path and line", () => {
  const content = `
export async function versions(baseUrl: string) {
  const res = await fetch(\`\${baseUrl}/_matrix/client/versions\`);
  return res.json();
}
const MEDIA = '/_matrix/client/v1/media/download';
const PREFIX = 'matrix-js-sdk:crypto';
`;
  const findings = scanNetworkingIndicators(
    content,
    "synara/src/app/cs-api.ts"
  );
  assert.ok(findings.length >= 2);
  assert.ok(
    findings.every((f) => f.line > 0 && f.indicator.includes("/_matrix/"))
  );
  assert.ok(
    !findings.some((f) => f.indicator.includes("matrix-js-sdk:crypto"))
  );

  const files = {
    "synara/src/app/cs-api.ts": content,
    "synara/src/sw.ts": `
const MEDIA_PATHS = ['/_matrix/client/v1/media/download', '/_matrix/client/v1/media/thumbnail'];
self.addEventListener('fetch', (event) => { void event; });
`,
    "synara/src/app/utils/__tests__/net.test.ts": `
const u = 'https://example.org/_matrix/client/v3/sync';
`,
  };
  const { root, fileList } = makeFixtureRoot(files);
  try {
    const inventory = buildInventory({ root, fileList });
    assert.equal(inventory.summary.repositoryWide.productionImportFiles, 0);
    assert.ok(inventory.summary.repositoryWide.productionNetworkingFiles >= 2);
    assert.equal(inventory.summary.repositoryWide.testNetworkingFiles, 1);
    // production networking totals must not include test hits
    assert.equal(
      inventory.aggregates.production.networkingFindingCount,
      inventory.summary.repositoryWide.productionNetworkingFindings
    );
    assert.equal(
      inventory.aggregates.test.networkingFindingCount,
      inventory.summary.repositoryWide.testNetworkingFindings
    );
    for (const finding of inventory.aggregates.production.networking) {
      assert.ok(!path.isAbsolute(finding.path));
      assert.match(finding.path, /^synara\/src\//);
    }
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("output is deterministic and uses repository-relative POSIX paths only", () => {
  const files = {
    "synara/src/app/utils/b.ts": `import { Room } from 'matrix-js-sdk';\nexport type R = Room;\n`,
    "synara/src/app/utils/a.ts": `import type { MatrixClient } from 'matrix-js-sdk';\nexport type C = MatrixClient;\n`,
  };
  const { root, fileList } = makeFixtureRoot(files);
  try {
    const first = stableStringify(buildInventory({ root, fileList }));
    const second = stableStringify(buildInventory({ root, fileList }));
    assert.equal(first, second);

    assert.ok(!first.includes(root));
    assert.ok(!first.includes(tmpdir()));
    assert.ok(!/"\/Users\//.test(first));
    assert.ok(!/"\/home\//.test(first));

    const inventory = buildInventory({ root, fileList });
    assert.deepEqual(
      inventory.files.map((f) => f.path),
      ["synara/src/app/utils/a.ts", "synara/src/app/utils/b.ts"]
    );
    assert.equal(inventory.schemaVersion, 2);
    assert.ok(!("generatedAt" in inventory));
    assert.ok(!("timestamp" in inventory));

    const md = renderMarkdown(inventory);
    assert.match(md, /Generated report/);
    assert.ok(!md.includes(root));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("check mode detects stale artifacts and does not mutate them", () => {
  const files = {
    "synara/src/app/features/x/x.ts": `import { Room } from 'matrix-js-sdk';\nexport type R = Room;\n`,
  };
  const { root, fileList } = makeFixtureRoot(files);
  try {
    const inventory = buildInventory({ root, fileList });
    writeSnapshots(inventory, { root });

    const jsonRel = "docs/matrix-rust-sdk/desktop-sdk-usage.json";
    const mdRel = "docs/matrix-rust-sdk/desktop-sdk-usage.md";
    const jsonPath = path.join(root, jsonRel);
    const mdPath = path.join(root, mdRel);
    const beforeJson = readFileSync(jsonPath, "utf8");
    const beforeMd = readFileSync(mdPath, "utf8");

    const ok = checkSnapshots(inventory, { root });
    assert.equal(ok.ok, true);
    assert.deepEqual(ok.errors, []);

    writeFileSync(
      jsonPath,
      beforeJson.replace('"schemaVersion": 2', '"schemaVersion": 0'),
      "utf8"
    );
    const stale = checkSnapshots(inventory, { root });
    assert.equal(stale.ok, false);
    assert.ok(
      stale.errors.some(
        (e) => e.includes("Stale snapshot") && e.includes(jsonRel)
      )
    );

    const afterCheckJson = readFileSync(jsonPath, "utf8");
    const afterCheckMd = readFileSync(mdPath, "utf8");
    assert.notEqual(afterCheckJson, beforeJson);
    assert.equal(afterCheckMd, beforeMd);

    writeFileSync(jsonPath, beforeJson, "utf8");
    writeFileSync(mdPath, `${beforeMd}\n<!-- stale -->\n`, "utf8");
    const staleMd = checkSnapshots(inventory, { root });
    assert.equal(staleMd.ok, false);
    assert.ok(staleMd.errors.some((e) => e.includes(mdRel)));
    assert.equal(readFileSync(jsonPath, "utf8"), beforeJson);
    assert.match(readFileSync(mdPath, "utf8"), /stale/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("repository inventory includes tooling dynamic import and records the current deletion delta", () => {
  const inventory = buildInventory({ root: REPO_ROOT });
  const baseline = inventory.summary.desktopRuntimeBaseline;
  const rw = inventory.summary.repositoryWide;
  const productionImportDeclarations = inventory.files
    .filter((file) => file.role === "production")
    .reduce((count, file) => count + file.imports.length, 0);

  assert.equal(
    baseline.productionImportFiles,
    126,
    `expected 126 desktop runtime production import files, found ${baseline.productionImportFiles}`
  );
  assert.equal(productionImportDeclarations, 151);
  assert.equal(
    baseline.testImportFiles,
    10,
    `expected 10 desktop runtime test import files, found ${baseline.testImportFiles}`
  );
  assert.equal(baseline.buckets.feature, 47);
  assert.equal(baseline.buckets.hook, 35);
  assert.equal(baseline.buckets.component, 16);
  assert.equal(baseline.buckets.page, 7);
  assert.equal(baseline.buckets.utility, 9);
  assert.equal(baseline.buckets.state, 4);
  assert.equal(baseline.buckets.plugin, 6);
  assert.equal(baseline.buckets["client-lifecycle"], 2);
  assert.equal(baseline.buckets["media-boundary"] ?? 0, 0);
  assert.equal(baseline.buckets["shared-type"] ?? 0, 0);

  // Repository-wide includes tooling beyond synara/src
  assert.ok(rw.toolingImportFiles >= 1);
  assert.ok(rw.totalImportFiles >= baseline.totalImportFiles + 1);

  const harness = inventory.files.find(
    (f) => f.path === "synara/scripts/run-synapse-two-client-integration.mjs"
  );
  assert.ok(harness, "integration harness must be inventoried");
  assert.equal(harness.role, "tooling");
  assert.equal(harness.desktopRuntime, false);
  const dynamic = harness.imports.find(
    (imp) => imp.form === "dynamic" && imp.module === "matrix-js-sdk"
  );
  assert.ok(dynamic, "dynamic import of matrix-js-sdk must be recorded");
  assert.ok(dynamic.line >= 1);

  // Aggregates must be role-scoped
  assert.equal(
    inventory.aggregates.production.networkingFindingCount,
    rw.productionNetworkingFindings
  );
  assert.equal(
    inventory.aggregates.test.networkingFindingCount,
    rw.testNetworkingFindings
  );
  assert.ok(
    !inventory.aggregates.production.networking.some((n) =>
      n.path.includes("__tests__")
    )
  );

  const json = stableStringify(inventory);
  assert.ok(!json.includes(REPO_ROOT));
  assert.ok(!/"\/Users\//.test(json));
  assert.match(json, /run-synapse-two-client-integration\.mjs/);
  assert.match(json, /"form": "dynamic"/);
});

test("generator artifacts match external root Prettier CLI formatting", () => {
  const inventory = buildInventory({ root: REPO_ROOT });
  const jsonRel = "docs/matrix-rust-sdk/desktop-sdk-usage.json";
  const mdRel = "docs/matrix-rust-sdk/desktop-sdk-usage.md";
  const jsonPath = path.join(REPO_ROOT, jsonRel);
  const mdPath = path.join(REPO_ROOT, mdRel);

  const generatorJson = formatJsonArtifact(inventory, {
    root: REPO_ROOT,
    jsonPath,
  });
  const generatorMd = formatMarkdownArtifact(inventory, {
    root: REPO_ROOT,
    mdPath,
  });

  // Independent oracle: same binary/cwd as the required external CLI check.
  // Config resolution walks from the file path (null for docs/scripts => defaults).
  const cliJson = formatViaExternalPrettierCli(
    stableStringify(inventory),
    jsonRel
  );
  const cliMd = formatViaExternalPrettierCli(renderMarkdown(inventory), mdRel);

  assert.equal(
    generatorJson,
    cliJson,
    "generated JSON must match external Prettier CLI output"
  );
  assert.equal(
    generatorMd,
    cliMd,
    "generated Markdown must match external Prettier CLI output"
  );

  // Re-formatting CLI-clean text with the CLI is a no-op
  assert.equal(formatViaExternalPrettierCli(cliJson, jsonRel), cliJson);
  assert.equal(formatViaExternalPrettierCli(cliMd, mdRel), cliMd);

  // Explicitly reject the incorrect synara/.prettierrc force-path resolution
  const requireFromTest = createRequire(import.meta.url);
  const prettier = requireFromTest(
    path.join(REPO_ROOT, "synara/node_modules/prettier")
  );
  const pathConfig = prettier.resolveConfig.sync(jsonPath);
  const synaraForced = prettier.resolveConfig.sync(
    path.join(REPO_ROOT, "synara/package.json")
  );
  assert.equal(
    pathConfig,
    null,
    "docs artifacts must not inherit synara/.prettierrc from root CLI"
  );
  assert.ok(synaraForced, "synara config still exists for runtime sources");
  assert.notDeepEqual(
    pathConfig ?? {},
    synaraForced,
    "docs formatting rules must differ from forced synara config"
  );
});
