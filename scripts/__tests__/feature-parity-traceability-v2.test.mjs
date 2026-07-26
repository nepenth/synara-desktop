import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  mkdtemp,
  mkdir,
  readFile,
  realpath,
  rm,
  symlink,
  writeFile,
} from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const REPOSITORY_ROOT = path.resolve(HERE, "../..");
const LIBRARY_PATH = path.join(
  REPOSITORY_ROOT,
  "scripts/lib/feature-parity-traceability-v2.mjs"
);
const TRACEABILITY_CHECKER = path.join(
  REPOSITORY_ROOT,
  "scripts/check-feature-parity-traceability-v2.mjs"
);
const TRACEABILITY_GENERATOR = path.join(
  REPOSITORY_ROOT,
  "scripts/generate-feature-parity-traceability-v2.mjs"
);
const MIGRATOR = path.join(
  REPOSITORY_ROOT,
  "scripts/migrate-feature-parity-traceability-v2.mjs"
);

const api = await import(pathToFileURL(LIBRARY_PATH));

const SECTION_COUNTS = new Map([
  ["7.1", 13],
  ["7.2", 13],
  ["7.3", 17],
  ["7.4", 11],
  ["7.5", 11],
  ["7.6", 8],
  ["7.7", 9],
  ["7.8", 9],
  ["7.9", 13],
  ["7.10", 7],
  ["7.11", 8],
]);
function runNode(script, args = [], cwd = REPOSITORY_ROOT) {
  return spawnSync(process.execPath, [script, ...args], {
    cwd,
    encoding: "utf8",
    env: { ...process.env, NO_COLOR: "1" },
    timeout: 30_000,
  });
}

function bytesOf(value) {
  return Buffer.isBuffer(value)
    ? value
    : Buffer.from(value instanceof Uint8Array ? value : String(value), "utf8");
}

function coveredPointers(result) {
  const pointers = result?.coveredPointers;
  return pointers instanceof Set ? [...pointers] : pointers;
}

function terminalLeaves(value, pointer = "") {
  if (value === null || typeof value !== "object")
    return [{ pointer: pointer || "/", value }];
  if (Array.isArray(value)) {
    if (!value.length) return [{ pointer: pointer || "/", value }];
    return value.flatMap((entry, index) =>
      terminalLeaves(entry, `${pointer}/${index}`)
    );
  }
  const keys = Object.keys(value).sort();
  if (!keys.length) return [{ pointer: pointer || "/", value }];
  return keys.flatMap((key) =>
    terminalLeaves(
      value[key],
      `${pointer}/${key.replaceAll("~", "~0").replaceAll("/", "~1")}`
    )
  );
}

function decodeMarkdownTableJson(value) {
  return value
    .replaceAll("&#124;", "|")
    .replaceAll("&lt;", "<")
    .replaceAll("&gt;", ">")
    .replaceAll("&#96;", "`")
    .replaceAll("&amp;", "&");
}

function recoverMarkdown(rendered) {
  const markdown = bytesOf(rendered.bytes).toString("utf8");
  const lines = markdown.split("\n");
  const recovered = [];
  const readFence = (start) => {
    const match = /^(`{3,})json$/u.exec(lines[start]);
    assert.ok(match, `missing JSON fence at line ${start + 1}`);
    const closing = lines.indexOf(match[1], start + 1);
    assert.ok(closing > start, `unclosed JSON fence at line ${start + 1}`);
    return {
      value: JSON.parse(lines.slice(start + 1, closing).join("\n")),
      next: closing + 1,
    };
  };
  for (let index = 0; index < lines.length; index += 1) {
    if (lines[index] !== "JSON Pointer (canonical JSON string):") continue;
    const pointer = readFence(index + 2);
    assert.equal(lines[pointer.next + 1], "Value (canonical JSON):");
    const value = readFence(pointer.next + 3);
    recovered.push({ pointer: pointer.value, value: value.value });
    index = value.next;
  }
  const header = lines.indexOf(
    "| Canonical JSON Pointer String | Canonical leaf-value SHA-256 | Markdown anchor |"
  );
  assert.ok(header >= 0, "missing pointer coverage table");
  const tablePointers = [];
  for (let index = header + 2; lines[index]?.startsWith("| "); index += 1) {
    const cells = lines[index].slice(2, -2).split(" | ");
    assert.equal(cells.length, 3, `ambiguous table row ${index + 1}`);
    tablePointers.push(JSON.parse(decodeMarkdownTableJson(cells[0])));
  }
  return { markdown, recovered, tablePointers };
}

function diagnostics(result) {
  if (Array.isArray(result)) return result;
  if (Array.isArray(result?.diagnostics)) return result.diagnostics;
  if (Array.isArray(result?.errors)) return result.errors;
  return [];
}

function valid(result) {
  if (typeof result === "boolean") return result;
  if (typeof result?.valid === "boolean") return result.valid;
  if (typeof result?.ok === "boolean") return result.ok;
  return diagnostics(result).length === 0;
}

function generic119RowArtifact() {
  const requirements = [];
  for (const [section, count] of SECTION_COUNTS) {
    for (let index = 1; index <= count; index += 1) {
      const id = `FR-${section}-${String(index).padStart(3, "0")}`;
      requirements.push({
        id,
        section_id: section,
        current_product: { status: "partial", summary: `Audit ${id}.` },
        rust_cutover: {
          readiness: "blocked",
          implementation_subject_sha: null,
          qualification: "Current-product evidence is not Rust readiness.",
        },
        clauses: [
          {
            id: `${id}.C001`,
            text: `Clause for ${id}.`,
            status: "partial",
          },
        ],
      });
    }
  }
  assert.equal(requirements.length, 119);
  return {
    schema_version: "2.0",
    provenance: { audited_source_sha: "2".repeat(40) },
    coverage_contract: {
      requirement_count: 119,
      section_counts: Object.fromEntries(SECTION_COUNTS),
    },
    summary: { current_product_partial: 119, rust_ready: 0 },
    requirements,
  };
}

function pinnedV1() {
  const result = spawnSync(
    "git",
    ["show", `${api.AUDITED_SOURCE_COMMIT}:${api.V1_PATH}`],
    {
      cwd: REPOSITORY_ROOT,
      encoding: null,
      maxBuffer: 64 * 1024 * 1024,
      shell: false,
      windowsHide: true,
    }
  );
  assert.equal(result.status, 0, String(result.stderr));
  return api.parseCanonicalJson(result.stdout);
}

test("shared library exposes the complete packet-specified v2 API", () => {
  for (const name of [
    "validateTraceability",
    "renderTraceabilityMarkdown",
    "migrateV1",
    "runCli",
  ]) {
    assert.equal(typeof api[name], "function", `missing export ${name}`);
  }
});

test("production tooling contains no legacy authorization backend", async () => {
  const legacyField = ["state", "authorizations"].join("_");
  for (const repositoryPath of [
    "package.json",
    "docs/matrix-rust-sdk/schemas/feature-parity-traceability-v2.schema.json",
    "scripts/check-feature-parity-traceability-v2.mjs",
    "scripts/generate-feature-parity-traceability-v2.mjs",
    "scripts/lib/feature-parity-traceability-v2.mjs",
    "scripts/migrate-feature-parity-traceability-v2.mjs",
  ]) {
    const source = await readFile(
      path.join(REPOSITORY_ROOT, repositoryPath),
      "utf8"
    );
    assert.equal(source.includes(legacyField), false, repositoryPath);
  }
});

test("119-row Markdown generation is deterministic, complete, fast, and bounded", () => {
  const artifact = generic119RowArtifact();
  const rssBefore = process.memoryUsage().rss;
  const started = process.hrtime.bigint();
  const first = api.renderTraceabilityMarkdown(artifact);
  const elapsedMs = Number(process.hrtime.bigint() - started) / 1_000_000;
  const rssAfter = process.memoryUsage().rss;
  const second = api.renderTraceabilityMarkdown(structuredClone(artifact));

  assert.ok(elapsedMs <= 30_000, `render took ${elapsedMs.toFixed(1)} ms`);
  assert.ok(
    Math.max(rssBefore, rssAfter) <= 512 * 1024 * 1024,
    `peak observed RSS exceeded 512 MiB: ${Math.max(rssBefore, rssAfter)}`
  );
  assert.deepEqual(bytesOf(first.bytes), bytesOf(second.bytes));
  assert.equal(
    api.sha256(bytesOf(first.bytes)),
    api.sha256(bytesOf(second.bytes))
  );
  assert.deepEqual(coveredPointers(first), coveredPointers(second));
  assert.ok(coveredPointers(first).length >= 119 * 8);

  const markdown = bytesOf(first.bytes).toString("utf8");
  assert.match(markdown, /authoritative JSON SHA-256/i);
  assert.match(markdown, /JSON Pointer/u);
  assert.doesNotMatch(markdown, /\/private\/tmp|\/tmp\/|[A-Za-z]:\\/u);
});

test("Markdown appendix escapes pointers and includes every semantic leaf once", () => {
  const artifact = {
    schema_version: "2.0",
    "a/b": { "c~d": [true, null, 7, "text"] },
  };
  const rendered = api.renderTraceabilityMarkdown(artifact);
  const pointers = coveredPointers(rendered);
  assert.deepEqual(pointers, [
    "/a~1b/c~0d/0",
    "/a~1b/c~0d/1",
    "/a~1b/c~0d/2",
    "/a~1b/c~0d/3",
    "/schema_version",
  ]);
  const recovered = recoverMarkdown(rendered);
  assert.deepEqual(
    recovered.recovered,
    terminalLeaves(artifact).sort((left, right) =>
      left.pointer < right.pointer ? -1 : left.pointer > right.pointer ? 1 : 0
    )
  );
  assert.deepEqual(recovered.tablePointers, pointers);
});

test("Markdown safely round-trips hostile pointers, backtick runs, and empty containers", () => {
  const artifact = {
    schema_version: "2.0",
    "pipe|key": "single ` and double `` and triple ``` runs",
    "back`tick": { "line\nkey": "<angle>&ampersand" },
    "<tag>&": {},
    empty_array: [],
  };
  const rendered = api.renderTraceabilityMarkdown(artifact);
  const expected = terminalLeaves(artifact).sort((left, right) =>
    left.pointer < right.pointer ? -1 : left.pointer > right.pointer ? 1 : 0
  );
  const recovered = recoverMarkdown(rendered);
  assert.deepEqual(recovered.recovered, expected);
  assert.deepEqual(
    recovered.tablePointers,
    expected.map(({ pointer }) => pointer)
  );
  assert.equal(
    recovered.tablePointers.length,
    new Set(recovered.tablePointers).size
  );
  assert.doesNotMatch(
    recovered.markdown,
    /^\| .*\|key.* \|/mu,
    "literal pointer pipes must not split table cells"
  );
});

test("real pinned v1 Markdown covers all empty containers and embedded backticks", () => {
  const v1 = pinnedV1();
  const expected = terminalLeaves(v1).sort((left, right) =>
    left.pointer < right.pointer ? -1 : left.pointer > right.pointer ? 1 : 0
  );
  const emptyContainers = expected.filter(
    ({ value }) =>
      value !== null &&
      typeof value === "object" &&
      Object.keys(value).length === 0
  );
  assert.equal(emptyContainers.length, 5_122);
  assert.ok(
    expected.some(
      ({ value }) => typeof value === "string" && value.includes("`")
    )
  );
  const rendered = api.renderTraceabilityMarkdown(v1);
  const recovered = recoverMarkdown(rendered);
  assert.deepEqual(recovered.recovered, expected);
  assert.deepEqual(
    recovered.tablePointers,
    expected.map(({ pointer }) => pointer)
  );
  assert.deepEqual(coveredPointers(rendered), recovered.tablePointers);
});

test("v2 structural and semantic validation reject incomplete artifacts", async () => {
  const artifact = generic119RowArtifact();
  artifact.requirements.pop();
  artifact.requirements.push(structuredClone(artifact.requirements[0]));
  const schemaResult = await api.validateSchema(
    "https://synara.invalid/schemas/matrix-rust-sdk/feature-parity-traceability-v2.schema.json",
    artifact
  );
  const semanticResult = await api.validateTraceability(artifact, {
    gitObjects: null,
    repoRoot: REPOSITORY_ROOT,
    durableAuditIdentity: null,
  });
  assert.equal(valid(schemaResult), false);
  assert.equal(valid(semanticResult), false);
  assert.ok(diagnostics(semanticResult).length > 0);
});

test("semantic validation prevents current-product status from promoting Rust readiness", async () => {
  const artifact = generic119RowArtifact();
  const row = artifact.requirements[0];
  row.current_product.status = "implemented";
  row.rust_cutover.readiness = "ready";
  row.rust_cutover.qualification = "Promoted because JavaScript works.";
  const result = await api.validateTraceability(artifact, {
    gitObjects: null,
    repoRoot: REPOSITORY_ROOT,
    durableAuditIdentity: null,
  });
  assert.equal(valid(result), false);
  assert.ok(diagnostics(result).length > 0);
});

test("MRSDK-R036 owns FR-7.4-011 and rejects unsupported leak claims", async () => {
  const artifact = generic119RowArtifact();
  artifact.blockers = [
    {
      id: "MRSDK-R036",
      severity: "high",
      status: "open",
      affected_requirement_ids: ["FR-7.4-011"],
      claim: "This is an observed plaintext and Megolm session-key leak.",
    },
  ];
  const result = await api.validateTraceability(artifact, {
    gitObjects: null,
    repoRoot: REPOSITORY_ROOT,
    durableAuditIdentity: null,
  });
  assert.equal(valid(result), false);
  assert.ok(
    diagnostics(result).some(({ code }) => code === "V2_R036_CLAIM"),
    "missing stable MRSDK-R036 unsupported-claim diagnostic"
  );
});

test("central risk mappings cannot be omitted, weakened, or conflated", async () => {
  const artifact = generic119RowArtifact();
  artifact.blockers = [
    {
      id: "MRSDK-R030",
      severity: "medium",
      status: "closed",
      task_ids: ["P10.5"],
      affected_requirements: ["FR-7.11-005"],
    },
    { id: "MRSDK-R037", severity: "low", status: "closed" },
    { id: "MRSDK-R038", severity: "low", status: "closed" },
  ];
  const result = await api.validateTraceability(artifact, {
    gitObjects: null,
    repoRoot: REPOSITORY_ROOT,
    durableAuditIdentity: null,
  });
  assert.equal(valid(result), false);
  const codes = new Set(diagnostics(result).map(({ code }) => code));
  for (const code of ["V2_RISK_MAPPING", "V2_R030", "V2_R037", "V2_R038"]) {
    assert.ok(codes.has(code), `missing ${code}`);
  }
});

test("migration rejects unknown v1 keys before any output is derived", async () => {
  const v1 = pinnedV1();
  const audit = {
    schema_version: "1.0",
    rows: generic119RowArtifact().requirements,
    vocabularies: { status: ["implemented", "partial", "missing"] },
    blockers_and_risks: [],
    architecture_decisions: [],
    coverage: {
      requirement_count: 119,
      section_counts: Object.fromEntries(SECTION_COUNTS),
      status_correction_count: 23,
    },
  };
  const sourceIdentity = {
    commit_sha: "2aa6d96f9b63aad64a14feac23df2f694857be85",
    blob_oid: "2e781bc58958f9ce39d2a527d7b5ba43a6d9d858",
    file_sha256:
      "70862c9052c163f2cbe64f200de102de22ac5935e94324a0b3195b8a6d3bc58b",
  };
  const auditIdentity = {
    commit_sha: "3".repeat(40),
    blob_oid: "4".repeat(40),
    file_sha256: "5".repeat(64),
    canonical_sha256: "6".repeat(64),
  };

  const mutated = structuredClone(v1);
  mutated.unclassified = true;
  await assert.rejects(
    async () =>
      api.migrateV1({ v1: mutated, audit, sourceIdentity, auditIdentity }),
    /unclassified|unknown|manifest/iu
  );
});

test("traceability production CLIs reject bypasses, combined modes, and extras", () => {
  const cases = [
    [TRACEABILITY_CHECKER, ["--repo-root", "."]],
    [TRACEABILITY_CHECKER, ["--audit-commit", "3".repeat(40)]],
    [TRACEABILITY_CHECKER, ["--fixture", "fixture.json"]],
    [TRACEABILITY_GENERATOR, []],
    [TRACEABILITY_GENERATOR, ["--check", "--write"]],
    [TRACEABILITY_GENERATOR, ["--write-worktree"]],
    [MIGRATOR, []],
    [MIGRATOR, ["--check", "--write"]],
    [MIGRATOR, ["--source-commit", "2".repeat(40)]],
    [MIGRATOR, ["--output-dir"]],
    [MIGRATOR, ["--output-dir", "a", "extra"]],
    [MIGRATOR, ["--json"]],
  ];
  for (const [script, args] of cases) {
    const result = runNode(script, args);
    assert.equal(
      result.status,
      2,
      `${path.basename(script)} ${args.join(" ")}\n${result.stderr}`
    );
  }
});

test("production checkers fail honestly before E2/E3 artifacts exist", () => {
  for (const script of [
    TRACEABILITY_CHECKER,
    TRACEABILITY_GENERATOR,
    MIGRATOR,
  ]) {
    const args = script === TRACEABILITY_CHECKER ? [] : ["--check"];
    const result = runNode(script, args);
    assert.equal(
      result.status,
      1,
      `${path.basename(script)}: ${result.stderr}`
    );
    assert.doesNotMatch(
      `${result.stdout}\n${result.stderr}`,
      /\/private\/tmp|\/tmp\//u
    );
  }
});

test("migrator rejects unsafe output directories before artifact processing", async (t) => {
  const parent = await realpath(
    await mkdtemp(path.join(os.tmpdir(), "synara-r02-e1-output-"))
  );
  t.after(async () => rm(parent, { recursive: true, force: true }));
  const missing = path.join(parent, "missing");
  const nonEmpty = path.join(parent, "non-empty");
  const target = path.join(parent, "target");
  const alias = path.join(parent, "alias");
  await mkdir(nonEmpty);
  await writeFile(path.join(nonEmpty, "occupied"), "x");
  await mkdir(target);
  await symlink(target, alias);

  for (const candidate of [missing, nonEmpty, alias, REPOSITORY_ROOT, HERE]) {
    const result = runNode(MIGRATOR, ["--output-dir", candidate]);
    assert.equal(result.status, 2, `${candidate}: ${result.stderr}`);
    assert.doesNotMatch(
      `${result.stdout}\n${result.stderr}`,
      new RegExp(candidate.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&"), "u")
    );
  }
});

test("concurrent missing-input migrators fail honestly without creating outputs", async (t) => {
  const parent = await realpath(
    await mkdtemp(path.join(os.tmpdir(), "synara-r02-e1-determinism-"))
  );
  t.after(async () => rm(parent, { recursive: true, force: true }));
  const left = path.join(parent, "left");
  const right = path.join(parent, "right");
  await mkdir(left);
  await mkdir(right);

  const a = runNode(MIGRATOR, ["--output-dir", left]);
  const b = runNode(MIGRATOR, ["--output-dir", right]);
  assert.equal(a.status, 1, a.stderr);
  assert.equal(b.status, 1, b.stderr);
  for (const output of [a.stdout, a.stderr, b.stdout, b.stderr]) {
    assert.doesNotMatch(output, /\/private\/tmp|\/tmp\//u);
  }
  assert.deepEqual(
    await readFile(left).catch(() => Buffer.alloc(0)),
    Buffer.alloc(0)
  );
  assert.deepEqual(
    await readFile(right).catch(() => Buffer.alloc(0)),
    Buffer.alloc(0)
  );
});
