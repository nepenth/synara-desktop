import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  realpathSync,
  renameSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  auditSupportedSchema,
  parseJsonStrict,
  validateSchemaInstance,
} from "./matrix-rust-governance.mjs";

export const AUDITED_SOURCE_COMMIT = "2aa6d96f9b63aad64a14feac23df2f694857be85";
export const AUDITED_PLAN_PATH =
  "docs/matrix-rust-sdk-full-replacement-plan.md";
export const AUDITED_PLAN_BLOB = "e949fc5c3e64e5a93a675dc96c3a7e37db7a5152";
export const AUDITED_PLAN_SHA256 =
  "a6e47c0ccaba9ea67ba82e6ddb981443b963ad11f6895ad232023c0494f66b5f";
export const CURRENT_PLAN_BLOB = "506435c6ef461c091e66c8c3fbe2efcaa888bdaa";
export const CURRENT_PLAN_SHA256 =
  "5056c9eafb338e395ddc4e982c5ae0223edb62529f3a5e0be991196034804e0f";
export const RISK_REGISTER_COMMIT = "c358502cf99ade2b81abf85ad954e21bf228dc33";
export const RISK_REGISTER_PATH =
  "docs/matrix-rust-sdk/security-risk-register.json";
export const RISK_REGISTER_BLOB = "2737ba78be3ad2a41a5952b9acc187949bf831a9";
export const RISK_REGISTER_SHA256 =
  "c07b7540b931e244e43f0dc750f52ed564393dc71f2543f1692aad916eb217f6";
export const V1_PATH = "docs/matrix-rust-sdk/feature-parity-traceability.json";
export const V1_MARKDOWN_PATH =
  "docs/matrix-rust-sdk/feature-parity-traceability.md";
export const V1_BLOB = "2e781bc58958f9ce39d2a527d7b5ba43a6d9d858";
export const V1_SHA256 =
  "70862c9052c163f2cbe64f200de102de22ac5935e94324a0b3195b8a6d3bc58b";
export const V1_CANONICAL_SHA256 =
  "e0ee1967aa6583d15027c5d31c88b98c769dbe803ed1ec7a29995549b08c8a58";
const V1_SUMMARY_CANONICAL_SHA256 =
  "1123b02b81bdc81895dee37c64ed7ba747c4d1e46358fafd3c2c986719d8d965";
const V1_REQUIREMENTS_CANONICAL_SHA256 =
  "c398264e8fb5b66ab6b33082cde683b009f12dc10e16217c61e204e8c62ecd48";
const V1_PROVENANCE_CANONICAL_SHA256 =
  "25fc29514339fd8dbcbb97db3f92ba7bc7bbb186046a7af2fc6fe9884935af5e";
const V1_VOCABULARIES_CANONICAL_SHA256 =
  "3f07389dbc9140d31487671f7b69268421291d043efb377910a69695bf591ade";
export const AUDIT_PATH =
  "docs/matrix-rust-sdk/reviews/r0.2-e-audit-normalization-119.json";
export const AUDIT_MARKDOWN_PATH =
  "docs/matrix-rust-sdk/reviews/r0.2-e-audit-normalization-119.md";
export const AUDIT_SCHEMA_PATH =
  "docs/matrix-rust-sdk/schemas/feature-parity-audit-normalization.schema.json";
export const V2_SCHEMA_PATH =
  "docs/matrix-rust-sdk/schemas/feature-parity-traceability-v2.schema.json";

export const EXPECTED_SECTION_COUNTS = Object.freeze({
  7.1: 13,
  7.2: 13,
  7.3: 17,
  7.4: 11,
  7.5: 11,
  7.6: 8,
  7.7: 9,
  7.8: 9,
  7.9: 13,
  "7.10": 7,
  7.11: 8,
});
export const EXPECTED_REQUIREMENT_COUNT = 119;
export const EXPECTED_STATUS_CORRECTIONS = 23;
export const EXPECTED_ARCHITECTURE_DECISIONS = 26;
export const EXPECTED_REQUIREMENT_IDS = Object.freeze(
  Object.entries(EXPECTED_SECTION_COUNTS).flatMap(([section, count]) =>
    Array.from(
      { length: count },
      (_, index) => `FR-${section}-${String(index + 1).padStart(3, "0")}`
    )
  )
);
const EXPECTED_STATUS_CORRECTION_TRANSITIONS = new Map(
  [
    "FR-7.1-004",
    "FR-7.1-010",
    "FR-7.2-003",
    "FR-7.2-008",
    "FR-7.2-011",
    "FR-7.3-007",
    "FR-7.3-009",
    "FR-7.3-011",
    "FR-7.4-006",
    "FR-7.4-007",
    "FR-7.4-009",
    "FR-7.4-011",
    "FR-7.6-002",
    "FR-7.6-006",
    "FR-7.8-005",
    "FR-7.8-006",
    "FR-7.8-007",
    "FR-7.9-004",
    "FR-7.10-003",
  ].map((id) => [id, ["implemented", "partial"]])
);
EXPECTED_STATUS_CORRECTION_TRANSITIONS.set("FR-7.1-009", [
  "implemented",
  "not_exposed",
]);
EXPECTED_STATUS_CORRECTION_TRANSITIONS.set("FR-7.5-010", [
  "implemented",
  "missing",
]);
EXPECTED_STATUS_CORRECTION_TRANSITIONS.set("FR-7.6-008", [
  "partial",
  "implemented",
]);
EXPECTED_STATUS_CORRECTION_TRANSITIONS.set("FR-7.11-004", [
  "partial",
  "implemented",
]);
const SUBTOTAL_SECTIONS = new Set(["7.3", "7.4", "7.5", "7.6", "7.7"]);
export const EXPECTED_73_77_SUBTOTAL = [
  ...EXPECTED_STATUS_CORRECTION_TRANSITIONS,
].filter(
  ([requirementId, [recorded, audited]]) =>
    recorded !== audited &&
    SUBTOTAL_SECTIONS.has(/^FR-(7\.(?:[1-9]|1[01]))-/u.exec(requirementId)?.[1])
).length;
export const EXPECTED_73_77_NON_STATUS_CORRECTIONS = Object.freeze(
  ["FR-7.5-004", "FR-7.5-009", "FR-7.6-003", "FR-7.7-003", "FR-7.7-009"].map(
    (requirement_id) =>
      Object.freeze({
        requirement_id,
        classification: "evidence_or_test_ledger",
      })
  )
);
export const EXPECTED_73_77_IDENTIFIED_CORRECTION_COUNT =
  EXPECTED_73_77_SUBTOTAL + EXPECTED_73_77_NON_STATUS_CORRECTIONS.length;
export const RISK_REGISTER_CANONICAL_SHA256 =
  "a7e6457ff8a33d70a49f5289b245aebb133c4e803f2e19d5a13bc4872a10bda6";
export const EXPECTED_CENTRAL_RISK_CONTRACT = Object.freeze({
  "MRSDK-R024": Object.freeze({
    severity: "high",
    status: "open",
    owner_task_ids: Object.freeze(["P3.5", "P7.7", "P11.1", "P11.6", "P14.1"]),
    threat_ids: Object.freeze(["TM-T01", "TM-T09"]),
    boundary_ids: Object.freeze([]),
    closure_criteria: Object.freeze([
      "After cutover, Rust owns all Matrix session material and service-worker auth and credential-bearing WebView localStorage are deleted. Secure native restore is specified and tested separately from fallback and platform durability: every supported platform either restores from an approved durable native store or fails explicitly into the approved reauthentication/recovery state, never silently stores a token in the WebView; unsupported Windows persistence is explicit. Packaged tests cover clean quit/relaunch, crash/relaunch, OS restart, and native-store unavailable/locked cases, and scans/runtime tests prove no credential remains in WebView/service-worker storage or crosses IPC.",
    ]),
  }),
  "MRSDK-R027": Object.freeze({
    severity: "critical",
    status: "open",
    owner_task_ids: Object.freeze(["R0.5"]),
    threat_ids: Object.freeze(["TM-T02", "TM-T07"]),
    boundary_ids: Object.freeze([]),
    closure_criteria: Object.freeze([
      "Every filesystem, keyring, and session-state deletion is verified and fail-closed. Native credential-store or Matrix IndexedDB deletion failure produces a specified user-visible recoverable state and retry/support path, emits no completed result, causes no false completion or reload, preserves enough state for a safe idempotent retry, and never triggers automatic wipe. Injected failures for each deletion independently and together prove the behavior, and success is reported only after both removals are verified.",
    ]),
  }),
  "MRSDK-R030": Object.freeze({
    severity: "high",
    status: "open",
    owner_task_ids: Object.freeze([
      "P10.2",
      "P10.3",
      "P10.6",
      "P10.7",
      "P11.7",
    ]),
    threat_ids: Object.freeze(["TM-T10"]),
    boundary_ids: Object.freeze([]),
    closure_criteria: Object.freeze([
      "Every Matrix listener uses a stable callback or stored disposer identity, and call/widget teardown is exact and idempotent. Tests prove listener counts return to baseline and no event is delivered after disposal across hangup or replacement, the approved room-navigation lifecycle, logout, and window close. Explicit origin allowlists, least-privilege CSP, capability scope, secret handling, and experimental-feature acceptance must still pass packaged security tests.",
    ]),
  }),
  "MRSDK-R036": Object.freeze({
    severity: "medium",
    status: "open",
    owner_task_ids: Object.freeze(["P6.1", "P7.4", "P13.5"]),
    threat_ids: Object.freeze(["TM-T13"]),
    boundary_ids: Object.freeze([]),
    closure_criteria: Object.freeze([
      "Forwarding rejects m.room.encrypted events without successfully decrypted clear content, rejects m.bad.encrypted and other decryption-failure placeholders, allowlists approved clear event types and fields, and fails closed for mixed or multi-select batches. Tests cover successful, raw/in-progress, and failed decryption into encrypted and unencrypted target rooms and prove wire algorithm, ciphertext, sender_key, device_id, and session_id fields are never republished. Standards-required encrypted-attachment file descriptors from successfully decrypted clear content remain preserved, subject to the existing unencrypted-target confirmation; closure does not require attachment re-upload/re-encryption and does not assert a plaintext, Megolm-key, or unintended attachment-key leak.",
    ]),
  }),
  "MRSDK-R037": Object.freeze({
    severity: "high",
    status: "open",
    owner_task_ids: Object.freeze(["P3.2", "P3.3", "P13.5"]),
    threat_ids: Object.freeze(["TM-T01"]),
    boundary_ids: Object.freeze([]),
    closure_criteria: Object.freeze([
      "Consume each login token once and scrub it from URL and history before any asynchronous work or UI rendering. The token never persists in referrer-visible state, browser history, logs, diagnostics, or storage; callback origin is strictly validated; replay and malformed callbacks fail closed. The target OAuth/OIDC callback validates state, nonce, and PKCE according to the selected flow. Hash-router and history-router tests plus hostile callback fixtures prove one-time consumption, immediate scrub, strict origin, integrity validation, and safe error handling.",
    ]),
  }),
  "MRSDK-R038": Object.freeze({
    severity: "medium",
    status: "open",
    owner_task_ids: Object.freeze(["P9.5", "P13.5"]),
    threat_ids: Object.freeze(["TM-T17"]),
    boundary_ids: Object.freeze(["B09"]),
    closure_criteria: Object.freeze([
      "Preserve a validated privacy field end to end across the notification DTO, IPC command, native adapter, and OS presentation. An approved policy redacts title/body/action context for encrypted rooms, private mode, and lock-screen presentation; sensitive approval command previews never enter OS notifications. Packaged platform tests and negative IPC/OS-notification fixtures prove privacy-field preservation, redaction behavior, bounded metadata, and absence of command-preview or event-content leakage.",
    ]),
  }),
});

export const SOURCE_INPUTS = Object.freeze(
  [
    [
      "R0.2-E-INPUT-7.1-7.2",
      "7.1-7.2",
      26,
      "f651b9ae4ba42c4cc9ceb4fca02fd25fb87e636216e16f21add9d98c5c395419",
    ],
    [
      "R0.2-E-INPUT-7.3-7.4",
      "7.3-7.4",
      28,
      "97e355194641120f5f6416c71c7331e7dc212fce884ed5795850fb4e978bc322",
    ],
    [
      "R0.2-E-INPUT-7.5-7.7",
      "7.5-7.7",
      28,
      "8a0668d1262b0222b2301ac57a01cb42a1b30f810e3f42f7aa09b004313e9bff",
    ],
    [
      "R0.2-E-INPUT-7.8-7.9",
      "7.8-7.9",
      22,
      "42be948073de22512c9b349b1d2d3f2b9f9a27b82f4490fb33b3797e92000555",
    ],
    [
      "R0.2-E-INPUT-7.10-7.11",
      "7.10-7.11",
      15,
      "c973534ac74a0069be2c235fdfea797f76462f744bc994a699a8e59d4cd47fca",
    ],
  ].map(([logical_id, section_range, row_count, sha256]) =>
    Object.freeze({
      logical_id,
      section_range,
      row_count,
      sha256,
      adapter_version: "r0.2-e1-v1",
      ingestion_notes: Object.freeze([
        "Logical durable provenance; no scratch path.",
      ]),
    })
  )
);

export const PRESERVED_V1_KEYS = Object.freeze([
  "task_id",
  "title",
  "p0_1_fidelity",
  "file_coverage_ledger",
  "direct_matrix_networking_ledger",
  "deep_import_ledger",
  "custom_contract_ledger",
  "dead_path_proofs",
  "implementation_task_catalog",
  "automated_test_catalog",
  "manual_acceptance_catalog",
  "risks_and_blockers",
  "correction_passes",
  "cx_extras",
  "cx_extras_note",
  "evidence_limitations",
  "invariants",
  "phase1_plus_handoff",
  "prohibited_claims_guard",
]);
const PRESERVED_V1_SHA256 = Object.freeze({
  task_id: "0bc389587a556007272a6f16ebe3af0e4cd5a3b911465647fc912bcbe533f537",
  title: "fed1dcdf03b5a08fea27eb0b60395a09a351113a025ed7a2cd7cbaff0281bc9c",
  p0_1_fidelity:
    "028f5ca4ce149aed2881e02b8103788f1ccd5f14cef636cc28e3bcd7e9dc224e",
  file_coverage_ledger:
    "0cf67e2765445768e1731ff265de45c551809b9d84fce508b8c67f29b91e7f54",
  direct_matrix_networking_ledger:
    "f57973d28e269041ddd225dfe9a18896f3a25ee852fc75a70a4cd28ee3dcc23b",
  deep_import_ledger:
    "ca43985390c20b6c3ccfadd6d06d90d18b6f13a3afbb881beb0be7a65794c777",
  custom_contract_ledger:
    "ee6702e049a27b3ea2a2f613d534cf19b654360ac6234ba2388bb5c50521c081",
  dead_path_proofs:
    "4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945",
  implementation_task_catalog:
    "f46c29aebca4643b992ef547e7a6cef8f989d5a1b04358c7cb9f30654eb4009a",
  automated_test_catalog:
    "f38e54b873cbb37e2b441252cd3f8e05f89fb6746e9f1ec845f2e4b0751e106c",
  manual_acceptance_catalog:
    "8daa3990f3d46e568faf7d8579c5ff9973fd55225f395785afdc9e68d3469399",
  risks_and_blockers:
    "3ab01e02b8dd60ef0bb763ca48a908f348dacbc69b51b386104eedd2aa15ecb6",
  correction_passes:
    "867cc462f96d7502f053107980025a1aea4217bdd55d8228d056d3c1cee86864",
  cx_extras: "4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945",
  cx_extras_note:
    "1bd3f5f8b816d31a60f7991821306e746059acf5e3189731a061896141693474",
  evidence_limitations:
    "e477f8e129f1146656fab7d56f0cfa63baaf4291a2d2dfe0d3d6dbfd53460fcb",
  invariants:
    "6b43a2c40c0082b5e6ba1c1b85425bb22e3444db102585445f8fce8049797292",
  phase1_plus_handoff:
    "420ca2442a87cb7453ac9cb28c3067c73cede019ae5f64905167d8eb6d1142e0",
  prohibited_claims_guard:
    "9cd6e11cdf44c7f62678994433b65aae3bc11e2bb9262c05d0445a582dee07a9",
});
export const TRANSFORMED_V1_KEYS = Object.freeze([
  "schema_version",
  "provenance",
  "summary",
  "vocabularies",
  "requirements",
]);
export const ADDED_V2_KEYS = Object.freeze([
  "migration_provenance",
  "coverage_contract",
  "source_audit",
  "lifecycle",
  "blockers",
  "architecture_decisions",
  "audit_reports",
  "validation_reports",
]);

const SAFE_SHA = /^[0-9a-f]{40}$/u;
const SAFE_OID = /^[0-9a-f]{40,64}$/u;
const SAFE_SHA256 = /^[0-9a-f]{64}$/u;
const SAFE_REQUIREMENT = /^FR-7\.(?:[1-9]|10|11)-\d{3}$/u;
const PROHIBITED_SERIALIZED_PATH =
  /(?:\/private\/tmp\/|\/tmp\/|(?:^|["'\s])\/(?:Users|home)\/|(?:^|["'\s])[A-Za-z]:[\\/]|\\\\|(?:^|\/)\.\.(?:\/|$)|agent(?:-|_)?id|worktree|transcript)/iu;

function safeGitEnvironment() {
  const environment = { ...process.env };
  for (const key of Object.keys(environment)) {
    if (key.startsWith("GIT_")) delete environment[key];
  }
  environment.GIT_OPTIONAL_LOCKS = "0";
  environment.GIT_CONFIG_NOSYSTEM = "1";
  environment.GIT_CONFIG_GLOBAL = "/dev/null";
  return environment;
}

function gitProcessOptions(cwd) {
  return {
    cwd,
    env: safeGitEnvironment(),
    shell: false,
    windowsHide: true,
  };
}

function issue(code, pointer, message, artifact = "") {
  return { artifact, code, pointer, message };
}

function internalError(message) {
  const error = new Error(message);
  error.code = "INTERNAL";
  return error;
}

export function sortDiagnostics(diagnostics) {
  return [...diagnostics].sort((a, b) => {
    const left = [a.artifact ?? "", a.pointer ?? "", a.code ?? ""].join("\0");
    const right = [b.artifact ?? "", b.pointer ?? "", b.code ?? ""].join("\0");
    return left < right ? -1 : left > right ? 1 : 0;
  });
}

export function formatDiagnostics(diagnostics) {
  return sortDiagnostics(diagnostics)
    .map(
      ({ artifact, code, pointer, message }) =>
        `${artifact ? `${artifact}:` : ""}${pointer} ${code}: ${message}`
    )
    .join("\n");
}

export function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function assertUnicodeString(value, location) {
  for (let index = 0; index < value.length; index += 1) {
    const code = value.charCodeAt(index);
    if (code >= 0xd800 && code <= 0xdbff) {
      const next = value.charCodeAt(index + 1);
      if (!(next >= 0xdc00 && next <= 0xdfff))
        throw new TypeError(`Unpaired high surrogate at ${location}.`);
      index += 1;
    } else if (code >= 0xdc00 && code <= 0xdfff) {
      throw new TypeError(`Unpaired low surrogate at ${location}.`);
    }
  }
}

// Canonical values are strictly JSON data. Arrays cannot contain holes or
// custom properties and objects cannot contain accessors, so validation never
// executes caller-controlled getters.
export function assertCanonicalValue(value, location = "$") {
  const active = new WeakSet();
  const visit = (entry, entryLocation) => {
    if (entry === null || typeof entry === "boolean") return;
    if (typeof entry === "string") {
      assertUnicodeString(entry, entryLocation);
      return;
    }
    if (typeof entry === "number") {
      if (!Number.isSafeInteger(entry) || Object.is(entry, -0))
        throw new TypeError(
          `Only safe non-negative-zero integers are allowed at ${entryLocation}.`
        );
      return;
    }
    if (!entry || typeof entry !== "object")
      throw new TypeError(`Unsupported JSON value at ${entryLocation}.`);
    if (active.has(entry))
      throw new TypeError(`Cyclic JSON value at ${entryLocation}.`);
    active.add(entry);
    try {
      if (Array.isArray(entry)) {
        const ownKeys = Reflect.ownKeys(entry);
        if (ownKeys.some((key) => typeof key === "symbol"))
          throw new TypeError(
            `Symbol-keyed array property at ${entryLocation}.`
          );
        for (let index = 0; index < entry.length; index += 1) {
          const key = String(index);
          const descriptor = Object.getOwnPropertyDescriptor(entry, key);
          if (!descriptor)
            throw new TypeError(
              `Sparse array hole at ${entryLocation}/${index}.`
            );
          if (!("value" in descriptor) || !descriptor.enumerable)
            throw new TypeError(
              `Accessor or hidden array value at ${entryLocation}/${index}.`
            );
          visit(descriptor.value, `${entryLocation}/${index}`);
        }
        const permitted = new Set([
          "length",
          ...Array.from({ length: entry.length }, (_, index) => String(index)),
        ]);
        if (
          ownKeys.some((key) => typeof key !== "string" || !permitted.has(key))
        )
          throw new TypeError(
            `Unsupported array property at ${entryLocation}.`
          );
        return;
      }
      if (Object.getPrototypeOf(entry) !== Object.prototype)
        throw new TypeError(`Unsupported JSON value at ${entryLocation}.`);
      for (const key of Reflect.ownKeys(entry)) {
        if (typeof key !== "string")
          throw new TypeError(
            `Symbol-keyed object property at ${entryLocation}.`
          );
        assertUnicodeString(key, `${entryLocation}/<key>`);
        const descriptor = Object.getOwnPropertyDescriptor(entry, key);
        if (!descriptor || !("value" in descriptor) || !descriptor.enumerable)
          throw new TypeError(
            `Accessor or hidden object value at ${entryLocation}/${escapePointer(
              key
            )}.`
          );
        if (descriptor.value === undefined)
          throw new TypeError(`Undefined value at ${entryLocation}/${key}.`);
        visit(descriptor.value, `${entryLocation}/${escapePointer(key)}`);
      }
    } finally {
      active.delete(entry);
    }
  };
  visit(value, location);
}

function canonicalNode(value) {
  if (value === null || typeof value !== "object") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(canonicalNode).join(",")}]`;
  return `{${Object.keys(value)
    .sort()
    .map((key) => `${JSON.stringify(key)}:${canonicalNode(value[key])}`)
    .join(",")}}`;
}

export function canonicalize(value) {
  assertCanonicalValue(value);
  return Buffer.from(canonicalNode(value), "utf8");
}

export function parseCanonicalJson(source) {
  if (Buffer.isBuffer(source)) {
    if (source.subarray(0, 3).equals(Buffer.from([0xef, 0xbb, 0xbf])))
      throw new SyntaxError("UTF-8 BOM is forbidden.");
    source = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(
      source
    );
  }
  if (typeof source !== "string")
    throw new TypeError("JSON input must be UTF-8 text.");
  if (source.charCodeAt(0) === 0xfeff)
    throw new SyntaxError("UTF-8 BOM is forbidden.");
  const value = parseJsonStrict(source);
  assertCanonicalValue(value);
  return value;
}

export function prettyJson(value) {
  assertCanonicalValue(value);
  const sorted = (entry) => {
    if (entry === null || typeof entry !== "object") return entry;
    if (Array.isArray(entry)) return entry.map(sorted);
    return Object.fromEntries(
      Object.keys(entry)
        .sort()
        .map((key) => [key, sorted(entry[key])])
    );
  };
  return `${JSON.stringify(sorted(value), null, 2)}\n`;
}

export function escapePointer(value) {
  return String(value).replaceAll("~", "~0").replaceAll("/", "~1");
}

export function unescapePointer(value) {
  if (/~(?:[^01]|$)/u.test(value))
    throw new TypeError("Invalid RFC 6901 escape.");
  return value.replaceAll("~1", "/").replaceAll("~0", "~");
}

export function cloneWithoutPointers(value, pointers) {
  assertCanonicalValue(value);
  const clone = structuredClone(value);
  for (const pointer of pointers) {
    if (!pointer.startsWith("/"))
      throw new TypeError(`Invalid JSON pointer ${pointer}.`);
    const parts = pointer.slice(1).split("/").map(unescapePointer);
    let current = clone;
    for (const part of parts.slice(0, -1)) {
      if (!current || typeof current !== "object" || !(part in current))
        throw new TypeError(`Missing mutable pointer ${pointer}.`);
      current = current[part];
    }
    const last = parts.at(-1);
    if (!current || typeof current !== "object" || !(last in current))
      throw new TypeError(`Missing mutable pointer ${pointer}.`);
    delete current[last];
  }
  return clone;
}

export const ROW_MUTABLE_POINTERS = Object.freeze([
  "/audit/state",
  "/audit/reviewer",
  "/audit/reviewed_at",
  "/audit/report_ids",
  "/audit/audited_payload_sha256",
]);

export function auditedRowDigest(row) {
  return sha256(canonicalize(cloneWithoutPointers(row, ROW_MUTABLE_POINTERS)));
}

export function normalizeRepositoryPath(value) {
  if (
    typeof value !== "string" ||
    !value ||
    value.startsWith("/") ||
    /^[A-Za-z]:/u.test(value) ||
    value.includes("\\") ||
    /[\u0000-\u001f\u007f]/u.test(value) ||
    value.endsWith("/") ||
    value.includes("//")
  )
    throw new TypeError(
      "Evidence path is not a normalized repository-relative path."
    );
  const components = value.split("/");
  if (
    components.some(
      (component) => !component || component === "." || component === ".."
    )
  )
    throw new TypeError("Evidence path contains a forbidden component.");
  if (value.normalize("NFC") !== value)
    throw new TypeError(
      "Evidence path must already be Unicode NFC normalized."
    );
  return value;
}

function streamBuffer(stream) {
  let buffer = Buffer.alloc(0);
  let ended = false;
  let failure;
  const waiters = [];
  const wake = () => waiters.splice(0).forEach((resolve) => resolve());
  stream.on("data", (chunk) => {
    buffer = Buffer.concat([buffer, chunk]);
    wake();
  });
  stream.on("end", () => {
    ended = true;
    wake();
  });
  stream.on("error", (error) => {
    failure = error;
    ended = true;
    wake();
  });
  const awaitData = async () => {
    if (failure) throw failure;
    if (ended) throw internalError("Unexpected Git batch EOF.");
    await new Promise((resolve) => waiters.push(resolve));
    if (failure) throw failure;
  };
  return {
    async line() {
      for (;;) {
        const newline = buffer.indexOf(0x0a);
        if (newline >= 0) {
          const value = buffer.subarray(0, newline).toString("utf8");
          buffer = buffer.subarray(newline + 1);
          return value;
        }
        await awaitData();
      }
    },
    async bytes(length) {
      while (buffer.length < length) await awaitData();
      const value = Buffer.from(buffer.subarray(0, length));
      buffer = buffer.subarray(length);
      return value;
    },
  };
}

export class GitObjectReader {
  constructor(repositoryRoot, { spawnProcess = spawn } = {}) {
    this.repositoryRoot = realpathSync(repositoryRoot);
    this.checkProcess = spawnProcess("git", ["cat-file", "--batch-check"], {
      ...gitProcessOptions(this.repositoryRoot),
      stdio: ["pipe", "pipe", "pipe"],
    });
    this.contentProcess = spawnProcess("git", ["cat-file", "--batch"], {
      ...gitProcessOptions(this.repositoryRoot),
      stdio: ["pipe", "pipe", "pipe"],
    });
    this.checkReader = streamBuffer(this.checkProcess.stdout);
    this.contentReader = streamBuffer(this.contentProcess.stdout);
    this.stderr = "";
    this.checkProcess.stderr.on(
      "data",
      (chunk) => (this.stderr += chunk.toString("utf8"))
    );
    this.contentProcess.stderr.on(
      "data",
      (chunk) => (this.stderr += chunk.toString("utf8"))
    );
    this.objectCache = new Map();
    this.contentCache = new Map();
    this.pathCache = new Map();
    this.treeCache = new Map();
    this.negativeCache = new Map();
    this.metrics = {
      batch_check_processes: 1,
      batch_content_processes: 1,
      content_transfers: 0,
      content_transfers_by_oid: {},
      negative_cache_hits: 0,
    };
    this.checkQueue = Promise.resolve();
    this.contentQueue = Promise.resolve();
    this.processFailures = [];
    for (const child of [this.checkProcess, this.contentProcess]) {
      child.on("error", (error) => this.processFailures.push(error));
      child.stdin.on("error", (error) => this.processFailures.push(error));
    }
    this.closed = false;
  }

  async request(stream, text) {
    await new Promise((resolve, reject) => {
      try {
        stream.write(text, (error) => (error ? reject(error) : resolve()));
      } catch (error) {
        reject(error);
      }
    });
  }

  enqueue(queueName, operation) {
    const result = this[queueName].then(operation, operation);
    this[queueName] = result.catch(() => {});
    return result;
  }

  async object(specification) {
    if (this.objectCache.has(specification))
      return this.objectCache.get(specification);
    if (/[\n\r\0]/u.test(specification))
      throw new TypeError("Unsafe Git object specification.");
    return this.enqueue("checkQueue", async () => {
      if (this.objectCache.has(specification))
        return this.objectCache.get(specification);
      await this.request(this.checkProcess.stdin, `${specification}\n`);
      const line = await this.checkReader.line();
      if (line === `${specification} missing` || line.endsWith(" missing")) {
        this.objectCache.set(specification, null);
        return null;
      }
      const match = /^([0-9a-f]{40,64}) ([a-z]+) (\d+)$/u.exec(line);
      if (!match) throw internalError("Malformed Git batch-check response.");
      const result = { oid: match[1], type: match[2], size: Number(match[3]) };
      this.objectCache.set(specification, result);
      return result;
    });
  }

  async bytes(oid) {
    if (!SAFE_OID.test(oid)) throw new TypeError("Invalid Git object ID.");
    if (this.contentCache.has(oid)) return this.contentCache.get(oid);
    return this.enqueue("contentQueue", async () => {
      if (this.contentCache.has(oid)) return this.contentCache.get(oid);
      await this.request(this.contentProcess.stdin, `${oid}\n`);
      const header = await this.contentReader.line();
      const match = /^([0-9a-f]{40,64}) ([a-z]+) (\d+)$/u.exec(header);
      if (!match || match[1] !== oid)
        throw internalError("Malformed Git batch content header.");
      const length = Number(match[3]);
      const bytes = await this.contentReader.bytes(length);
      const terminator = await this.contentReader.bytes(1);
      if (terminator[0] !== 0x0a || bytes.length !== length)
        throw internalError("Git batch content size mismatch.");
      this.metrics.content_transfers += 1;
      this.metrics.content_transfers_by_oid[oid] =
        (this.metrics.content_transfers_by_oid[oid] ?? 0) + 1;
      this.contentCache.set(oid, bytes);
      return bytes;
    });
  }

  async treeEntries(treeOid) {
    if (this.treeCache.has(treeOid)) return this.treeCache.get(treeOid);
    const object = await this.object(treeOid);
    if (!object || object.type !== "tree")
      throw new Error("Expected Git tree object.");
    const bytes = await this.bytes(object.oid);
    const entries = new Map();
    let cursor = 0;
    const oidLength = treeOid.length === 64 ? 32 : 20;
    while (cursor < bytes.length) {
      const space = bytes.indexOf(0x20, cursor);
      const nul = bytes.indexOf(0, space + 1);
      if (space < 0 || nul < 0 || nul + 1 + oidLength > bytes.length)
        throw internalError("Malformed raw Git tree object.");
      const mode = bytes.subarray(cursor, space).toString("ascii");
      const nameBytes = bytes.subarray(space + 1, nul);
      const name = new TextDecoder("utf-8", { fatal: true }).decode(nameBytes);
      const oid = bytes.subarray(nul + 1, nul + 1 + oidLength).toString("hex");
      entries.set(name, { mode, oid });
      cursor = nul + 1 + oidLength;
    }
    this.treeCache.set(treeOid, entries);
    return entries;
  }

  async resolvePath(commitSha, repositoryPath) {
    if (!SAFE_SHA.test(commitSha))
      throw new TypeError("Commit must be a full SHA-1.");
    repositoryPath = normalizeRepositoryPath(repositoryPath);
    const cacheKey = `${commitSha}\0${repositoryPath}`;
    if (this.pathCache.has(cacheKey)) return this.pathCache.get(cacheKey);
    if (this.negativeCache.has(cacheKey)) {
      this.metrics.negative_cache_hits += 1;
      return null;
    }
    const commit = await this.object(commitSha);
    if (!commit || commit.type !== "commit")
      throw new Error("Evidence commit is missing or not a commit.");
    const rootTree = await this.object(`${commitSha}^{tree}`);
    if (!rootTree || rootTree.type !== "tree")
      throw new Error("Commit tree is unavailable.");
    let treeOid = rootTree.oid;
    const parts = repositoryPath.split("/");
    for (let index = 0; index < parts.length; index += 1) {
      const entry = (await this.treeEntries(treeOid)).get(parts[index]);
      if (!entry) {
        this.negativeCache.set(cacheKey, true);
        return null;
      }
      if (index < parts.length - 1) {
        if (entry.mode !== "40000" && entry.mode !== "040000")
          throw new Error(
            "Evidence path crosses a non-directory or submodule."
          );
        treeOid = entry.oid;
      } else {
        if (!new Set(["100644", "100755"]).has(entry.mode))
          throw new Error("Evidence path is not a regular tracked blob.");
        const result = { ...entry, type: "blob" };
        this.pathCache.set(cacheKey, result);
        return result;
      }
    }
    return null;
  }

  async blobAt(commitSha, repositoryPath) {
    const resolved = await this.resolvePath(commitSha, repositoryPath);
    if (!resolved) return null;
    return this.bytes(resolved.oid);
  }

  workingTreePathIsClean(repositoryPath) {
    repositoryPath = normalizeRepositoryPath(repositoryPath);
    const result = spawnSync(
      "git",
      [
        "status",
        "--porcelain=v1",
        "-z",
        "--untracked-files=all",
        "--",
        repositoryPath,
      ],
      {
        ...gitProcessOptions(this.repositoryRoot),
        encoding: null,
        maxBuffer: 16 * 1024 * 1024,
      }
    );
    if (result.status !== 0)
      throw new Error("Git worktree status check failed.");
    return result.stdout.length === 0;
  }

  async durablePathIsRetained(storageCommitSha, repositoryPath, blobOid) {
    const headSha = runGit(this.repositoryRoot, [
      "rev-parse",
      "--verify",
      "HEAD",
    ]);
    if (!SAFE_SHA.test(headSha))
      throw new Error("Current HEAD is not a full commit SHA.");
    const ancestor = isAncestor(this.repositoryRoot, storageCommitSha, headSha);
    const current = await this.resolvePath(headSha, repositoryPath);
    return {
      ancestor,
      currentBlobMatches: Boolean(current && current.oid === blobOid),
    };
  }

  commitIsAncestorOfHead(commitSha) {
    if (!SAFE_SHA.test(commitSha))
      throw new TypeError("Commit must be a full SHA-1.");
    const headSha = runGit(this.repositoryRoot, [
      "rev-parse",
      "--verify",
      "HEAD",
    ]);
    return isAncestor(this.repositoryRoot, commitSha, headSha);
  }

  commitIsAncestorOf(commitSha, descendantSha) {
    if (!SAFE_SHA.test(commitSha) || !SAFE_SHA.test(descendantSha))
      throw new TypeError("Commits must be full SHA-1 values.");
    return isAncestor(this.repositoryRoot, commitSha, descendantSha);
  }

  derivePreviousArtifact(candidateBytes, repositoryPath = V1_PATH) {
    return derivePreviousV2ArtifactIdentity(
      this.repositoryRoot,
      candidateBytes,
      repositoryPath
    );
  }

  lifecyclePathVersions(repositoryPath = V1_PATH) {
    return firstParentPathVersions(this.repositoryRoot, repositoryPath);
  }

  async close() {
    if (this.closed) return;
    this.closed = true;
    this.checkProcess.stdin.end();
    this.contentProcess.stdin.end();
    await Promise.all([this.checkQueue, this.contentQueue]);
    const exits = await Promise.all(
      [this.checkProcess, this.contentProcess].map((child) =>
        child.exitCode !== null || child.signalCode !== null
          ? { code: child.exitCode, signal: child.signalCode }
          : new Promise((resolve) =>
              child.once("close", (code, signal) => resolve({ code, signal }))
            )
      )
    );
    if (
      this.processFailures.length ||
      this.stderr.trim() ||
      exits.some(({ code, signal }) => code !== 0 || signal !== null)
    )
      throw internalError("Git batch process failed.");
  }
}

function rawLines(bytes) {
  const lines = [];
  let start = 0;
  for (let index = 0; index < bytes.length; index += 1) {
    if (bytes[index] === 0x0a) {
      lines.push({ start, end: index + 1 });
      start = index + 1;
    }
  }
  if (start < bytes.length || bytes.length === 0)
    lines.push({ start, end: bytes.length });
  return lines;
}

// Cache line indexes by blob Buffer identity. GitObjectReader.contentCache returns
// the same Buffer for a given oid, so 119 evidence checks must not rebuild ~1e5
// line descriptors against the multi-megabyte pinned V1 blob on every call.
const RAW_LINES_CACHE = new WeakMap();

function rawLinesCached(bytes) {
  if (!Buffer.isBuffer(bytes)) return rawLines(bytes);
  let lines = RAW_LINES_CACHE.get(bytes);
  if (!lines) {
    lines = rawLines(bytes);
    RAW_LINES_CACHE.set(bytes, lines);
  }
  return lines;
}

export async function verifySourceEvidence(evidence, gitObjects) {
  const errors = [];
  try {
    const bytes = await gitObjects.blobAt(evidence.source_sha, evidence.path);
    if (!bytes)
      return [issue("EVIDENCE_MISSING", "/path", "Evidence blob is missing.")];
    const lines = rawLinesCached(bytes);
    const { start, end } = evidence.lines ?? {};
    if (
      !Number.isInteger(start) ||
      !Number.isInteger(end) ||
      start < 1 ||
      end < start ||
      end > lines.length
    )
      return [
        issue("EVIDENCE_LINES", "/lines", "Evidence line range is invalid."),
      ];
    const snippet = bytes.subarray(lines[start - 1].start, lines[end - 1].end);
    let decoded;
    try {
      decoded = new TextDecoder("utf-8", { fatal: true }).decode(snippet);
    } catch {
      return [
        issue("EVIDENCE_UTF8", "/path", "Evidence snippet is not UTF-8."),
      ];
    }
    if (!decoded.includes(evidence.symbol))
      errors.push(
        issue(
          "EVIDENCE_SYMBOL",
          "/symbol",
          "Symbol is absent from exact snippet bytes."
        )
      );
    if (sha256(snippet) !== evidence.snippet_sha256)
      errors.push(
        issue("EVIDENCE_DIGEST", "/snippet_sha256", "Snippet SHA-256 differs.")
      );
  } catch (error) {
    errors.push(issue("EVIDENCE_GIT", "/path", error.message));
  }
  return errors;
}

function globRegex(pattern) {
  let output = "^";
  for (let index = 0; index < pattern.length; index += 1) {
    const character = pattern[index];
    if (character === "*" && pattern[index + 1] === "*") {
      output += ".*";
      index += 1;
    } else if (character === "*") output += "[^/]*";
    else if (character === "?") output += "[^/]";
    else output += character.replace(/[\\^$.*+?()[\]{}|]/gu, "\\$&");
  }
  return new RegExp(`${output}$`, "u");
}

async function walkTree(reader, treeOid, prefix = "") {
  const files = [];
  const entries = await reader.treeEntries(treeOid);
  for (const [name, entry] of entries) {
    const child = prefix ? `${prefix}/${name}` : name;
    if (entry.mode === "40000" || entry.mode === "040000")
      files.push(...(await walkTree(reader, entry.oid, child)));
    else if (entry.mode === "100644" || entry.mode === "100755")
      files.push({ path: child, oid: entry.oid });
    else
      files.push({ path: child, oid: entry.oid, unsupportedMode: entry.mode });
  }
  return files;
}

export async function verifyAbsenceEvidence(evidence, gitObjects) {
  const errors = [];
  try {
    const commit = await gitObjects.object(evidence.source_sha);
    if (!commit || commit.type !== "commit")
      throw new Error("Absence commit is unavailable.");
    const root = await gitObjects.object(`${evidence.source_sha}^{tree}`);
    const allFiles = await walkTree(gitObjects, root.oid);
    const roots = evidence.roots.map(normalizeRepositoryPath);
    for (const rootPath of roots) {
      if (
        !allFiles.some(
          ({ path: file }) =>
            file === rootPath || file.startsWith(`${rootPath}/`)
        )
      )
        errors.push(
          issue(
            "ABSENCE_ROOT_MISSING",
            "/roots",
            "Absence search root is missing at the pinned commit."
          )
        );
    }
    const exclusions = (evidence.exclusions ?? []).map(globRegex);
    const selected = allFiles.filter(
      ({ path: file }) =>
        roots.some(
          (rootPath) => file === rootPath || file.startsWith(`${rootPath}/`)
        ) && !exclusions.some((regex) => regex.test(file))
    );
    const matcher =
      evidence.mode === "regex" ? new RegExp(evidence.expression, "u") : null;
    let matches = 0;
    for (const file of selected) {
      if (file.unsupportedMode) {
        const code =
          file.unsupportedMode === "120000"
            ? "ABSENCE_SYMLINK"
            : file.unsupportedMode === "160000"
            ? "ABSENCE_GITLINK"
            : "ABSENCE_NON_REGULAR";
        errors.push(
          issue(
            code,
            "/roots",
            "Absence scope contains a non-regular tracked entry."
          )
        );
        continue;
      }
      const bytes = await gitObjects.bytes(file.oid);
      let text;
      try {
        text = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
      } catch {
        errors.push(
          issue(
            "ABSENCE_UTF8",
            "/roots",
            "Absence scope contains a non-UTF-8 tracked file."
          )
        );
        continue;
      }
      if (matcher ? matcher.test(text) : text.includes(evidence.expression))
        matches += 1;
    }
    const expected = evidence.expected_match_count ?? evidence.expected_matches;
    if (matches !== expected)
      errors.push(
        issue(
          "ABSENCE_MATCHES",
          "/expected_match_count",
          `Expected ${expected}; found ${matches}.`
        )
      );
  } catch (error) {
    errors.push(issue("ABSENCE_GIT", "/expression", error.message));
  }
  return errors;
}

function schemaPath(schemaId) {
  if (
    schemaId === "audit" ||
    schemaId.endsWith("feature-parity-audit-normalization.schema.json")
  )
    return AUDIT_SCHEMA_PATH;
  if (
    schemaId === "v2" ||
    schemaId.endsWith("feature-parity-traceability-v2.schema.json")
  )
    return V2_SCHEMA_PATH;
  throw new TypeError("Unknown traceability schema ID.");
}

export function validateSchema(
  schemaId,
  value,
  { repositoryRoot = defaultRepositoryRoot() } = {}
) {
  try {
    assertCanonicalValue(value);
  } catch (error) {
    return [issue("CANONICAL_VALUE", "/", error.message, schemaPath(schemaId))];
  }
  const file = path.join(repositoryRoot, schemaPath(schemaId));
  const schema = parseCanonicalJson(readFileSync(file));
  const support = auditSupportedSchema(schema);
  if (support.length)
    return support.map((entry) =>
      issue(entry.code, entry.path, entry.message, schemaPath(schemaId))
    );
  return validateSchemaInstance(schema, value).map((entry) =>
    issue(entry.code, entry.path, entry.message, schemaPath(schemaId))
  );
}

function valuesEqual(left, right) {
  return canonicalize(left).equals(canonicalize(right));
}

function sectionCounts(rows) {
  const result = {};
  for (const row of rows)
    result[row.section_id] = (result[row.section_id] ?? 0) + 1;
  return result;
}

function duplicates(values) {
  const seen = new Set();
  return values.filter((value) =>
    seen.has(value) ? true : (seen.add(value), false)
  );
}

function correctionManifestIsValid(rows) {
  return rows.every((row) => {
    const correction = row.current_product?.status_correction;
    const expected = EXPECTED_STATUS_CORRECTION_TRANSITIONS.get(row.id);
    if (expected)
      return (
        correction?.changed === true &&
        correction.recorded_status === expected[0] &&
        correction.audited_status === expected[1] &&
        row.current_product.status === expected[1]
      );
    return (
      correction?.changed === false &&
      correction.recorded_status === correction.audited_status &&
      correction.audited_status === row.current_product?.status
    );
  });
}

function changedStatusCorrections(rows) {
  return rows.filter((row) => {
    const correction = row.current_product?.status_correction;
    return (
      correction?.changed === true &&
      correction.recorded_status !== correction.audited_status
    );
  });
}

function statusCorrectionSubtotal(rows) {
  return changedStatusCorrections(rows).filter((row) =>
    SUBTOTAL_SECTIONS.has(row.section_id)
  ).length;
}

function evidenceIntegrityErrors(rows, rowPointer) {
  const errors = [];
  const globallySeen = new Set();
  const globallySeenReachability = new Set();
  for (const [rowIndex, row] of rows.entries()) {
    if (row.audit?.subject_sha !== AUDITED_SOURCE_COMMIT)
      errors.push(
        issue(
          "EVIDENCE_SUBJECT",
          `${rowPointer}/${rowIndex}/audit/subject_sha`,
          "Row audit subject must be pinned to the audited source commit."
        )
      );
    for (const [clauseIndex, clause] of (row.clauses ?? []).entries()) {
      const clausePointer = `${rowPointer}/${rowIndex}/clauses/${clauseIndex}`;
      const reachabilityId = clause.reachability?.id;
      if (
        globallySeenReachability.has(reachabilityId) ||
        !reachabilityId?.startsWith(`REACH-${row.id}-`)
      )
        errors.push(
          issue(
            "REACHABILITY_ID_OWNERSHIP",
            `${clausePointer}/reachability/id`,
            "Reachability IDs must be globally unique and namespaced to their owning requirement."
          )
        );
      globallySeenReachability.add(reachabilityId);
      const collections = [
        ["source_evidence", clause.source_evidence ?? []],
        ["absence_evidence", clause.absence_evidence ?? []],
        ["existing_tests", clause.existing_tests ?? []],
      ];
      const localIds = new Set();
      const runtimeEvidenceIds = new Set();
      const reachabilityEntryIds = new Set();
      const absenceEvidenceIds = new Set();
      for (const [collectionName, records] of collections) {
        for (const [recordIndex, record] of records.entries()) {
          const pointer = `${clausePointer}/${collectionName}/${recordIndex}`;
          if (localIds.has(record.id) || globallySeen.has(record.id))
            errors.push(
              issue(
                "EVIDENCE_ID_DUPLICATE",
                `${pointer}/id`,
                "Evidence IDs must be globally unique across evidence and existing-test records."
              )
            );
          localIds.add(record.id);
          globallySeen.add(record.id);
          if (
            collectionName === "absence_evidence" ||
            (collectionName === "source_evidence" &&
              record.role !== "test_source")
          )
            runtimeEvidenceIds.add(record.id);
          if (
            collectionName === "source_evidence" &&
            record.role === "reachability_entry"
          )
            reachabilityEntryIds.add(record.id);
          if (collectionName === "absence_evidence")
            absenceEvidenceIds.add(record.id);
          if (record.source_sha !== AUDITED_SOURCE_COMMIT)
            errors.push(
              issue(
                "EVIDENCE_SUBJECT",
                `${pointer}/source_sha`,
                "Evidence must be pinned to the audited source commit."
              )
            );
        }
      }
      for (const [referenceIndex, evidenceId] of (
        clause.causality?.evidence_ids ?? []
      ).entries())
        if (!runtimeEvidenceIds.has(evidenceId))
          errors.push(
            issue(
              "EVIDENCE_ID_DANGLING",
              `${clausePointer}/causality/evidence_ids/${referenceIndex}`,
              "Causality evidence ID does not resolve to runtime source or absence evidence within its clause."
            )
          );
      for (const [referenceIndex, evidenceId] of (
        clause.reachability?.entry_evidence_ids ?? []
      ).entries())
        if (!reachabilityEntryIds.has(evidenceId))
          errors.push(
            issue(
              "REACHABILITY_ENTRY_REFERENCE",
              `${clausePointer}/reachability/entry_evidence_ids/${referenceIndex}`,
              "Reachability entry must resolve to clause-local source evidence with role reachability_entry."
            )
          );
      for (const [referenceIndex, evidenceId] of (
        clause.reachability?.absence_evidence_ids ?? []
      ).entries())
        if (!absenceEvidenceIds.has(evidenceId))
          errors.push(
            issue(
              "REACHABILITY_ABSENCE_REFERENCE",
              `${clausePointer}/reachability/absence_evidence_ids/${referenceIndex}`,
              "Reachability absence evidence must resolve to clause-local absence evidence."
            )
          );
      const reachabilityKind = clause.reachability?.kind;
      const entryCount = clause.reachability?.entry_evidence_ids?.length ?? 0;
      const chainCount = clause.reachability?.ordered_chain?.length ?? 0;
      const absenceCount =
        clause.reachability?.absence_evidence_ids?.length ?? 0;
      if (
        (["reachable", "conditional"].includes(reachabilityKind) &&
          (entryCount < 1 || chainCount < 1 || absenceCount !== 0)) ||
        (["not_exposed", "dead_path_proven"].includes(reachabilityKind) &&
          (entryCount !== 0 || chainCount !== 0 || absenceCount < 1))
      )
        errors.push(
          issue(
            "REACHABILITY_SHAPE",
            `${clausePointer}/reachability`,
            "Reachability kind has contradictory entry, chain, or absence evidence."
          )
        );
    }
  }
  return errors;
}

function countsBy(values, selector, vocabulary) {
  return Object.fromEntries(
    vocabulary.map((name) => [
      name,
      values.filter((value) => selector(value) === name).length,
    ])
  );
}

const REPORT_STORAGE_IDENTITY_KEYS = Object.freeze([
  "storage_commit_sha",
  "storage_blob_oid",
  "storage_projection",
  "path",
  "file_sha256",
]);

export function durableReportProjection(report) {
  assertCanonicalValue(report);
  const projection = structuredClone(report);
  for (const key of REPORT_STORAGE_IDENTITY_KEYS) delete projection[key];
  return projection;
}

async function verifyDurableReport(report, gitObjects) {
  const errors = [];
  try {
    const subject = await gitObjects.object(report.subject_sha);
    if (!subject || subject.type !== "commit")
      errors.push(
        issue(
          "REPORT_SUBJECT_COMMIT",
          "/subject_sha",
          "Report subject is missing or is not a commit."
        )
      );
    else if (
      typeof gitObjects.commitIsAncestorOfHead !== "function" ||
      !gitObjects.commitIsAncestorOfHead(report.subject_sha)
    )
      errors.push(
        issue(
          "REPORT_SUBJECT_HISTORY",
          "/subject_sha",
          "Report subject commit is not an ancestor of current HEAD."
        )
      );
    if (
      typeof gitObjects.commitIsAncestorOf !== "function" ||
      !gitObjects.commitIsAncestorOf(
        report.subject_sha,
        report.storage_commit_sha
      )
    )
      errors.push(
        issue(
          "REPORT_SUBJECT_STORAGE_HISTORY",
          "/subject_sha",
          "Report subject commit is not an ancestor of its storage commit."
        )
      );
    if (
      typeof gitObjects.workingTreePathIsClean !== "function" ||
      !gitObjects.workingTreePathIsClean(report.path)
    )
      errors.push(
        issue(
          "REPORT_WORKTREE_STATE",
          "/path",
          "Durable report path is dirty or untracked in the current worktree."
        )
      );
    const resolved = await gitObjects.resolvePath(
      report.storage_commit_sha,
      report.path
    );
    if (!resolved)
      return [
        ...errors,
        issue(
          "REPORT_MISSING",
          "/path",
          "Durable report is missing at its storage commit."
        ),
      ];
    if (resolved.oid !== report.storage_blob_oid)
      errors.push(
        issue(
          "REPORT_BLOB_OID",
          "/storage_blob_oid",
          "Durable report blob OID differs."
        )
      );
    if (typeof gitObjects.durablePathIsRetained !== "function")
      errors.push(
        issue(
          "REPORT_HISTORY",
          "/storage_commit_sha",
          "Git object adapter cannot prove durable report history."
        )
      );
    else {
      const retained = await gitObjects.durablePathIsRetained(
        report.storage_commit_sha,
        report.path,
        report.storage_blob_oid
      );
      if (!retained.ancestor)
        errors.push(
          issue(
            "REPORT_HISTORY",
            "/storage_commit_sha",
            "Report storage commit is not an ancestor of current HEAD."
          )
        );
      if (!retained.currentBlobMatches)
        errors.push(
          issue(
            "REPORT_HEAD_BLOB",
            "/path",
            "Current HEAD does not retain the exact durable report blob."
          )
        );
    }
    const bytes = await gitObjects.bytes(resolved.oid);
    if (sha256(bytes) !== report.file_sha256)
      errors.push(
        issue(
          "REPORT_FILE_DIGEST",
          "/file_sha256",
          "Durable report exact-byte SHA-256 differs."
        )
      );
    let payload;
    try {
      payload = parseCanonicalJson(bytes);
      if (!bytes.equals(Buffer.from(prettyJson(payload))))
        throw new SyntaxError(
          "Report JSON must use canonical committed pretty-JSON bytes."
        );
    } catch (error) {
      errors.push(issue("REPORT_JSON", "/path", error.message));
      return errors;
    }
    if (!valuesEqual(payload, durableReportProjection(report)))
      errors.push(
        issue(
          "REPORT_PAYLOAD",
          "/path",
          "Durable report payload differs from the deterministic embedded-record projection."
        )
      );
  } catch (error) {
    errors.push(issue("REPORT_GIT", "/path", error.message));
  }
  return errors;
}

const EXECUTION_STORAGE_IDENTITY_KEYS = Object.freeze([
  "storage_commit_sha",
  "storage_blob_oid",
  "storage_projection",
  "path",
  "file_sha256",
  "canonical_sha256",
]);

export function durableExecutionProjection(execution) {
  assertCanonicalValue(execution);
  const projection = structuredClone(execution);
  for (const key of EXECUTION_STORAGE_IDENTITY_KEYS) delete projection[key];
  return projection;
}

const EXECUTION_SEMANTIC_KEYS = new Set([
  "schema_version",
  "id",
  "kind",
  "test_case_id",
  "runner_id",
  "invocation_id",
  "invocation_fingerprint_sha256",
  "subject_sha",
  "started_at",
  "finished_at",
  "result",
  "exit_code",
  "assertions",
  "result_fingerprint_sha256",
]);
const EXECUTION_RECORD_ID = /^EXEC-[0-9]{6}$/u;
const EXECUTION_TEST_CASE_ID =
  /^TST-FR-7\.(?:[1-9]|1[01])-[0-9]{3}-REQ-[0-9]{3}$/u;
const EXECUTION_INVOCATION_ID =
  /^INV-FR-7\.(?:[1-9]|1[01])-[0-9]{3}-REQ-[0-9]{3}-[0-9]{3}$/u;
const EXECUTION_ASSERTION_ID =
  /^ASSERT-FR-7\.(?:[1-9]|1[01])-[0-9]{3}-REQ-[0-9]{3}-[0-9]{3}$/u;
const EXECUTION_RUNNERS = new Set([
  "NODE_TEST",
  "NPM_SCRIPT",
  "CARGO_TEST",
  "SYNAPSE_HARNESS",
  "DESKTOP_UI",
  "IOS_XCTEST",
  "PLATFORM_SCENARIO",
]);

function executionIdentifierOwnsTest(value, prefix, testCaseId) {
  return (
    typeof value === "string" &&
    value.startsWith(`${prefix}-${testCaseId.slice(4)}-`)
  );
}

function requiredTestRegistryEntries(rows, rootKey) {
  return rows.flatMap((row, requirementIndex) =>
    (row.clauses ?? []).flatMap((clause, clauseIndex) =>
      (clause.required_tests ?? []).map((requiredTest, requiredTestIndex) => [
        requiredTest.id,
        {
          requiredTest,
          clauseId: clause.id,
          requirementId: row.id,
          pointer: `/${rootKey}/${requirementIndex}/clauses/${clauseIndex}/required_tests/${requiredTestIndex}`,
        },
      ])
    )
  );
}

function requiredTestRegistryErrors(
  entries,
  { duplicateCode, duplicatePointer, contractCode, sourceAudit = false }
) {
  const errors = [];
  if (new Set(entries.map(([testId]) => testId)).size !== entries.length)
    errors.push(
      issue(
        duplicateCode,
        duplicatePointer,
        "Required-test IDs must be globally unique before executions can resolve them."
      )
    );
  for (const [testId, { requiredTest, pointer }] of entries) {
    const contract = requiredTest.execution_contract;
    const domainMismatch = sourceAudit
      ? requiredTest.status !== "planned" || contract !== undefined
      : (requiredTest.status === "planned" && contract !== undefined) ||
        (requiredTest.status !== "planned" && contract === undefined);
    if (domainMismatch)
      errors.push(
        issue(
          contractCode,
          pointer,
          sourceAudit
            ? "Source-audit required tests must remain planned and cannot carry execution contracts."
            : "Planned required tests cannot carry execution contracts, and every implemented or accepted test must define one."
        )
      );
    if (
      contract &&
      (!executionIdentifierOwnsTest(contract.invocation_id, "INV", testId) ||
        !(contract.assertion_ids ?? []).every((assertionId) =>
          executionIdentifierOwnsTest(assertionId, "ASSERT", testId)
        ))
    )
      errors.push(
        issue(
          contractCode,
          pointer,
          "Execution-contract invocation and assertion IDs must be derived from their exact owning required-test ID."
        )
      );
  }
  return errors;
}

function preMigrationAuthorityErrors(rows) {
  const errors = [];
  for (const [rowIndex, row] of rows.entries()) {
    const root = `/rows/${rowIndex}`;
    if (row.audit?.state === "accepted")
      errors.push(
        issue(
          "AUDIT_ACCEPTED_STATE_AUTHORITY",
          `${root}/audit/state`,
          "Source-audit rows cannot be accepted without a durable audit-report catalog and verification authority."
        )
      );
    if ((row.audit?.report_ids?.length ?? 0) > 0)
      errors.push(
        issue(
          "AUDIT_REPORT_REFERENCE_AUTHORITY",
          `${root}/audit/report_ids`,
          "Source-audit rows cannot reference audit reports that the source artifact does not carry and verify."
        )
      );
    if ((row.rust_cutover?.validation_report_ids?.length ?? 0) > 0)
      errors.push(
        issue(
          "AUDIT_VALIDATION_REFERENCE_AUTHORITY",
          `${root}/rust_cutover/validation_report_ids`,
          "Source-audit rows cannot reference validation reports that the source artifact does not carry and verify."
        )
      );
    if (typeof row.rust_cutover?.implementation_subject_sha === "string")
      errors.push(
        issue(
          "AUDIT_IMPLEMENTATION_SUBJECT_AUTHORITY",
          `${root}/rust_cutover/implementation_subject_sha`,
          "Source-audit rows cannot claim a Rust implementation subject without implementation-history verification authority."
        )
      );
    for (const [clauseIndex, clause] of (row.clauses ?? []).entries())
      if ((clause.rust_mapping?.validation_report_ids?.length ?? 0) > 0)
        errors.push(
          issue(
            "AUDIT_VALIDATION_REFERENCE_AUTHORITY",
            `${root}/clauses/${clauseIndex}/rust_mapping/validation_report_ids`,
            "Source-audit clauses cannot reference validation reports that the source artifact does not carry and verify."
          )
        );
  }
  return errors;
}

const REQUIREMENT_KEY_MANIFEST = Object.freeze([
  "architecture_decision_ids",
  "audit",
  "blocker_ids",
  "clauses",
  "current_product",
  "id",
  "legacy_inventory_context",
  "manual_acceptance_ids",
  "migration_disposition_ids",
  "origin",
  "plan",
  "rust_cutover",
  "section_id",
  "security_privacy_lifecycle_ids",
  "task_ids",
]);
const ROW_AUDIT_KEY_MANIFEST = Object.freeze([
  "audited_payload_sha256",
  "report_ids",
  "reviewed_at",
  "reviewer",
  "state",
  "subject_sha",
]);
const CLAUSE_KEY_MANIFEST = Object.freeze([
  "absence_evidence",
  "architecture_decision_ids",
  "blocker_ids",
  "causality",
  "current_product_status",
  "existing_tests",
  "id",
  "outcomes",
  "qualifiers",
  "reachability",
  "required_tests",
  "risk_ids",
  "rust_mapping",
  "source_evidence",
  "text",
]);
const REQUIRED_TEST_KEY_MANIFEST = Object.freeze([
  "acceptance",
  "evidence_class",
  "id",
  "status",
  "task_ids",
]);
const RUST_MAPPING_KEY_MANIFEST = Object.freeze([
  "capability_ids",
  "gap_ids",
  "gate_ids",
  "qualification",
  "task_ids",
  "validation_report_ids",
]);
const RUST_CUTOVER_KEY_MANIFEST = Object.freeze([
  "blocker_ids",
  "capability_ids",
  "gap_ids",
  "gate_ids",
  "implementation_subject_sha",
  "matrix_owner",
  "qualification",
  "readiness",
  "surviving_matrix_js_owner",
  "surviving_raw_matrix_http",
  "task_ids",
  "validation_report_ids",
]);
const BLOCKER_KEY_MANIFEST = Object.freeze([
  "affected_clause_ids",
  "affected_requirement_ids",
  "authority",
  "boundary_ids",
  "closure_criteria",
  "closure_evidence",
  "id",
  "kind",
  "owner_task_ids",
  "qualification",
  "severity",
  "status",
  "threat_ids",
]);
const ARCHITECTURE_DECISION_AUDIT_KEY_MANIFEST = Object.freeze([
  "affected_clause_ids",
  "affected_requirement_ids",
  "closure_evidence",
  "decision",
  "id",
  "owner",
  "scope_authority",
  "status",
]);
const ARCHITECTURE_DECISION_V2_KEY_MANIFEST = Object.freeze([
  ...ARCHITECTURE_DECISION_AUDIT_KEY_MANIFEST,
  "superseded_by_id",
]);

function assertExactKeyManifest(value, expected, location) {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new TypeError(`${location} must be an object.`);
  const actual = Object.keys(value).sort();
  if (!valuesEqual(actual, [...expected].sort()))
    throw new TypeError(`${location} key manifest differs.`);
}

function assertRequiredTestKeyManifest(requiredTest, location) {
  const expected = [...REQUIRED_TEST_KEY_MANIFEST];
  if (Object.hasOwn(requiredTest, "execution_contract"))
    expected.push("execution_contract");
  assertExactKeyManifest(requiredTest, expected, location);
}

export function sourceAuditRowProjection(row) {
  assertCanonicalValue(row);
  assertExactKeyManifest(row, REQUIREMENT_KEY_MANIFEST, "requirement");
  assertExactKeyManifest(
    row.audit,
    ROW_AUDIT_KEY_MANIFEST,
    "requirement.audit"
  );
  assertExactKeyManifest(
    row.rust_cutover,
    RUST_CUTOVER_KEY_MANIFEST,
    "requirement.rust_cutover"
  );
  return {
    id: row.id,
    origin: row.origin,
    section_id: row.section_id,
    plan: row.plan,
    audit: { subject_sha: row.audit.subject_sha },
    current_product: row.current_product,
    clauses: row.clauses.map((clause, clauseIndex) => {
      const location = `requirement.clauses[${clauseIndex}]`;
      assertExactKeyManifest(clause, CLAUSE_KEY_MANIFEST, location);
      assertExactKeyManifest(
        clause.rust_mapping,
        RUST_MAPPING_KEY_MANIFEST,
        `${location}.rust_mapping`
      );
      return {
        id: clause.id,
        text: clause.text,
        current_product_status: clause.current_product_status,
        qualifiers: clause.qualifiers,
        source_evidence: clause.source_evidence,
        absence_evidence: clause.absence_evidence,
        causality: clause.causality,
        existing_tests: clause.existing_tests,
        required_tests: clause.required_tests.map(
          (requiredTest, requiredTestIndex) => {
            assertRequiredTestKeyManifest(
              requiredTest,
              `${location}.required_tests[${requiredTestIndex}]`
            );
            return {
              id: requiredTest.id,
              evidence_class: requiredTest.evidence_class,
              task_ids: requiredTest.task_ids,
              acceptance: requiredTest.acceptance,
            };
          }
        ),
        outcomes: clause.outcomes,
        reachability: clause.reachability,
        rust_mapping: {
          capability_ids: clause.rust_mapping.capability_ids,
          gap_ids: clause.rust_mapping.gap_ids,
          task_ids: clause.rust_mapping.task_ids,
          gate_ids: clause.rust_mapping.gate_ids,
          qualification: clause.rust_mapping.qualification,
        },
        blocker_ids: clause.blocker_ids,
        risk_ids: clause.risk_ids,
        architecture_decision_ids: clause.architecture_decision_ids,
      };
    }),
    rust_cutover: {
      capability_ids: row.rust_cutover.capability_ids,
      task_ids: row.rust_cutover.task_ids,
    },
    blocker_ids: row.blocker_ids,
    task_ids: row.task_ids,
    manual_acceptance_ids: row.manual_acceptance_ids,
    migration_disposition_ids: row.migration_disposition_ids,
    security_privacy_lifecycle_ids: row.security_privacy_lifecycle_ids,
    architecture_decision_ids: row.architecture_decision_ids,
    legacy_inventory_context: row.legacy_inventory_context,
  };
}

export function cutoverStateProjection(row) {
  assertCanonicalValue(row);
  assertExactKeyManifest(row, REQUIREMENT_KEY_MANIFEST, "requirement");
  assertExactKeyManifest(
    row.rust_cutover,
    RUST_CUTOVER_KEY_MANIFEST,
    "requirement.rust_cutover"
  );
  return {
    projection: "rust-cutover-row-state-v1",
    requirement_id: row.id,
    required_tests: row.clauses.map((clause, clauseIndex) => {
      const location = `requirement.clauses[${clauseIndex}]`;
      assertExactKeyManifest(clause, CLAUSE_KEY_MANIFEST, location);
      return {
        clause_id: clause.id,
        tests: clause.required_tests.map((requiredTest, requiredTestIndex) => {
          assertRequiredTestKeyManifest(
            requiredTest,
            `${location}.required_tests[${requiredTestIndex}]`
          );
          return {
            id: requiredTest.id,
            status: requiredTest.status,
            execution_contract: Object.hasOwn(
              requiredTest,
              "execution_contract"
            )
              ? requiredTest.execution_contract
              : null,
          };
        }),
      };
    }),
    clauses: row.clauses.map((clause, clauseIndex) => {
      assertExactKeyManifest(
        clause.rust_mapping,
        RUST_MAPPING_KEY_MANIFEST,
        `requirement.clauses[${clauseIndex}].rust_mapping`
      );
      return {
        clause_id: clause.id,
        validation_report_ids: clause.rust_mapping.validation_report_ids,
      };
    }),
    rust_cutover: {
      readiness: row.rust_cutover.readiness,
      implementation_subject_sha: row.rust_cutover.implementation_subject_sha,
      matrix_owner: row.rust_cutover.matrix_owner,
      surviving_raw_matrix_http: row.rust_cutover.surviving_raw_matrix_http,
      surviving_matrix_js_owner: row.rust_cutover.surviving_matrix_js_owner,
      gap_ids: row.rust_cutover.gap_ids,
      gate_ids: row.rust_cutover.gate_ids,
      blocker_ids: row.rust_cutover.blocker_ids,
      validation_report_ids: row.rust_cutover.validation_report_ids,
      qualification: row.rust_cutover.qualification,
    },
  };
}

export function cutoverStateDigest(row) {
  return sha256(canonicalize(cutoverStateProjection(row)));
}

export function blockerSourceProjection(blocker) {
  assertCanonicalValue(blocker);
  assertExactKeyManifest(blocker, BLOCKER_KEY_MANIFEST, "blocker");
  return {
    id: blocker.id,
    kind: blocker.kind,
    severity: blocker.severity,
    owner_task_ids: blocker.owner_task_ids,
    authority: blocker.authority,
    affected_requirement_ids: blocker.affected_requirement_ids,
    affected_clause_ids: blocker.affected_clause_ids,
    threat_ids: blocker.threat_ids,
    boundary_ids: blocker.boundary_ids,
    closure_criteria: blocker.closure_criteria,
  };
}

export function blockerLifecycleProjection(blocker) {
  assertCanonicalValue(blocker);
  assertExactKeyManifest(blocker, BLOCKER_KEY_MANIFEST, "blocker");
  return {
    projection: "blocker-lifecycle-state-v1",
    blocker_id: blocker.id,
    status: blocker.status,
    closure_evidence: blocker.closure_evidence,
    qualification: blocker.qualification,
  };
}

export function blockerLifecycleDigest(blocker) {
  return sha256(canonicalize(blockerLifecycleProjection(blocker)));
}

function assertArchitectureDecisionKeyManifest(decision, location) {
  assertExactKeyManifest(
    decision,
    Object.hasOwn(decision, "superseded_by_id")
      ? ARCHITECTURE_DECISION_V2_KEY_MANIFEST
      : ARCHITECTURE_DECISION_AUDIT_KEY_MANIFEST,
    location
  );
}

export function architectureDecisionSourceProjection(decision) {
  assertCanonicalValue(decision);
  assertArchitectureDecisionKeyManifest(decision, "architecture_decision");
  return {
    id: decision.id,
    decision: decision.decision,
    owner: decision.owner,
    scope_authority: decision.scope_authority,
    affected_requirement_ids: decision.affected_requirement_ids,
    affected_clause_ids: decision.affected_clause_ids,
  };
}

export function architectureDecisionLifecycleProjection(decision) {
  assertCanonicalValue(decision);
  assertExactKeyManifest(
    decision,
    ARCHITECTURE_DECISION_V2_KEY_MANIFEST,
    "architecture_decision"
  );
  return {
    projection: "architecture-decision-lifecycle-state-v1",
    architecture_decision_id: decision.id,
    status: decision.status,
    closure_evidence: decision.closure_evidence,
    superseded_by_id: decision.superseded_by_id,
  };
}

export function architectureDecisionLifecycleDigest(decision) {
  return sha256(
    canonicalize(architectureDecisionLifecycleProjection(decision))
  );
}

export function lifecycleAuthorizationDigest(authorization) {
  assertCanonicalValue(authorization);
  const projection = structuredClone(authorization);
  delete projection.authorization_sha256;
  return sha256(canonicalize(projection));
}

function entityKey(entityKind, entityId) {
  return `${entityKind}\u0000${entityId}`;
}

function lifecycleProjectionForEntity(entityKind, entity) {
  if (entityKind === "requirement") return cutoverStateProjection(entity);
  if (entityKind === "blocker") return blockerLifecycleProjection(entity);
  if (entityKind === "architecture_decision")
    return architectureDecisionLifecycleProjection(entity);
  throw new TypeError(`Unsupported lifecycle entity kind ${entityKind}.`);
}

function lifecycleDigestForEntity(entityKind, entity) {
  return sha256(canonicalize(lifecycleProjectionForEntity(entityKind, entity)));
}

function sourceProjectionForEntity(entityKind, entity) {
  if (entityKind === "requirement") return sourceAuditRowProjection(entity);
  if (entityKind === "blocker") return blockerSourceProjection(entity);
  if (entityKind === "architecture_decision")
    return architectureDecisionSourceProjection(entity);
  throw new TypeError(`Unsupported lifecycle entity kind ${entityKind}.`);
}

function durableDecisionLifecycleEntity(decision) {
  return { ...decision, superseded_by_id: null };
}

export function sourceBaselineManifest(durableAudit) {
  assertCanonicalValue(durableAudit);
  return [
    ...(durableAudit.rows ?? []).map((entity) => ({
      entity_kind: "requirement",
      entity_id: entity.id,
      projection: "rust-cutover-row-state-v1",
      source_entity_sha256: sha256(
        canonicalize(sourceAuditRowProjection(entity))
      ),
      baseline_payload_sha256: cutoverStateDigest(entity),
    })),
    ...(durableAudit.blockers_and_risks ?? []).map((entity) => ({
      entity_kind: "blocker",
      entity_id: entity.id,
      projection: "blocker-lifecycle-state-v1",
      source_entity_sha256: sha256(
        canonicalize(blockerSourceProjection(entity))
      ),
      baseline_payload_sha256: blockerLifecycleDigest(entity),
    })),
    ...(durableAudit.architecture_decisions ?? []).map((entity) => ({
      entity_kind: "architecture_decision",
      entity_id: entity.id,
      projection: "architecture-decision-lifecycle-state-v1",
      source_entity_sha256: sha256(
        canonicalize(architectureDecisionSourceProjection(entity))
      ),
      baseline_payload_sha256: architectureDecisionLifecycleDigest(
        durableDecisionLifecycleEntity(entity)
      ),
    })),
  ];
}

export function sourceBaselineDigest(durableAudit) {
  return sha256(canonicalize(sourceBaselineManifest(durableAudit)));
}

function lifecycleAuthorizations(value) {
  return (value?.validation_reports ?? []).flatMap(
    (report) => report.lifecycle_authorizations ?? []
  );
}

export function deriveLifecycleManifest(
  value,
  durableAudit,
  previousArtifact = value?.lifecycle?.previous_artifact ?? null
) {
  assertCanonicalValue(value);
  assertCanonicalValue(durableAudit);
  const baseline = sourceBaselineManifest(durableAudit);
  const baselineByKey = new Map(
    baseline.map((entry) => [
      entityKey(entry.entity_kind, entry.entity_id),
      entry,
    ])
  );
  const authorizations = lifecycleAuthorizations(value);
  const authorizationsByKey = new Map();
  for (const authorization of authorizations) {
    const key = entityKey(authorization.entity_kind, authorization.entity_id);
    if (!authorizationsByKey.has(key)) authorizationsByKey.set(key, []);
    authorizationsByKey.get(key).push(authorization);
  }
  const currentEntities = [
    ...(value.requirements ?? []).map((entity) => ["requirement", entity]),
    ...(value.blockers ?? []).map((entity) => ["blocker", entity]),
    ...(value.architecture_decisions ?? []).map((entity) => [
      "architecture_decision",
      entity,
    ]),
  ];
  const currentByKey = new Map(
    currentEntities.map(([kind, entity]) => [
      entityKey(kind, entity.id),
      entity,
    ])
  );
  const orderedKeys = [
    ...baseline.map((entry) => entityKey(entry.entity_kind, entry.entity_id)),
    ...authorizations
      .filter((authorization) => authorization.operation === "create")
      .map((authorization) =>
        entityKey(authorization.entity_kind, authorization.entity_id)
      )
      .filter(
        (key, index, keys) =>
          !baselineByKey.has(key) && keys.indexOf(key) === index
      ),
    ...currentEntities
      .filter(([kind, entity]) => {
        const key = entityKey(kind, entity.id);
        return (
          !baselineByKey.has(key) &&
          !authorizations.some(
            (authorization) =>
              authorization.operation === "create" &&
              entityKey(authorization.entity_kind, authorization.entity_id) ===
                key
          )
        );
      })
      .map(([kind, entity]) => entityKey(kind, entity.id)),
  ];
  return {
    projection: "matrix-rust-cutover-lifecycle-v1",
    source_baseline_sha256: sha256(canonicalize(baseline)),
    previous_artifact: deepCopy(previousArtifact),
    audit_report_ids: (value.audit_reports ?? []).map((report) => report.id),
    validation_report_ids: (value.validation_reports ?? []).map(
      (report) => report.id
    ),
    entity_chains: orderedKeys.map((key) => {
      const baselineEntry = baselineByKey.get(key);
      const entity = currentByKey.get(key);
      const chainAuthorizations = authorizationsByKey.get(key) ?? [];
      const first = chainAuthorizations[0];
      const [entityKind, entityId] = key.split("\u0000");
      return {
        entity_kind: entityKind,
        entity_id: entityId,
        projection: lifecycleProjectionForEntity(entityKind, entity).projection,
        source_origin: baselineEntry ? "durable_audit" : "authorized_create",
        source_entity_sha256:
          baselineEntry?.source_entity_sha256 ?? first?.source_entity_sha256,
        baseline_payload_sha256: baselineEntry?.baseline_payload_sha256 ?? null,
        head_payload_sha256: lifecycleDigestForEntity(entityKind, entity),
        authorization_ids: chainAuthorizations.map(
          (authorization) => authorization.id
        ),
      };
    }),
  };
}

function lifecycleStatus(entityKind, state) {
  if (entityKind === "requirement") return state?.rust_cutover?.readiness;
  return state?.status;
}

function expectedLifecycleOperation(
  entityKind,
  fromState,
  toState,
  seenDigests
) {
  const toDigest = sha256(canonicalize(toState));
  if (seenDigests.has(toDigest)) return "rollback";
  const fromStatus = lifecycleStatus(entityKind, fromState);
  const toStatus = lifecycleStatus(entityKind, toState);
  if (entityKind === "requirement") {
    const core = [
      "not_assessed",
      "implementation_planned",
      "implementation_in_progress",
      "validation_pending",
      "ready",
    ];
    const fromSubject = fromState?.rust_cutover?.implementation_subject_sha;
    const toSubject = toState?.rust_cutover?.implementation_subject_sha;
    if (fromSubject && !toSubject) return "rollback";
    const testRank = new Map([
      ["planned", 0],
      ["implemented_unvalidated", 1],
      ["validation_pending", 2],
      ["accepted", 3],
    ]);
    const fromTests = new Map(
      (fromState?.required_tests ?? []).flatMap((group) =>
        (group.tests ?? []).map((entry) => [entry.id, entry])
      )
    );
    const regressedTest = (toState?.required_tests ?? []).some((group) =>
      (group.tests ?? []).some((entry) => {
        const prior = fromTests.get(entry.id);
        return (
          prior &&
          ((testRank.get(entry.status) ?? -1) <
            (testRank.get(prior.status) ?? -1) ||
            (prior.execution_contract !== null &&
              entry.execution_contract === null))
        );
      })
    );
    const priorClauseReports = new Map(
      (fromState?.clauses ?? []).map((entry) => [
        entry.clause_id,
        entry.validation_report_ids ?? [],
      ])
    );
    const removedClauseReport = (toState?.clauses ?? []).some(
      (entry) =>
        !(priorClauseReports.get(entry.clause_id) ?? []).every((id) =>
          (entry.validation_report_ids ?? []).includes(id)
        )
    );
    const fromCutover = fromState?.rust_cutover ?? {};
    const toCutover = toState?.rust_cutover ?? {};
    const removedRowReport = (fromCutover.validation_report_ids ?? []).some(
      (id) => !(toCutover.validation_report_ids ?? []).includes(id)
    );
    const addedBlock = ["gap_ids", "gate_ids", "blocker_ids"].some((field) =>
      (toCutover[field] ?? []).some(
        (id) => !(fromCutover[field] ?? []).includes(id)
      )
    );
    const degradedOwnership =
      (fromCutover.matrix_owner === "matrix_rust_sdk" &&
        toCutover.matrix_owner !== "matrix_rust_sdk") ||
      (fromCutover.surviving_raw_matrix_http === false &&
        toCutover.surviving_raw_matrix_http === true) ||
      (fromCutover.surviving_matrix_js_owner === false &&
        toCutover.surviving_matrix_js_owner === true);
    if (
      regressedTest ||
      removedClauseReport ||
      removedRowReport ||
      addedBlock ||
      degradedOwnership
    )
      return "rollback";
    if (fromSubject && toSubject && fromSubject !== toSubject) return null;
    if (fromStatus === toStatus) return "advance";
    if (toStatus === "blocked" && fromStatus !== "blocked") return "rollback";
    if (toStatus === "not_assessed" && fromStatus !== "not_assessed")
      return "rollback";
    if (fromStatus === "blocked")
      return core.indexOf(toStatus) > 0 ? "advance" : "rollback";
    return core.indexOf(toStatus) >= core.indexOf(fromStatus)
      ? "advance"
      : "rollback";
  }
  if (entityKind === "blocker") {
    if (fromStatus === toStatus) return "advance";
    const advances = new Set([
      "open\u0000mitigating",
      "open\u0000closed",
      "open\u0000accepted_risk",
      "mitigating\u0000closed",
      "mitigating\u0000accepted_risk",
      "blocked\u0000open",
      "blocked\u0000mitigating",
      "blocked\u0000closed",
      "blocked\u0000accepted_risk",
      "accepted_risk\u0000closed",
    ]);
    return advances.has(`${fromStatus}\u0000${toStatus}`)
      ? "advance"
      : "rollback";
  }
  if (fromStatus === toStatus) return "advance";
  const advances = new Set([
    "unresolved\u0000proposed",
    "unresolved\u0000approved",
    "unresolved\u0000rejected",
    "unresolved\u0000superseded",
    "proposed\u0000approved",
    "proposed\u0000rejected",
    "proposed\u0000superseded",
    "approved\u0000superseded",
    "rejected\u0000superseded",
  ]);
  return advances.has(`${fromStatus}\u0000${toStatus}`)
    ? "advance"
    : "rollback";
}

function changedRowClauseIds(fromState, toState) {
  const fromTests = new Map(
    (fromState?.required_tests ?? []).map((entry) => [entry.clause_id, entry])
  );
  const fromClauses = new Map(
    (fromState?.clauses ?? []).map((entry) => [entry.clause_id, entry])
  );
  const toTests = new Map(
    (toState?.required_tests ?? []).map((entry) => [entry.clause_id, entry])
  );
  const toClauses = new Map(
    (toState?.clauses ?? []).map((entry) => [entry.clause_id, entry])
  );
  return [
    ...new Set([
      ...fromTests.keys(),
      ...toTests.keys(),
      ...fromClauses.keys(),
      ...toClauses.keys(),
    ]),
  ].filter(
    (clauseId) =>
      !valuesEqual(
        fromTests.get(clauseId) ?? {},
        toTests.get(clauseId) ?? {}
      ) ||
      !valuesEqual(
        fromClauses.get(clauseId) ?? {},
        toClauses.get(clauseId) ?? {}
      )
  );
}

function lifecycleContractAnalysis(value, durableAudit) {
  const errors = [];
  const reports = value?.validation_reports ?? [];
  const reportById = new Map(reports.map((report) => [report.id, report]));
  const baselineManifest = sourceBaselineManifest(durableAudit);
  const baselineByKey = new Map(
    baselineManifest.map((entry) => [
      entityKey(entry.entity_kind, entry.entity_id),
      entry,
    ])
  );
  const baselineEntities = new Map([
    ...(durableAudit.rows ?? []).map((entity) => [
      entityKey("requirement", entity.id),
      entity,
    ]),
    ...(durableAudit.blockers_and_risks ?? []).map((entity) => [
      entityKey("blocker", entity.id),
      entity,
    ]),
    ...(durableAudit.architecture_decisions ?? []).map((entity) => [
      entityKey("architecture_decision", entity.id),
      durableDecisionLifecycleEntity(entity),
    ]),
  ]);
  const currentEntities = new Map([
    ...(value.requirements ?? []).map((entity) => [
      entityKey("requirement", entity.id),
      entity,
    ]),
    ...(value.blockers ?? []).map((entity) => [
      entityKey("blocker", entity.id),
      entity,
    ]),
    ...(value.architecture_decisions ?? []).map((entity) => [
      entityKey("architecture_decision", entity.id),
      entity,
    ]),
  ]);
  const allAuthorizations = reports.flatMap((report) =>
    (report.lifecycle_authorizations ?? []).map((authorization) => ({
      authorization,
      report,
    }))
  );
  for (const [kind, durableEntities, currentEntitiesForKind] of [
    ["blocker", durableAudit.blockers_and_risks ?? [], value.blockers ?? []],
    [
      "architecture_decision",
      durableAudit.architecture_decisions ?? [],
      value.architecture_decisions ?? [],
    ],
  ]) {
    const createdIds = allAuthorizations
      .map(({ authorization }) => authorization)
      .filter(
        (authorization) =>
          authorization.operation === "create" &&
          authorization.entity_kind === kind
      )
      .map((authorization) => authorization.entity_id);
    const currentSuffixIds = currentEntitiesForKind
      .slice(durableEntities.length)
      .map((entity) => entity.id);
    if (!valuesEqual(currentSuffixIds, createdIds))
      errors.push(
        issue(
          "V2_LIFECYCLE_CREATE_ORDER",
          kind === "blocker" ? "/blockers" : "/architecture_decisions",
          "Created entities must remain an exact append-only suffix in first-create authorization order."
        )
      );
  }
  const ids = allAuthorizations.map(({ authorization }) => authorization.id);
  if (new Set(ids).size !== ids.length)
    errors.push(
      issue(
        "V2_LIFECYCLE_AUTHORIZATION_IDS",
        "/validation_reports",
        "Lifecycle authorization IDs must be globally unique."
      )
    );
  const grouped = new Map();
  for (const entry of allAuthorizations) {
    const key = entityKey(
      entry.authorization.entity_kind,
      entry.authorization.entity_id
    );
    if (!grouped.has(key)) grouped.set(key, []);
    grouped.get(key).push(entry);
  }
  const latestAuthorizationByKey = new Map();
  for (const [key, entries] of grouped) {
    const baseline = baselineByKey.get(key);
    const currentEntity = currentEntities.get(key);
    const first = entries[0]?.authorization;
    const entityKind = first?.entity_kind;
    let sourceDigest = baseline?.source_entity_sha256 ?? null;
    let sourceEntityProjection = baseline
      ? sourceProjectionForEntity(entityKind, baselineEntities.get(key))
      : null;
    let priorState = baselineEntities.has(key)
      ? lifecycleProjectionForEntity(entityKind, baselineEntities.get(key))
      : null;
    let priorDigest = baseline?.baseline_payload_sha256 ?? null;
    let priorAuthorization = null;
    let priorReviewedAt = Number.NEGATIVE_INFINITY;
    const seenDigests = new Set(priorDigest ? [priorDigest] : []);
    for (const [index, { authorization, report }] of entries.entries()) {
      const pointer = `/validation_reports/${reports.indexOf(
        report
      )}/lifecycle_authorizations/${(
        report.lifecycle_authorizations ?? []
      ).indexOf(authorization)}`;
      const isCreate = authorization.operation === "create";
      if (index === 0 && !baseline) {
        if (!isCreate || authorization.sequence !== 1)
          errors.push(
            issue(
              "V2_LIFECYCLE_CREATE",
              pointer,
              "A created entity must begin with exactly one sequence-1 create authorization."
            )
          );
        try {
          sourceEntityProjection = authorization.source_entity;
          sourceDigest = sha256(canonicalize(authorization.source_entity));
        } catch {
          sourceDigest = null;
        }
      } else if (isCreate)
        errors.push(
          issue(
            "V2_LIFECYCLE_CREATE",
            pointer,
            "Create is legal only for the first authorization of a new blocker or architecture decision."
          )
        );
      if (
        authorization.sequence !== index + 1 ||
        authorization.previous_authorization_sha256 !==
          (priorAuthorization
            ? lifecycleAuthorizationDigest(priorAuthorization)
            : null) ||
        authorization.from_payload_sha256 !== priorDigest
      )
        errors.push(
          issue(
            "V2_LIFECYCLE_CHAIN",
            pointer,
            "Lifecycle sequence, predecessor digest, or from-payload digest is discontinuous."
          )
        );
      if (
        isCreate &&
        (entityKind === "requirement" ||
          (entityKind === "blocker" &&
            authorization.source_entity?.id !== authorization.entity_id) ||
          (entityKind === "architecture_decision" &&
            authorization.source_entity?.id !== authorization.entity_id))
      )
        errors.push(
          issue(
            "V2_LIFECYCLE_CREATE_SOURCE",
            pointer,
            "Create source projection must match its new blocker or architecture-decision identity."
          )
        );
      if (
        authorization.source_entity_sha256 !== sourceDigest ||
        authorization.report_id !== report.id ||
        authorization.subject_sha !== report.subject_sha ||
        authorization.reviewer !== report.reviewer ||
        authorization.reviewed_at !== report.reviewed_at ||
        authorization.authorization_sha256 !==
          lifecycleAuthorizationDigest(authorization)
      )
        errors.push(
          issue(
            "V2_LIFECYCLE_AUTHORIZATION_BINDING",
            pointer,
            "Lifecycle authorization source, report attestation, or self digest differs."
          )
        );
      let toDigest;
      try {
        toDigest = sha256(canonicalize(authorization.to_state));
      } catch {
        toDigest = null;
      }
      const expectedProjection = {
        requirement: "rust-cutover-row-state-v1",
        blocker: "blocker-lifecycle-state-v1",
        architecture_decision: "architecture-decision-lifecycle-state-v1",
      }[entityKind];
      const stateEntityId =
        authorization.to_state?.requirement_id ??
        authorization.to_state?.blocker_id ??
        authorization.to_state?.architecture_decision_id;
      if (
        authorization.projection !== expectedProjection ||
        authorization.to_state?.projection !== expectedProjection ||
        stateEntityId !== authorization.entity_id ||
        authorization.to_payload_sha256 !== toDigest
      )
        errors.push(
          issue(
            "V2_LIFECYCLE_TO_STATE",
            pointer,
            "Lifecycle projection, entity, to-state snapshot, or to-state digest differs."
          )
        );
      const reviewedAt = exactIsoTimestamp(authorization.reviewed_at)
        ? Date.parse(authorization.reviewed_at)
        : Number.NaN;
      if (!Number.isFinite(reviewedAt) || reviewedAt < priorReviewedAt)
        errors.push(
          issue(
            "V2_LIFECYCLE_TIME_ORDER",
            pointer,
            "Lifecycle authorization review times must be canonical and nondecreasing."
          )
        );
      if (!isCreate && priorState) {
        if (toDigest === priorDigest)
          errors.push(
            issue(
              "V2_LIFECYCLE_NOOP",
              pointer,
              "Lifecycle authorizations must change the exact payload and cannot record a no-op transition."
            )
          );
        const expectedOperation = expectedLifecycleOperation(
          entityKind,
          priorState,
          authorization.to_state,
          seenDigests
        );
        if (expectedOperation && authorization.operation !== expectedOperation)
          errors.push(
            issue(
              "V2_LIFECYCLE_OPERATION",
              pointer,
              `Lifecycle transition must be classified as ${expectedOperation}.`
            )
          );
        if (
          expectedOperation === "rollback" &&
          entityKind === "requirement" &&
          authorization.to_state?.rust_cutover?.readiness === "ready"
        )
          errors.push(
            issue(
              "V2_LIFECYCLE_ROLLBACK_READY",
              pointer,
              "A requirement rollback must finish below ready."
            )
          );
      }
      if (entityKind === "requirement" && priorState) {
        const sourceRow = baselineEntities.get(key);
        const expectedClauseIds = (sourceRow?.clauses ?? []).map(
          (clause) => clause.id
        );
        const requiredTestClauseIds = (
          authorization.to_state?.required_tests ?? []
        ).map((group) => group.clause_id);
        const stateClauseIds = (authorization.to_state?.clauses ?? []).map(
          (clause) => clause.clause_id
        );
        const testManifestMatches = (sourceRow?.clauses ?? []).every(
          (sourceClause, clauseIndex) =>
            valuesEqual(
              (
                authorization.to_state?.required_tests?.[clauseIndex]?.tests ??
                []
              ).map((requiredTest) => requiredTest.id),
              (sourceClause.required_tests ?? []).map(
                (requiredTest) => requiredTest.id
              )
            )
        );
        if (
          !valuesEqual(requiredTestClauseIds, expectedClauseIds) ||
          !valuesEqual(stateClauseIds, expectedClauseIds) ||
          !testManifestMatches
        )
          errors.push(
            issue(
              "V2_LIFECYCLE_ROW_MANIFEST",
              pointer,
              "Every requirement snapshot must retain the durable source's exact ordered clause and required-test manifests."
            )
          );
        const changedClauses = changedRowClauseIds(
          priorState,
          authorization.to_state
        );
        const reportClausesForRequirement = (report.covered_clause_ids ?? [])
          .filter((clauseId) =>
            clauseId.startsWith(`${authorization.entity_id}.C`)
          )
          .sort();
        if (
          !report.covered_requirement_ids?.includes(authorization.entity_id) ||
          !valuesEqual([...changedClauses].sort(), reportClausesForRequirement)
        )
          errors.push(
            issue(
              "V2_LIFECYCLE_ROW_SCOPE",
              pointer,
              "Requirement authorization report scope must contain the requirement and precisely its changed clauses."
            )
          );
        const fromSubject = priorState.rust_cutover?.implementation_subject_sha;
        const toSubject =
          authorization.to_state?.rust_cutover?.implementation_subject_sha;
        if (toSubject && authorization.subject_sha !== toSubject)
          errors.push(
            issue(
              "V2_LIFECYCLE_SUBJECT",
              pointer,
              "Requirement authorization subject must equal its non-null to-state implementation subject."
            )
          );
        if (fromSubject && !toSubject && authorization.operation !== "rollback")
          errors.push(
            issue(
              "V2_LIFECYCLE_SUBJECT",
              pointer,
              "Removing an implementation subject requires rollback."
            )
          );
      }
      if (new Set(["blocker", "architecture_decision"]).has(entityKind)) {
        const source = sourceEntityProjection;
        if (
          !(source?.affected_requirement_ids ?? []).every((id) =>
            report.covered_requirement_ids?.includes(id)
          ) ||
          !(source?.affected_clause_ids ?? []).every((id) =>
            report.covered_clause_ids?.includes(id)
          )
        )
          errors.push(
            issue(
              "V2_LIFECYCLE_ENTITY_SCOPE",
              pointer,
              "Blocker or decision authorization report must cover every affected requirement and clause."
            )
          );
      }
      priorState = authorization.to_state;
      priorDigest = toDigest;
      priorAuthorization = authorization;
      priorReviewedAt = reviewedAt;
      if (toDigest) seenDigests.add(toDigest);
    }
    if (!currentEntity)
      errors.push(
        issue(
          "V2_LIFECYCLE_ENTITY_MISSING",
          "/lifecycle/entity_chains",
          "An authorized lifecycle entity is absent from current state."
        )
      );
    else {
      try {
        if (
          priorDigest !== lifecycleDigestForEntity(entityKind, currentEntity) ||
          sourceDigest !==
            sha256(
              canonicalize(sourceProjectionForEntity(entityKind, currentEntity))
            )
        )
          errors.push(
            issue(
              "V2_LIFECYCLE_HEAD",
              "/lifecycle/entity_chains",
              "Lifecycle chain head or immutable source differs from current entity state."
            )
          );
      } catch (error) {
        errors.push(
          issue("V2_LIFECYCLE_HEAD", "/lifecycle/entity_chains", error.message)
        );
      }
    }
    latestAuthorizationByKey.set(key, entries.at(-1));
  }
  const timelineStates = new Map(
    [...baselineEntities].map(([key, entity]) => {
      const [kind] = key.split("\u0000");
      return [key, lifecycleProjectionForEntity(kind, entity)];
    })
  );
  const timelineSources = new Map(
    [...baselineEntities].map(([key, entity]) => {
      const [kind] = key.split("\u0000");
      return [key, sourceProjectionForEntity(kind, entity)];
    })
  );
  for (const { authorization, report } of allAuthorizations) {
    const key = entityKey(authorization.entity_kind, authorization.entity_id);
    const pointer = `/validation_reports/${reports.indexOf(
      report
    )}/lifecycle_authorizations/${(
      report.lifecycle_authorizations ?? []
    ).indexOf(authorization)}`;
    if (authorization.operation === "create")
      timelineSources.set(key, authorization.source_entity);
    timelineStates.set(key, authorization.to_state);
    if (
      authorization.entity_kind === "blocker" &&
      authorization.to_state?.status === "closed" &&
      (!(authorization.to_state.closure_evidence ?? []).length ||
        report.status !== "pass")
    )
      errors.push(
        issue(
          "V2_LIFECYCLE_BLOCKER_CLOSURE",
          pointer,
          "Every historical blocker-closing transition requires nonempty closure evidence in a passing report."
        )
      );
    if (
      authorization.entity_kind === "architecture_decision" &&
      new Set(["approved", "rejected", "superseded"]).has(
        authorization.to_state?.status
      )
    ) {
      const successorKey = entityKey(
        "architecture_decision",
        authorization.to_state?.superseded_by_id ?? ""
      );
      const successorState = timelineStates.get(successorKey);
      const validSupersession =
        authorization.to_state.status !== "superseded" ||
        (authorization.to_state.superseded_by_id !== authorization.entity_id &&
          new Set(["approved", "rejected"]).has(successorState?.status));
      if (
        !(authorization.to_state.closure_evidence ?? []).length ||
        !validSupersession
      )
        errors.push(
          issue(
            "V2_LIFECYCLE_DECISION_CLOSURE",
            pointer,
            "Every historical terminal decision requires closure evidence, and supersession must target an already terminal distinct successor."
          )
        );
    }
    if (
      authorization.entity_kind === "requirement" &&
      authorization.to_state?.rust_cutover?.readiness === "ready"
    ) {
      const state = authorization.to_state;
      const cutover = state.rust_cutover;
      const reportExecutions = new Map(
        (report.executions ?? []).map((execution) => [
          execution.test_case_id,
          execution,
        ])
      );
      const reportSnapshots = new Map(
        (report.test_contract_snapshots ?? []).map((snapshot) => [
          snapshot.test_id,
          snapshot,
        ])
      );
      const testsReady = (state.required_tests ?? []).every((group) =>
        (group.tests ?? []).every((requiredTest) => {
          const execution = reportExecutions.get(requiredTest.id);
          const snapshot = reportSnapshots.get(requiredTest.id);
          return (
            requiredTest.status === "accepted" &&
            requiredTest.execution_contract !== null &&
            valuesEqual(
              snapshot?.execution_contract ?? {},
              requiredTest.execution_contract
            ) &&
            execution?.result === "pass" &&
            execution.subject_sha === cutover.implementation_subject_sha
          );
        })
      );
      const clauseReportsReady = (state.clauses ?? []).every((clause) =>
        clause.validation_report_ids?.includes(report.id)
      );
      const rowClauseIds = new Set(
        (state.clauses ?? []).map((clause) => clause.clause_id)
      );
      const globallyBlocked = [...timelineSources].some(
        ([entityStateKey, source]) => {
          const [kind] = entityStateKey.split("\u0000");
          if (!new Set(["blocker", "architecture_decision"]).has(kind))
            return false;
          const affects =
            source?.affected_requirement_ids?.includes(
              authorization.entity_id
            ) ||
            source?.affected_clause_ids?.some((clauseId) =>
              rowClauseIds.has(clauseId)
            );
          if (!affects) return false;
          const status = timelineStates.get(entityStateKey)?.status;
          return kind === "blocker"
            ? isUnresolvedRiskStatus(status)
            : new Set(["unresolved", "proposed"]).has(status);
        }
      );
      if (
        report.status !== "pass" ||
        report.subject_sha !== cutover.implementation_subject_sha ||
        !testsReady ||
        !clauseReportsReady ||
        globallyBlocked ||
        (cutover.gap_ids ?? []).length ||
        (cutover.gate_ids ?? []).length ||
        (cutover.blocker_ids ?? []).length ||
        !new Set(["matrix_rust_sdk", "product_only", "none"]).has(
          cutover.matrix_owner
        ) ||
        cutover.surviving_raw_matrix_http !== false ||
        cutover.surviving_matrix_js_owner !== false
      )
        errors.push(
          issue(
            "V2_LIFECYCLE_HISTORICAL_READY",
            pointer,
            "Every historical ready transition requires contemporaneous passing contracts, safe ownership, no local residuals, and no globally affecting unresolved blocker or decision."
          )
        );
    }
  }
  for (const [key, entity] of currentEntities) {
    const baseline = baselineByKey.get(key);
    const entries = grouped.get(key) ?? [];
    if (!baseline && entries.length === 0)
      errors.push(
        issue(
          "V2_LIFECYCLE_UNAUTHORIZED_CREATE",
          "/lifecycle/entity_chains",
          "Every appended blocker or architecture decision requires a create authorization."
        )
      );
    if (baseline && entries.length === 0) {
      const [kind] = key.split("\u0000");
      if (
        lifecycleDigestForEntity(kind, entity) !==
        baseline.baseline_payload_sha256
      )
        errors.push(
          issue(
            "V2_LIFECYCLE_UNAUTHORIZED_STATE",
            "/lifecycle/entity_chains",
            "Lifecycle state differs from its durable baseline without authorization."
          )
        );
    }
  }
  try {
    if (
      !valuesEqual(
        value.lifecycle,
        deriveLifecycleManifest(
          value,
          durableAudit,
          value?.lifecycle?.previous_artifact ?? null
        )
      )
    )
      errors.push(
        issue(
          "V2_LIFECYCLE_MANIFEST",
          "/lifecycle",
          "Lifecycle manifest must be independently derived from source, state, reports, and authorizations."
        )
      );
  } catch (error) {
    errors.push(issue("V2_LIFECYCLE_MANIFEST", "/lifecycle", error.message));
  }
  return {
    errors,
    grouped,
    latestAuthorizationByKey,
    reportById,
    baselineByKey,
    currentEntities,
  };
}

function lifecyclePrefixErrors(previous, current) {
  const errors = [];
  const exactArrayPrefix = (prior, next) =>
    prior.length <= next.length &&
    prior.every((entry, index) => valuesEqual(entry, next[index]));
  for (const [field, prior, next] of [
    [
      "audit_reports",
      previous.audit_reports ?? [],
      current.audit_reports ?? [],
    ],
    [
      "validation_reports",
      previous.validation_reports ?? [],
      current.validation_reports ?? [],
    ],
  ])
    if (!exactArrayPrefix(prior, next))
      errors.push(
        issue(
          "V2_LIFECYCLE_REPORT_PREFIX",
          `/${field}`,
          "Historical durable report records must remain an exact ordered prefix."
        )
      );
  if (
    !exactArrayPrefix(
      previous.lifecycle?.audit_report_ids ?? [],
      current.lifecycle?.audit_report_ids ?? []
    ) ||
    !exactArrayPrefix(
      previous.lifecycle?.validation_report_ids ?? [],
      current.lifecycle?.validation_report_ids ?? []
    )
  )
    errors.push(
      issue(
        "V2_LIFECYCLE_REPORT_ID_PREFIX",
        "/lifecycle",
        "Lifecycle report ID manifests must retain exact ordered prefixes."
      )
    );
  const priorChains = previous.lifecycle?.entity_chains ?? [];
  const currentChains = current.lifecycle?.entity_chains ?? [];
  if (priorChains.length > currentChains.length)
    errors.push(
      issue(
        "V2_LIFECYCLE_CHAIN_PREFIX",
        "/lifecycle/entity_chains",
        "Lifecycle entity chains cannot be truncated."
      )
    );
  for (const [index, prior] of priorChains.entries()) {
    const next = currentChains[index];
    if (
      !next ||
      !valuesEqual(
        {
          entity_kind: prior.entity_kind,
          entity_id: prior.entity_id,
          projection: prior.projection,
          source_origin: prior.source_origin,
          source_entity_sha256: prior.source_entity_sha256,
          baseline_payload_sha256: prior.baseline_payload_sha256,
        },
        {
          entity_kind: next.entity_kind,
          entity_id: next.entity_id,
          projection: next.projection,
          source_origin: next.source_origin,
          source_entity_sha256: next.source_entity_sha256,
          baseline_payload_sha256: next.baseline_payload_sha256,
        }
      ) ||
      !exactArrayPrefix(
        prior.authorization_ids ?? [],
        next.authorization_ids ?? []
      )
    )
      errors.push(
        issue(
          "V2_LIFECYCLE_CHAIN_PREFIX",
          `/lifecycle/entity_chains/${index}`,
          "Historical chain identity and authorization IDs must remain exact ordered prefixes."
        )
      );
  }
  return errors;
}

function terminalIntroductionErrors(previous, current) {
  const errors = [];
  const priorCounts = new Map(
    (previous?.lifecycle?.entity_chains ?? []).map((chain) => [
      entityKey(chain.entity_kind, chain.entity_id),
      chain.authorization_ids?.length ?? 0,
    ])
  );
  const authorizationById = new Map(
    lifecycleAuthorizations(current).map((authorization) => [
      authorization.id,
      authorization,
    ])
  );
  for (const [chainIndex, chain] of (
    current?.lifecycle?.entity_chains ?? []
  ).entries()) {
    const priorCount =
      priorCounts.get(entityKey(chain.entity_kind, chain.entity_id)) ?? 0;
    const introducedIds = (chain.authorization_ids ?? []).slice(priorCount);
    for (const [index, authorizationId] of introducedIds.entries()) {
      const authorization = authorizationById.get(authorizationId);
      const terminal =
        (authorization?.entity_kind === "requirement" &&
          authorization.to_state?.rust_cutover?.readiness === "ready") ||
        (authorization?.entity_kind === "blocker" &&
          authorization.to_state?.status === "closed") ||
        (authorization?.entity_kind === "architecture_decision" &&
          new Set(["approved", "rejected", "superseded"]).has(
            authorization.to_state?.status
          ));
      if (terminal && index !== introducedIds.length - 1)
        errors.push(
          issue(
            "V2_LIFECYCLE_TERMINAL_INTRODUCTION",
            `/lifecycle/entity_chains/${chainIndex}/authorization_ids/${
              priorCount + index
            }`,
            "A terminal lifecycle claim must be the entity-chain head in the artifact where first introduced; rollback belongs in a later predecessor-linked artifact."
          )
        );
    }
  }
  return errors;
}

function exactIsoTimestamp(value) {
  if (
    typeof value !== "string" ||
    !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/u.test(value)
  )
    return false;
  const milliseconds = Date.parse(value);
  return (
    Number.isFinite(milliseconds) &&
    new Date(milliseconds).toISOString() === value
  );
}

export function executionResultFingerprint(execution) {
  const projection = durableExecutionProjection(execution);
  delete projection.result_fingerprint_sha256;
  return sha256(canonicalize(projection));
}

/**
 * Authoritative execution transcripts are deliberately not general-purpose
 * logs. This structural allowlist makes raw commands, output, prose, URLs,
 * Matrix identifiers, credentials, and local paths unrepresentable even when
 * schema validation is accidentally skipped by a caller.
 */
export function executionTranscriptPrivacyViolations(execution) {
  let projection;
  try {
    projection = durableExecutionProjection(execution);
  } catch {
    return [{ pointer: "", reason: "Execution semantics are not canonical." }];
  }
  if (
    !projection ||
    typeof projection !== "object" ||
    Array.isArray(projection)
  )
    return [{ pointer: "", reason: "Execution semantics must be an object." }];

  const violations = [];
  for (const key of Object.keys(projection)) {
    if (!EXECUTION_SEMANTIC_KEYS.has(key))
      violations.push({
        pointer: `/${key}`,
        reason: "Unallowlisted execution transcript field.",
      });
  }
  if (projection.schema_version !== "1.0")
    violations.push({
      pointer: "/schema_version",
      reason: "Execution schema version is not allowlisted.",
    });
  if (
    typeof projection.id !== "string" ||
    projection.id.length > 128 ||
    !EXECUTION_RECORD_ID.test(projection.id)
  )
    violations.push({ pointer: "/id", reason: "Unsafe execution record ID." });
  if (!EXECUTION_TEST_CASE_ID.test(projection.test_case_id ?? ""))
    violations.push({
      pointer: "/test_case_id",
      reason: "Execution test-case ID is not privacy-safe.",
    });
  if (!new Set(["command", "scenario"]).has(projection.kind))
    violations.push({
      pointer: "/kind",
      reason: "Execution kind is not allowlisted.",
    });
  if (!EXECUTION_RUNNERS.has(projection.runner_id))
    violations.push({
      pointer: "/runner_id",
      reason: "Execution runner is not allowlisted.",
    });
  if (!EXECUTION_INVOCATION_ID.test(projection.invocation_id ?? ""))
    violations.push({
      pointer: "/invocation_id",
      reason: "Execution invocation ID is not in the closed registry shape.",
    });
  if (!SAFE_SHA256.test(projection.invocation_fingerprint_sha256 ?? ""))
    violations.push({
      pointer: "/invocation_fingerprint_sha256",
      reason: "Invocation fingerprint must be a SHA-256 digest.",
    });
  if (!SAFE_SHA.test(projection.subject_sha ?? ""))
    violations.push({
      pointer: "/subject_sha",
      reason: "Execution subject must be a full commit SHA.",
    });
  if (!new Set(["pass", "fail", "blocked"]).has(projection.result))
    violations.push({
      pointer: "/result",
      reason: "Execution result is not allowlisted.",
    });
  if (
    !(
      projection.exit_code === null ||
      (Number.isInteger(projection.exit_code) &&
        projection.exit_code >= 0 &&
        projection.exit_code <= 255)
    )
  )
    violations.push({
      pointer: "/exit_code",
      reason: "Execution exit code is outside the closed numeric domain.",
    });
  for (const key of ["started_at", "finished_at"]) {
    if (!exactIsoTimestamp(projection[key]))
      violations.push({
        pointer: `/${key}`,
        reason: "Execution timestamp is not canonical UTC time.",
      });
  }
  if (!Array.isArray(projection.assertions))
    violations.push({
      pointer: "/assertions",
      reason: "Execution assertions must be structured outcomes.",
    });
  else
    for (const [index, assertion] of projection.assertions.entries()) {
      const pointer = `/assertions/${index}`;
      if (
        !assertion ||
        typeof assertion !== "object" ||
        Array.isArray(assertion)
      ) {
        violations.push({
          pointer,
          reason: "Execution assertion must be a structured outcome.",
        });
        continue;
      }
      if (
        Object.keys(assertion).some(
          (key) => key !== "assertion_id" && key !== "result"
        )
      )
        violations.push({
          pointer,
          reason: "Unallowlisted execution assertion field.",
        });
      if (!EXECUTION_ASSERTION_ID.test(assertion.assertion_id ?? ""))
        violations.push({
          pointer: `${pointer}/assertion_id`,
          reason: "Assertion identifier is not in the privacy-safe allowlist.",
        });
      if (!["pass", "fail", "blocked"].includes(assertion.result))
        violations.push({
          pointer: `${pointer}/result`,
          reason: "Assertion outcome is not allowlisted.",
        });
    }
  if (!SAFE_SHA256.test(projection.result_fingerprint_sha256 ?? ""))
    violations.push({
      pointer: "/result_fingerprint_sha256",
      reason: "Result fingerprint must be a SHA-256 digest.",
    });
  return violations;
}

async function verifyDurableExecution(
  execution,
  gitObjects,
  containingReportStorageCommit
) {
  const errors = [];
  try {
    const subject = await gitObjects.object(execution.subject_sha);
    if (!subject || subject.type !== "commit")
      errors.push(
        issue(
          "EXECUTION_SUBJECT_COMMIT",
          "/subject_sha",
          "Execution subject is missing or is not a commit."
        )
      );
    else if (
      typeof gitObjects.commitIsAncestorOfHead !== "function" ||
      !gitObjects.commitIsAncestorOfHead(execution.subject_sha)
    )
      errors.push(
        issue(
          "EXECUTION_SUBJECT_HISTORY",
          "/subject_sha",
          "Execution subject commit is not an ancestor of current HEAD."
        )
      );
    if (
      typeof gitObjects.commitIsAncestorOf !== "function" ||
      !gitObjects.commitIsAncestorOf(
        execution.subject_sha,
        execution.storage_commit_sha
      )
    )
      errors.push(
        issue(
          "EXECUTION_SUBJECT_STORAGE_HISTORY",
          "/subject_sha",
          "Execution subject commit is not an ancestor of its transcript storage commit."
        )
      );
    if (
      typeof gitObjects.commitIsAncestorOf !== "function" ||
      !gitObjects.commitIsAncestorOf(
        execution.storage_commit_sha,
        containingReportStorageCommit
      )
    )
      errors.push(
        issue(
          "EXECUTION_REPORT_HISTORY",
          "/storage_commit_sha",
          "Execution transcript storage commit is not an ancestor of its containing validation report storage commit."
        )
      );
    if (
      typeof gitObjects.workingTreePathIsClean !== "function" ||
      !gitObjects.workingTreePathIsClean(execution.path)
    )
      errors.push(
        issue(
          "EXECUTION_WORKTREE_STATE",
          "/path",
          "Execution transcript path is dirty or untracked."
        )
      );
    const resolved = await gitObjects.resolvePath(
      execution.storage_commit_sha,
      execution.path
    );
    if (!resolved)
      return [
        ...errors,
        issue(
          "EXECUTION_MISSING",
          "/path",
          "Execution transcript is missing at its storage commit."
        ),
      ];
    if (resolved.oid !== execution.storage_blob_oid)
      errors.push(
        issue(
          "EXECUTION_BLOB_OID",
          "/storage_blob_oid",
          "Execution transcript blob OID differs."
        )
      );
    if (typeof gitObjects.durablePathIsRetained !== "function")
      errors.push(
        issue(
          "EXECUTION_HISTORY",
          "/storage_commit_sha",
          "Git object adapter cannot prove execution transcript history."
        )
      );
    else {
      const retained = await gitObjects.durablePathIsRetained(
        execution.storage_commit_sha,
        execution.path,
        execution.storage_blob_oid
      );
      if (!retained.ancestor)
        errors.push(
          issue(
            "EXECUTION_HISTORY",
            "/storage_commit_sha",
            "Execution transcript storage commit is not an ancestor of current HEAD."
          )
        );
      if (!retained.currentBlobMatches)
        errors.push(
          issue(
            "EXECUTION_HEAD_BLOB",
            "/path",
            "Current HEAD does not retain the exact execution transcript blob."
          )
        );
    }
    const bytes = await gitObjects.bytes(resolved.oid);
    if (sha256(bytes) !== execution.file_sha256)
      errors.push(
        issue(
          "EXECUTION_FILE_DIGEST",
          "/file_sha256",
          "Execution transcript exact-byte SHA-256 differs."
        )
      );
    let payload;
    try {
      payload = parseCanonicalJson(bytes);
      if (!bytes.equals(Buffer.from(prettyJson(payload))))
        throw new SyntaxError(
          "Execution transcript must use canonical committed pretty-JSON bytes."
        );
    } catch (error) {
      errors.push(issue("EXECUTION_JSON", "/path", error.message));
      return errors;
    }
    if (sha256(canonicalize(payload)) !== execution.canonical_sha256)
      errors.push(
        issue(
          "EXECUTION_CANONICAL_DIGEST",
          "/canonical_sha256",
          "Execution transcript canonical semantic SHA-256 differs."
        )
      );
    if (!valuesEqual(payload, durableExecutionProjection(execution)))
      errors.push(
        issue(
          "EXECUTION_PAYLOAD",
          "/path",
          "Execution transcript differs from the deterministic embedded-record projection."
        )
      );
    if (executionTranscriptPrivacyViolations(payload).length > 0)
      errors.push(
        issue(
          "EXECUTION_PRIVACY",
          "/path",
          "Execution transcript contains content outside the privacy-safe structural allowlist."
        )
      );
    else if (
      payload.result_fingerprint_sha256 !== executionResultFingerprint(payload)
    )
      errors.push(
        issue(
          "EXECUTION_RESULT_FINGERPRINT",
          "/result_fingerprint_sha256",
          "Execution result fingerprint differs from the canonical secret-free transcript semantics."
        )
      );
  } catch (error) {
    errors.push(issue("EXECUTION_GIT", "/path", error.message));
  }
  return errors;
}

export function normalizeRiskSeverity(value) {
  if (typeof value !== "string") return null;
  if (value === "critical" || value.startsWith("critical_")) return "critical";
  return new Set(["high", "medium", "low", "informational"]).has(value)
    ? value
    : null;
}

export function normalizeRiskStatus(value) {
  return (
    {
      open: "open",
      mitigating: "mitigating",
      blocked_on_decision: "blocked",
      blocked: "blocked",
      accepted: "accepted_risk",
      accepted_risk: "accepted_risk",
      closed: "closed",
    }[value] ?? null
  );
}

export function isUnresolvedRiskStatus(value) {
  return new Set(["open", "mitigating", "blocked", "accepted_risk"]).has(value);
}

async function verifyRiskAuthority(identity, projectedRisks, gitObjects) {
  const errors = [];
  const expectedIdentity = {
    commit_sha: RISK_REGISTER_COMMIT,
    path: RISK_REGISTER_PATH,
    blob_oid: RISK_REGISTER_BLOB,
    file_sha256: RISK_REGISTER_SHA256,
    canonical_sha256: RISK_REGISTER_CANONICAL_SHA256,
  };
  if (!valuesEqual(identity ?? {}, expectedIdentity))
    errors.push(
      issue(
        "RISK_REGISTER_IDENTITY",
        "/",
        "Security risk-register Git identity differs."
      )
    );
  try {
    if (
      typeof gitObjects.commitIsAncestorOfHead !== "function" ||
      !gitObjects.commitIsAncestorOfHead(RISK_REGISTER_COMMIT)
    )
      errors.push(
        issue(
          "RISK_REGISTER_HISTORY",
          "/commit_sha",
          "Current HEAD does not descend from the pinned security risk-register commit."
        )
      );
    const resolved = await gitObjects.resolvePath(
      RISK_REGISTER_COMMIT,
      RISK_REGISTER_PATH
    );
    if (!resolved)
      return [
        ...errors,
        issue(
          "RISK_REGISTER_MISSING",
          "/path",
          "Pinned security risk register is missing."
        ),
      ];
    if (resolved.oid !== RISK_REGISTER_BLOB)
      errors.push(
        issue(
          "RISK_REGISTER_BLOB",
          "/blob_oid",
          "Security risk-register blob OID differs."
        )
      );
    const bytes = await gitObjects.bytes(resolved.oid);
    if (sha256(bytes) !== RISK_REGISTER_SHA256)
      errors.push(
        issue(
          "RISK_REGISTER_DIGEST",
          "/file_sha256",
          "Security risk-register exact-byte digest differs."
        )
      );
    const authority = parseCanonicalJson(bytes);
    if (
      sha256(canonicalize(authority)) !== RISK_REGISTER_CANONICAL_SHA256 ||
      authority.authoritative !== true
    )
      errors.push(
        issue(
          "RISK_REGISTER_CANONICAL",
          "/",
          "Security risk-register canonical semantic identity differs."
        )
      );
    const authorityRisks = authority.risks ?? [];
    const sourceById = new Map(
      authorityRisks.map((risk) => [risk.risk_id, risk])
    );
    const projectedById = new Map(
      (projectedRisks ?? []).map((risk) => [risk.id, risk])
    );
    for (const [riskId, expected] of Object.entries(
      EXPECTED_CENTRAL_RISK_CONTRACT
    )) {
      const sourceMatches = authorityRisks.filter(
        (risk) => risk.risk_id === riskId
      );
      const source = sourceById.get(riskId);
      const projected = projectedById.get(riskId);
      const derived = source && {
        severity: normalizeRiskSeverity(source.inherent_severity),
        status: normalizeRiskStatus(source.status),
        owner_task_ids: source.owning_tasks,
        threat_ids: source.threat_ids,
        boundary_ids: riskId === "MRSDK-R038" ? ["B09"] : [],
        closure_criteria: [source.closure_criteria],
      };
      if (
        sourceMatches.length !== 1 ||
        !valuesEqual(derived ?? {}, expected) ||
        !projected ||
        !valuesEqual(
          {
            severity: projected.severity,
            status: projected.status,
            owner_task_ids: projected.owner_task_ids,
            threat_ids: projected.threat_ids,
            boundary_ids: projected.boundary_ids,
            closure_criteria: projected.closure_criteria,
          },
          derived
        )
      )
        errors.push(
          issue(
            "RISK_REGISTER_CONTRACT",
            "/",
            `${riskId} differs from the uniquely derived authoritative record.`
          )
        );
    }
  } catch (error) {
    errors.push(issue("RISK_REGISTER_GIT", "/", error.message));
  }
  return errors;
}

function semanticAuditErrors(value) {
  const errors = [];
  const rows = value?.rows ?? [];
  const requiredTestEntries = requiredTestRegistryEntries(rows, "rows");
  errors.push(
    ...requiredTestRegistryErrors(requiredTestEntries, {
      duplicateCode: "AUDIT_REQUIRED_TEST_IDS_GLOBAL",
      duplicatePointer: "/rows",
      contractCode: "AUDIT_REQUIRED_TEST_CONTRACT",
      sourceAudit: true,
    })
  );
  errors.push(...preMigrationAuthorityErrors(rows));
  if (rows.length !== EXPECTED_REQUIREMENT_COUNT)
    errors.push(
      issue("AUDIT_ROW_COUNT", "/rows", "Audit must contain exactly 119 rows.")
    );
  const ids = rows.map((row) => row.id);
  if (
    duplicates(ids).length ||
    ids.some((id) => !SAFE_REQUIREMENT.test(id ?? ""))
  )
    errors.push(
      issue(
        "AUDIT_ROW_IDS",
        "/rows",
        "Requirement IDs must be valid and globally unique."
      )
    );
  if (!valuesEqual(ids, EXPECTED_REQUIREMENT_IDS))
    errors.push(
      issue(
        "AUDIT_ROW_MANIFEST",
        "/rows",
        "Requirement IDs and order differ from the exact Section 7 manifest."
      )
    );
  if (!valuesEqual(sectionCounts(rows), EXPECTED_SECTION_COUNTS))
    errors.push(
      issue(
        "AUDIT_SECTION_COUNTS",
        "/coverage/section_counts",
        "Section counts differ from the frozen contract."
      )
    );
  if (
    value?.coverage?.expected_row_count !== EXPECTED_REQUIREMENT_COUNT ||
    value?.coverage?.actual_row_count !== rows.length ||
    value?.coverage?.unique_requirement_count !== new Set(ids).size ||
    value?.coverage?.requirement_id_sha256 !== sha256(canonicalize(ids))
  )
    errors.push(
      issue(
        "AUDIT_COVERAGE",
        "/coverage",
        "Coverage totals and ordered requirement digest must be derived from rows."
      )
    );
  if (value?.coverage?.status_correction_count !== EXPECTED_STATUS_CORRECTIONS)
    errors.push(
      issue(
        "AUDIT_CORRECTIONS",
        "/coverage/status_correction_count",
        "Total status correction count must be 23."
      )
    );
  const derivedCorrections = changedStatusCorrections(rows);
  if (
    derivedCorrections.length !== EXPECTED_STATUS_CORRECTIONS ||
    !correctionManifestIsValid(rows) ||
    derivedCorrections.some(
      (row) =>
        row.current_product.status_correction.audited_status !==
        row.current_product.status
    )
  )
    errors.push(
      issue(
        "AUDIT_CORRECTIONS",
        "/rows",
        "Exactly 23 internally consistent status corrections must be derived from the rows."
      )
    );
  const derivedSubtotal = statusCorrectionSubtotal(rows);
  if (
    derivedSubtotal !== EXPECTED_73_77_SUBTOTAL ||
    value?.coverage?.sections_7_3_through_7_7_status_correction_subtotal !==
      derivedSubtotal
  )
    errors.push(
      issue(
        "AUDIT_SUBTOTAL",
        "/coverage/sections_7_3_through_7_7_status_correction_subtotal",
        "7.3-7.7 status-correction subtotal must be derived from the exact transition manifest."
      )
    );
  if (
    !valuesEqual(
      value?.coverage
        ?.sections_7_3_through_7_7_non_status_correction_manifest ?? [],
      EXPECTED_73_77_NON_STATUS_CORRECTIONS
    )
  )
    errors.push(
      issue(
        "AUDIT_NON_STATUS_CORRECTIONS",
        "/coverage/sections_7_3_through_7_7_non_status_correction_manifest",
        "The five recovered evidence/test-ledger corrections must retain their exact classified manifest."
      )
    );
  const identifiedCorrectionCount =
    derivedSubtotal + EXPECTED_73_77_NON_STATUS_CORRECTIONS.length;
  if (
    identifiedCorrectionCount !== EXPECTED_73_77_IDENTIFIED_CORRECTION_COUNT ||
    value?.coverage?.sections_7_3_through_7_7_identified_correction_count !==
      identifiedCorrectionCount
  )
    errors.push(
      issue(
        "AUDIT_IDENTIFIED_CORRECTIONS",
        "/coverage/sections_7_3_through_7_7_identified_correction_count",
        "The broader identified-correction count must equal 11 status transitions plus five classified evidence/test-ledger corrections."
      )
    );
  if (
    (value?.architecture_decisions ?? []).length !==
    EXPECTED_ARCHITECTURE_DECISIONS
  )
    errors.push(
      issue(
        "AUDIT_DECISIONS",
        "/architecture_decisions",
        "Exactly 26 architecture decisions must be retained."
      )
    );
  if (value?.subject?.source_commit !== AUDITED_SOURCE_COMMIT)
    errors.push(
      issue(
        "AUDIT_SUBJECT",
        "/subject/source_commit",
        "Audit subject SHA differs."
      )
    );
  if (
    value?.subject?.plan?.path !== AUDITED_PLAN_PATH ||
    value?.subject?.plan?.blob_oid !== AUDITED_PLAN_BLOB ||
    value?.subject?.plan?.file_sha256 !== AUDITED_PLAN_SHA256
  )
    errors.push(
      issue(
        "AUDIT_PLAN_IDENTITY",
        "/subject/plan",
        "Audited plan path, blob, and exact-byte digest must remain pinned."
      )
    );
  const expectedRiskRegisterIdentity = {
    commit_sha: RISK_REGISTER_COMMIT,
    path: RISK_REGISTER_PATH,
    blob_oid: RISK_REGISTER_BLOB,
    file_sha256: RISK_REGISTER_SHA256,
    canonical_sha256: RISK_REGISTER_CANONICAL_SHA256,
  };
  if (
    !valuesEqual(
      value?.subject?.risk_register ?? {},
      expectedRiskRegisterIdentity
    )
  )
    errors.push(
      issue(
        "AUDIT_RISK_PROVENANCE",
        "/subject/risk_register",
        "Authoritative security risk-register identity differs."
      )
    );
  if (!valuesEqual(value?.source_inputs ?? [], SOURCE_INPUTS))
    errors.push(
      issue(
        "AUDIT_SOURCE_INPUT",
        "/source_inputs",
        "The complete ordered immutable source-input manifest differs."
      )
    );
  const serialized = JSON.stringify(value);
  if (PROHIBITED_SERIALIZED_PATH.test(serialized))
    errors.push(
      issue(
        "AUDIT_EPHEMERAL_PATH",
        "/",
        "Committed audit contains an ephemeral or local reference."
      )
    );
  const expectedRequirementDigest = sha256(canonicalize([...ids].sort()));
  if (value?.digests?.requirement_id_sha256 !== expectedRequirementDigest)
    errors.push(
      issue(
        "AUDIT_REQUIREMENT_DIGEST",
        "/digests/requirement_id_sha256",
        "Requirement-ID digest differs."
      )
    );
  const clauseIds = rows.flatMap((row) =>
    (row.clauses ?? []).map((clause) => clause.id)
  );
  if (duplicates(clauseIds).length)
    errors.push(
      issue("AUDIT_CLAUSE_IDS", "/rows", "Clause IDs must be globally unique.")
    );
  if (
    rows.some((row) =>
      (row.clauses ?? []).some((clause) => !clause.id.startsWith(`${row.id}.C`))
    )
  )
    errors.push(
      issue(
        "AUDIT_CLAUSE_OWNERSHIP",
        "/rows",
        "Every clause ID must be namespaced to its owning requirement."
      )
    );
  if (
    value?.digests?.clause_id_sha256 !==
    sha256(canonicalize([...clauseIds].sort()))
  )
    errors.push(
      issue(
        "AUDIT_CLAUSE_DIGEST",
        "/digests/clause_id_sha256",
        "Clause-ID digest differs."
      )
    );
  errors.push(...evidenceIntegrityErrors(rows, "/rows"));
  try {
    const payload = cloneWithoutPointers(value, [
      "/review",
      "/digests/canonical_payload_sha256",
    ]);
    if (
      value?.digests?.canonical_payload_sha256 !== sha256(canonicalize(payload))
    )
      errors.push(
        issue(
          "AUDIT_PAYLOAD_DIGEST",
          "/digests/canonical_payload_sha256",
          "Audit canonical payload digest differs."
        )
      );
  } catch (error) {
    errors.push(issue("AUDIT_PAYLOAD_DIGEST", "/digests", error.message));
  }
  const riskById = new Map(
    (value?.blockers_and_risks ?? []).map((risk) => [
      risk.id ?? risk.risk_id,
      risk,
    ])
  );
  if (riskById.size !== (value?.blockers_and_risks ?? []).length)
    errors.push(
      issue(
        "AUDIT_RISK_IDS",
        "/blockers_and_risks",
        "Blocker and risk IDs must be globally unique."
      )
    );
  for (const riskId of [
    "MRSDK-R024",
    "MRSDK-R027",
    "MRSDK-R030",
    "MRSDK-R036",
    "MRSDK-R037",
    "MRSDK-R038",
  ])
    if (!riskById.has(riskId))
      errors.push(
        issue(
          "AUDIT_RISK_MAPPING",
          "/blockers_and_risks",
          `Missing central risk mapping ${riskId}.`
        )
      );
  for (const [riskId, expected] of Object.entries(
    EXPECTED_CENTRAL_RISK_CONTRACT
  )) {
    const risk = riskById.get(riskId);
    if (
      risk &&
      (String(risk.severity).toLowerCase() !== expected.severity ||
        String(risk.status).toLowerCase() !== expected.status ||
        !valuesEqual(risk.owner_task_ids ?? [], expected.owner_task_ids) ||
        !valuesEqual(risk.threat_ids ?? [], expected.threat_ids) ||
        !valuesEqual(risk.boundary_ids ?? [], expected.boundary_ids) ||
        !valuesEqual(risk.closure_criteria ?? [], expected.closure_criteria))
    )
      errors.push(
        issue(
          `AUDIT_${riskId.slice(-4)}`,
          "/blockers_and_risks",
          `${riskId} differs from the exact authoritative risk contract.`
        )
      );
  }
  const forwardingRisk = riskById.get("MRSDK-R036");
  if (
    forwardingRisk &&
    (!(forwardingRisk.affected_requirement_ids ?? []).includes("FR-7.4-011") ||
      riskById.has("FR-7.4-011"))
  )
    errors.push(
      issue(
        "AUDIT_FR_7_4_011_RISK",
        "/blockers_and_risks",
        "FR-7.4-011 must be a requirement reference owned by MRSDK-R036, never a blocker identity."
      )
    );
  if (
    forwardingRisk &&
    /(?:observed|confirmed|proven).{0,80}(?:plaintext|megolm|session[-_ ]?key|attachment[-_ ]?key).{0,40}(?:leak|exfiltrat)/iu.test(
      JSON.stringify(forwardingRisk)
    )
  )
    errors.push(
      issue(
        "AUDIT_R036_CLAIM",
        "/blockers_and_risks",
        "MRSDK-R036 is a forwarding correctness and metadata-privacy risk, not evidence of a plaintext or key leak."
      )
    );
  if (
    (value?.architecture_decisions ?? []).some(
      (decision) => decision.status !== "unresolved"
    )
  )
    errors.push(
      issue(
        "AUDIT_DECISION_STATE",
        "/architecture_decisions",
        "All 26 source-audit architecture decisions remain unresolved."
      )
    );
  if (
    !valuesEqual(
      (value?.architecture_decisions ?? []).map((decision) => decision.id),
      Array.from(
        { length: EXPECTED_ARCHITECTURE_DECISIONS },
        (_, index) => `AD-${String(index + 1).padStart(2, "0")}`
      )
    )
  )
    errors.push(
      issue(
        "AUDIT_DECISION_MANIFEST",
        "/architecture_decisions",
        "Architecture decision IDs and order must be AD-01 through AD-26."
      )
    );
  for (const [index, row] of rows.entries()) {
    try {
      if (row.audit?.audited_payload_sha256 !== auditedRowDigest(row))
        errors.push(
          issue(
            "AUDIT_ROW_DIGEST",
            `/rows/${index}/audit/audited_payload_sha256`,
            "Audited row payload digest differs."
          )
        );
    } catch (error) {
      errors.push(
        issue("AUDIT_ROW_DIGEST", `/rows/${index}/audit`, error.message)
      );
    }
    if (row.current_product?.status === "implemented") {
      const incomplete = (row.clauses ?? []).some(
        (clause) => clause.current_product_status !== "implemented"
      );
      if (incomplete)
        errors.push(
          issue(
            "AUDIT_ROLLUP",
            `/rows/${index}/current_product/status`,
            "Implemented row contains an incomplete clause."
          )
        );
    }
    if (row.rust_cutover?.readiness === "ready")
      errors.push(
        issue(
          "AUDIT_RUST_READY",
          `/rows/${index}/rust_cutover/readiness`,
          "R0.2 source audit cannot establish Rust-ready status."
        )
      );
    if (row.id === "FR-7.4-011") {
      const rowText = JSON.stringify(row);
      if (
        !/medium/iu.test(rowText) ||
        !rowText.includes("event.isEncrypted() && !event.getClearContent()") ||
        !rowText.includes("event.isDecryptionFailure()")
      )
        errors.push(
          issue(
            "AUDIT_FR_7_4_011",
            `/rows/${index}`,
            "Forwarding finding must retain Medium semantics and both rejection predicates."
          )
        );
    }
  }
  return errors;
}

export async function validateAudit(
  value,
  {
    gitObjects,
    repoRoot = defaultRepositoryRoot(),
    skipGitEvidence = false,
  } = {}
) {
  try {
    assertCanonicalValue(value);
  } catch (error) {
    return [issue("CANONICAL_VALUE", "/", error.message, AUDIT_PATH)];
  }
  const schemaErrors = validateSchema("audit", value, {
    repositoryRoot: repoRoot,
  });
  let semanticErrors;
  try {
    semanticErrors = semanticAuditErrors(value);
  } catch {
    semanticErrors = [
      issue(
        "AUDIT_SEMANTIC_INPUT",
        "/",
        "Semantic validation could not process structurally invalid input."
      ),
    ];
  }
  const errors = [...schemaErrors, ...semanticErrors];
  if (!skipGitEvidence && schemaErrors.length === 0) {
    if (!gitObjects) throw new TypeError("Git object adapter is required.");
    errors.push(
      ...(
        await verifyRiskAuthority(
          value.subject?.risk_register,
          value.blockers_and_risks,
          gitObjects
        )
      ).map((entry) => ({
        ...entry,
        pointer: `/subject/risk_register${entry.pointer}`,
      }))
    );
    for (
      let rowIndex = 0;
      rowIndex < (value.rows ?? []).length;
      rowIndex += 1
    ) {
      const row = value.rows[rowIndex];
      for (
        let clauseIndex = 0;
        clauseIndex < (row.clauses ?? []).length;
        clauseIndex += 1
      ) {
        const clause = row.clauses[clauseIndex];
        for (
          let index = 0;
          index < (clause.source_evidence ?? []).length;
          index += 1
        )
          errors.push(
            ...(
              await verifySourceEvidence(
                clause.source_evidence[index],
                gitObjects
              )
            ).map((entry) => ({
              ...entry,
              pointer: `/rows/${rowIndex}/clauses/${clauseIndex}/source_evidence/${index}${entry.pointer}`,
            }))
          );
        for (
          let index = 0;
          index < (clause.absence_evidence ?? []).length;
          index += 1
        )
          errors.push(
            ...(
              await verifyAbsenceEvidence(
                clause.absence_evidence[index],
                gitObjects
              )
            ).map((entry) => ({
              ...entry,
              pointer: `/rows/${rowIndex}/clauses/${clauseIndex}/absence_evidence/${index}${entry.pointer}`,
            }))
          );
        for (
          let index = 0;
          index < (clause.existing_tests ?? []).length;
          index += 1
        )
          errors.push(
            ...(
              await verifySourceEvidence(
                clause.existing_tests[index],
                gitObjects
              )
            ).map((entry) => ({
              ...entry,
              pointer: `/rows/${rowIndex}/clauses/${clauseIndex}/existing_tests/${index}${entry.pointer}`,
            }))
          );
      }
    }
  }
  return sortDiagnostics(errors);
}

function semanticV2Errors(value, durableAuditIdentity, durableAudit) {
  const errors = [];
  const requirements = value?.requirements ?? [];
  const durableRowById = new Map(
    (durableAudit?.rows ?? []).map((row) => [row.id, row])
  );
  if (requirements.length !== EXPECTED_REQUIREMENT_COUNT)
    errors.push(
      issue(
        "V2_ROW_COUNT",
        "/requirements",
        "V2 must contain exactly 119 requirements."
      )
    );
  if (!valuesEqual(sectionCounts(requirements), EXPECTED_SECTION_COUNTS))
    errors.push(
      issue(
        "V2_SECTION_COUNTS",
        "/coverage_contract/section_counts",
        "V2 section counts differ."
      )
    );
  const requirementIds = requirements.map((row) => row.id);
  if (!valuesEqual(requirementIds, EXPECTED_REQUIREMENT_IDS))
    errors.push(
      issue(
        "V2_ROW_MANIFEST",
        "/requirements",
        "Requirement IDs and order differ from the exact Section 7 manifest."
      )
    );
  if (!correctionManifestIsValid(requirements))
    errors.push(
      issue(
        "V2_CORRECTIONS",
        "/requirements",
        "The exact 23 pinned status-correction transitions must be retained."
      )
    );
  if (
    requirements.some((row) =>
      (row.clauses ?? []).some((clause) => !clause.id.startsWith(`${row.id}.C`))
    )
  )
    errors.push(
      issue(
        "V2_CLAUSE_OWNERSHIP",
        "/requirements",
        "Every clause ID must be namespaced to its owning requirement."
      )
    );
  errors.push(...evidenceIntegrityErrors(requirements, "/requirements"));
  if (durableAuditIdentity) {
    for (const [field, expected] of Object.entries(durableAuditIdentity)) {
      if (value?.source_audit?.artifact?.[field] !== expected)
        errors.push(
          issue(
            "V2_AUDIT_IDENTITY",
            `/source_audit/artifact/${field}`,
            "Durable audit identity differs."
          )
        );
    }
  } else {
    errors.push(
      issue(
        "V2_AUDIT_IDENTITY",
        "/source_audit/artifact",
        "V2 validation requires an independently derived durable-audit identity."
      )
    );
  }
  if (!durableAudit) {
    errors.push(
      issue(
        "V2_AUDIT_BINDING",
        "/source_audit/artifact",
        "V2 validation requires the exact durable audit payload."
      )
    );
  } else {
    if (
      durableAuditIdentity &&
      (durableAuditIdentity.canonical_semantic_sha256 !==
        sha256(canonicalize(durableAudit)) ||
        durableAuditIdentity.file_sha256 !==
          sha256(Buffer.from(prettyJson(durableAudit))))
    )
      errors.push(
        issue(
          "V2_AUDIT_BINDING",
          "/source_audit/artifact",
          "Durable-audit bytes and semantic payload differ from the derived immutable identity."
        )
      );
    if (
      value?.source_audit?.audit_id !== durableAudit.audit_id ||
      value?.source_audit?.subject_sha !==
        durableAudit.subject?.source_commit ||
      value?.source_audit?.canonical_payload_sha256 !==
        durableAudit.digests?.canonical_payload_sha256 ||
      value?.source_audit?.review_state !== durableAudit.review?.state
    )
      errors.push(
        issue(
          "V2_AUDIT_BINDING",
          "/source_audit",
          "V2 source-audit metadata differs from the durable audit."
        )
      );
    for (const [index, row] of requirements.entries()) {
      const durableRow = durableRowById.get(row.id);
      try {
        if (
          !durableRow ||
          !valuesEqual(
            sourceAuditRowProjection(row),
            sourceAuditRowProjection(durableRow)
          )
        )
          errors.push(
            issue(
              "V2_AUDIT_ROW_BINDING",
              `/requirements/${index}`,
              "Immutable source-audit row projection differs from its durable-audit row."
            )
          );
      } catch (error) {
        errors.push(
          issue("V2_AUDIT_ROW_BINDING", `/requirements/${index}`, error.message)
        );
      }
    }
  }
  const serialized = JSON.stringify(value);
  if (PROHIBITED_SERIALIZED_PATH.test(serialized))
    errors.push(
      issue(
        "V2_EPHEMERAL_PATH",
        "/",
        "V2 contains an ephemeral or local reference."
      )
    );
  const clauseIds = requirements.flatMap((row) =>
    (row.clauses ?? []).map((clause) => clause.id)
  );
  const expectedCoverage = {
    requirement_count: EXPECTED_REQUIREMENT_COUNT,
    section_counts: EXPECTED_SECTION_COUNTS,
    requirement_id_sha256: sha256(canonicalize([...requirementIds].sort())),
    clause_id_sha256: sha256(canonicalize([...clauseIds].sort())),
    status_correction_count: changedStatusCorrections(requirements).length,
    sections_7_3_through_7_7_status_correction_subtotal:
      statusCorrectionSubtotal(requirements),
    sections_7_3_through_7_7_non_status_correction_manifest: deepCopy(
      EXPECTED_73_77_NON_STATUS_CORRECTIONS
    ),
    sections_7_3_through_7_7_identified_correction_count:
      statusCorrectionSubtotal(requirements) +
      EXPECTED_73_77_NON_STATUS_CORRECTIONS.length,
  };
  if (!valuesEqual(value?.coverage_contract ?? {}, expectedCoverage))
    errors.push(
      issue(
        "V2_COVERAGE",
        "/coverage_contract",
        "Coverage counts and digests must be derived from the exact requirement manifest."
      )
    );
  const statusVocabulary = [
    "implemented",
    "partial",
    "missing",
    "not_exposed",
    "dead_path_proven",
    "unverified",
  ];
  const readinessVocabulary = [
    "not_assessed",
    "blocked",
    "implementation_planned",
    "implementation_in_progress",
    "validation_pending",
    "ready",
  ];
  const expectedSummary = {
    requirement_count: requirements.length,
    clause_count: clauseIds.length,
    current_product_status_counts: countsBy(
      requirements,
      (row) => row.current_product?.status,
      statusVocabulary
    ),
    rust_cutover_readiness_counts: countsBy(
      requirements,
      (row) => row.rust_cutover?.readiness,
      readinessVocabulary
    ),
    open_blocker_count: (value?.blockers ?? []).filter((blocker) =>
      isUnresolvedRiskStatus(blocker.status)
    ).length,
    unresolved_critical_high_blocker_count: (value?.blockers ?? []).filter(
      (blocker) =>
        isUnresolvedRiskStatus(blocker.status) &&
        new Set(["critical", "high"]).has(blocker.severity)
    ).length,
    open_architecture_decision_count: (
      value?.architecture_decisions ?? []
    ).filter(
      (decision) =>
        decision.status !== "approved" &&
        decision.status !== "rejected" &&
        decision.status !== "superseded"
    ).length,
    accepted_audit_report_count: (value?.audit_reports ?? []).filter(
      (report) => report.verdict === "accept"
    ).length,
    accepted_validation_report_count: (value?.validation_reports ?? []).filter(
      (report) => report.status === "pass"
    ).length,
    derived: true,
  };
  if (!valuesEqual(value?.summary ?? {}, expectedSummary))
    errors.push(
      issue(
        "V2_SUMMARY",
        "/summary",
        "Summary values must be derived from authoritative v2 records."
      )
    );
  const provenance = value?.provenance;
  const migrationSource = provenance?.migration_source;
  const riskRegister = provenance?.risk_register;
  if (
    provenance?.plan?.path !== AUDITED_PLAN_PATH ||
    provenance?.plan?.blob_oid !== CURRENT_PLAN_BLOB ||
    provenance?.plan?.file_sha256 !== CURRENT_PLAN_SHA256 ||
    migrationSource?.commit_sha !== AUDITED_SOURCE_COMMIT ||
    migrationSource?.path !== V1_PATH ||
    migrationSource?.blob_oid !== V1_BLOB ||
    migrationSource?.file_sha256 !== V1_SHA256 ||
    riskRegister?.commit_sha !== RISK_REGISTER_COMMIT ||
    riskRegister?.path !== RISK_REGISTER_PATH ||
    riskRegister?.blob_oid !== RISK_REGISTER_BLOB ||
    riskRegister?.file_sha256 !== RISK_REGISTER_SHA256 ||
    riskRegister?.canonical_sha256 !== RISK_REGISTER_CANONICAL_SHA256
  )
    errors.push(
      issue(
        "V2_PROVENANCE",
        "/provenance",
        "Plan, migration-source, and security risk-register identities must remain pinned."
      )
    );
  const legacy = value?.migration_provenance?.legacy_v1;
  if (
    legacy?.summary_canonical_sha256 !== V1_SUMMARY_CANONICAL_SHA256 ||
    legacy?.requirements_canonical_sha256 !==
      V1_REQUIREMENTS_CANONICAL_SHA256 ||
    !legacy?.provenance ||
    sha256(canonicalize(legacy.provenance)) !==
      V1_PROVENANCE_CANONICAL_SHA256 ||
    !legacy?.vocabularies ||
    sha256(canonicalize(legacy.vocabularies)) !==
      V1_VOCABULARIES_CANONICAL_SHA256 ||
    !valuesEqual(
      value?.migration_provenance?.transformed_keys ?? [],
      TRANSFORMED_V1_KEYS
    ) ||
    !valuesEqual(
      value?.migration_provenance?.added_keys ?? [],
      ADDED_V2_KEYS
    ) ||
    !valuesEqual(
      value?.migration_provenance?.preserved_keys ?? [],
      PRESERVED_V1_KEYS
    )
  )
    errors.push(
      issue(
        "V2_MIGRATION_PROVENANCE",
        "/migration_provenance",
        "Pinned v1 provenance and field manifests differ."
      )
    );
  const blockerById = new Map(
    (value?.blockers ?? []).map((blocker) => [blocker.id, blocker])
  );
  if (blockerById.size !== (value?.blockers ?? []).length)
    errors.push(
      issue("V2_RISK_IDS", "/blockers", "Blocker IDs must be globally unique.")
    );
  for (const riskId of [
    "MRSDK-R024",
    "MRSDK-R027",
    "MRSDK-R030",
    "MRSDK-R036",
    "MRSDK-R037",
    "MRSDK-R038",
  ])
    if (!blockerById.has(riskId))
      errors.push(
        issue(
          "V2_RISK_MAPPING",
          "/blockers",
          `Missing central risk mapping ${riskId}.`
        )
      );
  for (const [riskId, expected] of Object.entries(
    EXPECTED_CENTRAL_RISK_CONTRACT
  )) {
    const blocker = blockerById.get(riskId);
    if (
      blocker &&
      (String(blocker.severity).toLowerCase() !== expected.severity ||
        !valuesEqual(blocker.owner_task_ids ?? [], expected.owner_task_ids) ||
        !valuesEqual(blocker.threat_ids ?? [], expected.threat_ids) ||
        !valuesEqual(blocker.boundary_ids ?? [], expected.boundary_ids) ||
        !valuesEqual(blocker.closure_criteria ?? [], expected.closure_criteria))
    )
      errors.push(
        issue(
          `V2_${riskId.slice(-4)}`,
          "/blockers",
          `${riskId} differs from the exact authoritative risk contract.`
        )
      );
  }
  const forwarding = blockerById.get("MRSDK-R036");
  if (
    forwarding &&
    (!(forwarding.affected_requirement_ids ?? []).includes("FR-7.4-011") ||
      blockerById.has("FR-7.4-011"))
  )
    errors.push(
      issue(
        "V2_FR_7_4_011_RISK",
        "/blockers",
        "FR-7.4-011 must be a requirement reference owned by MRSDK-R036, never a blocker identity."
      )
    );
  if (
    forwarding &&
    /(?:observed|confirmed|proven).{0,80}(?:plaintext|megolm|session[-_ ]?key|attachment[-_ ]?key).{0,40}(?:leak|exfiltrat)/iu.test(
      JSON.stringify(forwarding)
    )
  )
    errors.push(
      issue(
        "V2_R036_CLAIM",
        "/blockers",
        "MRSDK-R036 is a forwarding correctness and metadata-privacy risk, not evidence of a plaintext or key leak."
      )
    );
  const durableBlockers = durableAudit?.blockers_and_risks ?? [];
  const currentBlockers = value?.blockers ?? [];
  if (
    currentBlockers.length < durableBlockers.length ||
    durableBlockers.some(
      (blocker, index) =>
        currentBlockers[index]?.id !== blocker.id ||
        !valuesEqual(
          blockerSourceProjection(currentBlockers[index]),
          blockerSourceProjection(blocker)
        )
    )
  )
    errors.push(
      issue(
        "V2_BLOCKER_SOURCE_BINDING",
        "/blockers",
        "Durable-audit blockers must retain exact order, cardinality prefix, and immutable source projections."
      )
    );
  const durableDecisions = durableAudit?.architecture_decisions ?? [];
  const currentDecisions = value?.architecture_decisions ?? [];
  if (
    currentDecisions.length < durableDecisions.length ||
    durableDecisions.some(
      (decision, index) =>
        currentDecisions[index]?.id !== decision.id ||
        !valuesEqual(
          architectureDecisionSourceProjection(currentDecisions[index]),
          architectureDecisionSourceProjection(decision)
        )
    )
  )
    errors.push(
      issue(
        "V2_DECISION_SOURCE_BINDING",
        "/architecture_decisions",
        "Durable-audit decisions must retain exact order, cardinality prefix, and immutable source projections."
      )
    );
  const createdDecisionNumbers = currentDecisions
    .slice(durableDecisions.length)
    .map((decision) => Number(/^AD-(\d+)$/u.exec(decision.id)?.[1]));
  if (
    createdDecisionNumbers.some(
      (number, index) =>
        !Number.isInteger(number) ||
        number <= 26 ||
        (index > 0 && number <= createdDecisionNumbers[index - 1])
    )
  )
    errors.push(
      issue(
        "V2_DECISION_CREATE_ORDER",
        "/architecture_decisions",
        "Created architecture decisions must be appended in strictly increasing IDs after AD-26."
      )
    );
  const auditReports = value?.audit_reports ?? [];
  const validationReports = value?.validation_reports ?? [];
  const auditReportById = new Map(
    auditReports.map((report) => [report.id, report])
  );
  const validationReportById = new Map(
    validationReports.map((report) => [report.id, report])
  );
  if (auditReportById.size !== auditReports.length)
    errors.push(
      issue(
        "V2_AUDIT_REPORT_IDS",
        "/audit_reports",
        "Audit report IDs must be globally unique."
      )
    );
  if (validationReportById.size !== validationReports.length)
    errors.push(
      issue(
        "V2_VALIDATION_REPORT_IDS",
        "/validation_reports",
        "Validation report IDs must be globally unique."
      )
    );
  const combinedReportIds = [
    ...auditReports.map(({ id }) => id),
    ...validationReports.map(({ id }) => id),
  ];
  if (new Set(combinedReportIds).size !== combinedReportIds.length)
    errors.push(
      issue(
        "V2_REPORT_IDS_GLOBAL",
        "/",
        "Audit and validation report IDs must be unique across both report namespaces."
      )
    );
  const requirementById = new Map(requirements.map((row) => [row.id, row]));
  const clauseOwnerById = new Map(
    requirements.flatMap((row) =>
      (row.clauses ?? []).map((clause) => [clause.id, row.id])
    )
  );
  for (const [kind, entities] of [
    ["blockers", value?.blockers ?? []],
    ["architecture_decisions", value?.architecture_decisions ?? []],
  ])
    for (const [entityIndex, entity] of entities.entries()) {
      const affectedRequirements = new Set(
        entity.affected_requirement_ids ?? []
      );
      if (
        [...affectedRequirements].some(
          (requirementId) => !requirementById.has(requirementId)
        ) ||
        (entity.affected_clause_ids ?? []).some(
          (clauseId) =>
            !clauseOwnerById.has(clauseId) ||
            !affectedRequirements.has(clauseOwnerById.get(clauseId))
        )
      )
        errors.push(
          issue(
            "V2_AFFECTED_SCOPE_OWNERSHIP",
            `/${kind}/${entityIndex}`,
            "Every affected clause must exist and belong to an affected requirement."
          )
        );
    }
  const requiredTestEntries = requiredTestRegistryEntries(
    requirements,
    "requirements"
  );
  const requiredTestById = new Map(requiredTestEntries);
  errors.push(
    ...requiredTestRegistryErrors(requiredTestEntries, {
      duplicateCode: "V2_REQUIRED_TEST_IDS_GLOBAL",
      duplicatePointer: "/requirements",
      contractCode: "V2_REQUIRED_TEST_CONTRACT",
    })
  );
  const lifecycleAnalysis = durableAudit
    ? lifecycleContractAnalysis(value, durableAudit)
    : {
        errors: [],
        grouped: new Map(),
        latestAuthorizationByKey: new Map(),
        reportById: new Map(),
        baselineByKey: new Map(),
        currentEntities: new Map(),
      };
  errors.push(...lifecycleAnalysis.errors);
  for (const [index, blocker] of (value?.blockers ?? []).entries()) {
    const latest = lifecycleAnalysis.latestAuthorizationByKey.get(
      entityKey("blocker", blocker.id)
    );
    if (
      blocker.status === "closed" &&
      (!(blocker.closure_evidence ?? []).length ||
        !latest ||
        latest.authorization.to_state?.status !== "closed" ||
        latest.report.status !== "pass")
    )
      errors.push(
        issue(
          "V2_BLOCKER_CLOSURE",
          `/blockers/${index}`,
          "A closed blocker requires nonempty closure evidence and a latest closing authorization in a passing report."
        )
      );
  }
  const decisionById = new Map(
    (value?.architecture_decisions ?? []).map((decision) => [
      decision.id,
      decision,
    ])
  );
  for (const [index, decision] of (
    value?.architecture_decisions ?? []
  ).entries()) {
    const latest = lifecycleAnalysis.latestAuthorizationByKey.get(
      entityKey("architecture_decision", decision.id)
    );
    const terminal = new Set(["approved", "rejected", "superseded"]).has(
      decision.status
    );
    if (
      terminal &&
      (!(decision.closure_evidence ?? []).length ||
        !latest ||
        latest.authorization.to_state?.status !== decision.status)
    )
      errors.push(
        issue(
          "V2_DECISION_CLOSURE",
          `/architecture_decisions/${index}`,
          "A terminal decision requires nonempty closure evidence and an exact latest authorization."
        )
      );
    if (
      (decision.status === "superseded" &&
        (!decision.superseded_by_id ||
          decision.superseded_by_id === decision.id ||
          !decisionById.has(decision.superseded_by_id))) ||
      (decision.status !== "superseded" && decision.superseded_by_id !== null)
    )
      errors.push(
        issue(
          "V2_DECISION_SUPERSESSION",
          `/architecture_decisions/${index}/superseded_by_id`,
          "Supersession requires one distinct existing successor and is null for every other status."
        )
      );
  }
  for (const decision of value?.architecture_decisions ?? []) {
    const visited = new Set();
    let cursor = decision;
    while (cursor?.status === "superseded") {
      if (visited.has(cursor.id)) {
        errors.push(
          issue(
            "V2_DECISION_SUPERSESSION_CYCLE",
            "/architecture_decisions",
            "Architecture-decision supersession must be acyclic."
          )
        );
        break;
      }
      visited.add(cursor.id);
      cursor = decisionById.get(cursor.superseded_by_id);
    }
    if (
      decision.status === "superseded" &&
      cursor &&
      !new Set(["approved", "rejected"]).has(cursor.status)
    )
      errors.push(
        issue(
          "V2_DECISION_SUPERSESSION_TARGET",
          "/architecture_decisions",
          "Every supersession chain must terminate at an approved or rejected decision."
        )
      );
  }
  const allExecutions = validationReports.flatMap(
    (report) => report.executions ?? []
  );
  if (
    new Set(allExecutions.map((execution) => execution.id)).size !==
    allExecutions.length
  )
    errors.push(
      issue(
        "V2_EXECUTION_IDS_GLOBAL",
        "/validation_reports",
        "Execution IDs must be globally unique across validation reports."
      )
    );
  for (const [kind, reports] of [
    ["audit_reports", auditReports],
    ["validation_reports", validationReports],
  ])
    for (const [reportIndex, report] of reports.entries()) {
      const requirementScope = new Set(report.covered_requirement_ids ?? []);
      for (const [index, requirementId] of (
        report.covered_requirement_ids ?? []
      ).entries())
        if (!requirementById.has(requirementId))
          errors.push(
            issue(
              "V2_REPORT_SCOPE",
              `/${kind}/${reportIndex}/covered_requirement_ids/${index}`,
              "Report scope references an unknown requirement."
            )
          );
      for (const [index, clauseId] of (
        report.covered_clause_ids ?? []
      ).entries()) {
        const owner = clauseOwnerById.get(clauseId);
        if (!owner)
          errors.push(
            issue(
              "V2_REPORT_SCOPE",
              `/${kind}/${reportIndex}/covered_clause_ids/${index}`,
              "Report scope references an unknown clause."
            )
          );
        else if (!requirementScope.has(owner))
          errors.push(
            issue(
              "V2_REPORT_SCOPE_OWNERSHIP",
              `/${kind}/${reportIndex}/covered_clause_ids/${index}`,
              "Covered clause owner is absent from the report requirement scope."
            )
          );
      }
      if (kind === "validation_reports") {
        const reportReviewedAt = exactIsoTimestamp(report.reviewed_at)
          ? Date.parse(report.reviewed_at)
          : Number.NaN;
        const reportAcceptedAt = exactIsoTimestamp(report.accepted_at)
          ? Date.parse(report.accepted_at)
          : Number.NaN;
        if (
          (report.lifecycle_authorizations ?? []).length > 0 &&
          (typeof report.reviewer !== "string" ||
            !Number.isFinite(reportReviewedAt))
        )
          errors.push(
            issue(
              "V2_LIFECYCLE_AUTHORIZATION_ATTESTATION",
              `/${kind}/${reportIndex}`,
              "A validation report containing lifecycle authorizations requires a non-null reviewer and exact review timestamp."
            )
          );
        if (
          report.status === "pass" &&
          (!Number.isFinite(reportReviewedAt) ||
            !Number.isFinite(reportAcceptedAt) ||
            reportAcceptedAt < reportReviewedAt)
        )
          errors.push(
            issue(
              "V2_VALIDATION_ACCEPTANCE_ORDER",
              `/${kind}/${reportIndex}/accepted_at`,
              "A passing report must be accepted no earlier than its exact review timestamp."
            )
          );
        const executions = report.executions ?? [];
        const snapshots = report.test_contract_snapshots ?? [];
        const snapshotByTestId = new Map(
          snapshots.map((snapshot) => [snapshot.test_id, snapshot])
        );
        if (
          snapshotByTestId.size !== snapshots.length ||
          snapshots.length !== executions.length ||
          !executions.every((execution) =>
            snapshotByTestId.has(execution.test_case_id)
          )
        )
          errors.push(
            issue(
              "V2_TEST_CONTRACT_SNAPSHOTS",
              `/${kind}/${reportIndex}/test_contract_snapshots`,
              "Every execution requires exactly one unique immutable test-contract snapshot, with no extras."
            )
          );
        const executionIds = executions.map((execution) => execution.id);
        let invalidExecution = false;
        let privacyViolation = false;
        for (const [executionIndex, execution] of executions.entries()) {
          const pointer = `/${kind}/${reportIndex}/executions/${executionIndex}`;
          if (
            execution.path !==
            `docs/matrix-rust-sdk/validation/executions/${execution.id}.json`
          ) {
            invalidExecution = true;
            errors.push(
              issue(
                "V2_EXECUTION_STORAGE_PATH",
                `${pointer}/path`,
                "Execution transcript path must be the closed repository path derived from its exact execution ID."
              )
            );
          }
          const privacyErrors = executionTranscriptPrivacyViolations(execution);
          if (privacyErrors.length > 0) {
            privacyViolation = true;
            invalidExecution = true;
          }
          const registryEntry = requiredTestById.get(execution.test_case_id);
          const snapshot = snapshotByTestId.get(execution.test_case_id);
          const contract = snapshot?.execution_contract;
          const assertionIds = (execution.assertions ?? []).map(
            (assertion) => assertion?.assertion_id
          );
          const expectedAssertionIds = contract?.assertion_ids ?? [];
          const contractMatches =
            registryEntry &&
            snapshot &&
            contract &&
            snapshot.requirement_id === registryEntry.requirementId &&
            snapshot.clause_id === registryEntry.clauseId &&
            snapshot.evidence_class ===
              registryEntry.requiredTest.evidence_class &&
            report.evidence_class === snapshot.evidence_class &&
            report.covered_requirement_ids?.includes(
              registryEntry.requirementId
            ) &&
            report.covered_clause_ids?.includes(registryEntry.clauseId) &&
            execution.kind === contract.kind &&
            execution.runner_id === contract.runner_id &&
            execution.invocation_id === contract.invocation_id &&
            execution.invocation_fingerprint_sha256 ===
              contract.invocation_fingerprint_sha256 &&
            executionIdentifierOwnsTest(
              contract.invocation_id,
              "INV",
              execution.test_case_id
            ) &&
            expectedAssertionIds.every((assertionId) =>
              executionIdentifierOwnsTest(
                assertionId,
                "ASSERT",
                execution.test_case_id
              )
            ) &&
            assertionIds.every(
              (assertionId) => typeof assertionId === "string"
            ) &&
            assertionIds.length === new Set(assertionIds).size &&
            valuesEqual(
              [...assertionIds].sort(),
              [...expectedAssertionIds].sort()
            );
          if (!contractMatches) {
            invalidExecution = true;
            errors.push(
              issue(
                "V2_EXECUTION_CONTRACT",
                pointer,
                "Execution must resolve to its immutable owned report snapshot and exactly match that closed execution contract."
              )
            );
          }
          let fingerprintMatches = false;
          try {
            fingerprintMatches =
              execution.result_fingerprint_sha256 ===
              executionResultFingerprint(execution);
          } catch {
            fingerprintMatches = false;
          }
          if (!fingerprintMatches) {
            invalidExecution = true;
            errors.push(
              issue(
                "V2_EXECUTION_FINGERPRINT",
                `${pointer}/result_fingerprint_sha256`,
                "Execution result fingerprint must cover the exact canonical secret-free transcript semantics."
              )
            );
          }
          const assertionResults = (execution.assertions ?? []).map(
            (assertion) => assertion?.result
          );
          const resultMatches =
            (execution.result === "pass" &&
              assertionResults.length > 0 &&
              assertionResults.every((result) => result === "pass")) ||
            (execution.result === "fail" &&
              assertionResults.includes("fail") &&
              !assertionResults.includes("blocked")) ||
            (execution.result === "blocked" &&
              assertionResults.includes("blocked") &&
              !assertionResults.includes("fail"));
          const exitMatches =
            (execution.kind === "scenario" && execution.exit_code === null) ||
            (execution.kind === "command" &&
              ((execution.result === "pass" && execution.exit_code === 0) ||
                (execution.result === "fail" &&
                  Number.isInteger(execution.exit_code) &&
                  execution.exit_code >= 1 &&
                  execution.exit_code <= 255) ||
                (execution.result === "blocked" &&
                  Number.isInteger(execution.exit_code) &&
                  execution.exit_code >= 1 &&
                  execution.exit_code <= 255)));
          if (!resultMatches || !exitMatches) {
            invalidExecution = true;
            errors.push(
              issue(
                "V2_EXECUTION_RESULT",
                pointer,
                "Execution result, assertion outcomes, and command/scenario exit semantics contradict."
              )
            );
          }
          const started = Date.parse(execution.started_at);
          const finished = Date.parse(execution.finished_at);
          if (
            !Number.isFinite(started) ||
            !Number.isFinite(finished) ||
            finished < started
          ) {
            invalidExecution = true;
            errors.push(
              issue(
                "V2_EXECUTION_TIMESTAMPS",
                pointer,
                "Execution timestamps must be valid and ordered."
              )
            );
          }
          if (
            !Number.isFinite(reportReviewedAt) ||
            (Number.isFinite(finished) && finished > reportReviewedAt)
          ) {
            invalidExecution = true;
            errors.push(
              issue(
                "V2_EXECUTION_REVIEW_ORDER",
                `${pointer}/finished_at`,
                "Every execution must finish no later than the containing report review timestamp."
              )
            );
          }
          if (
            execution.subject_sha !== report.subject_sha ||
            (report.status === "pass" && execution.result !== "pass")
          )
            invalidExecution = true;
        }
        if (
          (report.status === "pass" && executionIds.length === 0) ||
          new Set(executionIds).size !== executionIds.length ||
          invalidExecution
        )
          errors.push(
            issue(
              "V2_VALIDATION_EXECUTIONS",
              `/${kind}/${reportIndex}/executions`,
              "Passing validation requires unique successful execution evidence bound to the exact implementation subject."
            )
          );
        if (privacyViolation)
          errors.push(
            issue(
              "V2_VALIDATION_EXECUTION_PRIVACY",
              `/${kind}/${reportIndex}/executions`,
              "Execution evidence contains content outside the privacy-safe structural allowlist."
            )
          );
      }
    }
  for (const [index, row] of requirements.entries()) {
    try {
      const durableRow = durableRowById.get(row.id);
      if (
        !durableRow ||
        durableRow.audit?.audited_payload_sha256 !==
          auditedRowDigest(durableRow) ||
        row.audit?.audited_payload_sha256 !==
          durableRow.audit?.audited_payload_sha256
      )
        errors.push(
          issue(
            "V2_ROW_DIGEST",
            `/requirements/${index}/audit/audited_payload_sha256`,
            "Requirement audit digest must equal the independently validated durable source-audit row digest."
          )
        );
    } catch (error) {
      errors.push(
        issue(
          "V2_ROW_DIGEST",
          `/requirements/${index}/audit/audited_payload_sha256`,
          error.message
        )
      );
    }
    if (
      row.current_product?.status === "implemented" &&
      (row.clauses ?? []).some(
        (clause) => clause.current_product_status !== "implemented"
      )
    )
      errors.push(
        issue(
          "V2_ROLLUP",
          `/requirements/${index}/current_product/status`,
          "Implemented row contains incomplete clause."
        )
      );
    const clauseIdsForRow = (row.clauses ?? []).map((clause) => clause.id);
    const auditReferences = row.audit?.report_ids ?? [];
    let acceptedAuditScope = false;
    for (const [referenceIndex, reportId] of auditReferences.entries()) {
      const report = auditReportById.get(reportId);
      const scopeMatches =
        report &&
        report.subject_sha === value?.source_audit?.subject_sha &&
        report.artifact_payload_sha256 ===
          value?.source_audit?.canonical_payload_sha256 &&
        report.covered_requirement_ids?.includes(row.id) &&
        clauseIdsForRow.every((clauseId) =>
          report.covered_clause_ids?.includes(clauseId)
        );
      if (!scopeMatches)
        errors.push(
          issue(
            "V2_AUDIT_REPORT_REFERENCE",
            `/requirements/${index}/audit/report_ids/${referenceIndex}`,
            "Audit report does not resolve with exact subject, payload, requirement, and clause scope."
          )
        );
      if (
        scopeMatches &&
        report.verdict === "accept" &&
        report.reviewer === row.audit?.reviewer &&
        report.reviewed_at === row.audit?.reviewed_at
      )
        acceptedAuditScope = true;
    }
    if (row.audit?.state === "accepted" && !acceptedAuditScope)
      errors.push(
        issue(
          "V2_AUDIT_ACCEPTANCE",
          `/requirements/${index}/audit/report_ids`,
          "Accepted row state requires an exact-scope accepted audit report."
        )
      );
    const rowValidationReferences =
      row.rust_cutover?.validation_report_ids ?? [];
    const implementationSubject =
      row.rust_cutover?.implementation_subject_sha ?? null;
    let passingValidationScope = rowValidationReferences.length > 0;
    for (const [
      referenceIndex,
      reportId,
    ] of rowValidationReferences.entries()) {
      const report = validationReportById.get(reportId);
      const scopeMatches =
        report &&
        report.subject_sha === implementationSubject &&
        report.covered_requirement_ids?.includes(row.id);
      const passingScope =
        scopeMatches &&
        report.status === "pass" &&
        report.executions?.some((execution) => {
          const entry = requiredTestById.get(execution.test_case_id);
          const snapshot = report.test_contract_snapshots?.find(
            (candidate) => candidate.test_id === execution.test_case_id
          );
          return (
            entry?.requirementId === row.id &&
            entry.requiredTest.status === "accepted" &&
            valuesEqual(
              snapshot?.execution_contract ?? {},
              entry.requiredTest.execution_contract ?? {}
            ) &&
            execution.result === "pass"
          );
        });
      if (!scopeMatches)
        errors.push(
          issue(
            "V2_VALIDATION_REPORT_REFERENCE",
            `/requirements/${index}/rust_cutover/validation_report_ids/${referenceIndex}`,
            "Validation report does not resolve with the exact Rust implementation subject and required requirement scope."
          )
        );
      if (!passingScope) passingValidationScope = false;
    }
    let everyClauseHasPassingValidation = true;
    let everyClauseRequiredTestIsValidated = true;
    for (const [clauseIndex, clause] of (row.clauses ?? []).entries()) {
      const references = clause.rust_mapping?.validation_report_ids ?? [];
      const passingRequiredTestIds = new Set();
      if (!references.length) everyClauseHasPassingValidation = false;
      for (const [referenceIndex, reportId] of references.entries()) {
        const report = validationReportById.get(reportId);
        const scopeMatches =
          report &&
          report.subject_sha === implementationSubject &&
          report.covered_requirement_ids?.includes(row.id) &&
          report.covered_clause_ids?.includes(clause.id);
        const passingScope =
          scopeMatches &&
          report.status === "pass" &&
          report.executions?.some((execution) => {
            const entry = requiredTestById.get(execution.test_case_id);
            const snapshot = report.test_contract_snapshots?.find(
              (candidate) => candidate.test_id === execution.test_case_id
            );
            return (
              entry?.clauseId === clause.id &&
              entry.requiredTest.status === "accepted" &&
              valuesEqual(
                snapshot?.execution_contract ?? {},
                entry.requiredTest.execution_contract ?? {}
              ) &&
              execution.result === "pass"
            );
          });
        if (!scopeMatches)
          errors.push(
            issue(
              "V2_VALIDATION_REPORT_REFERENCE",
              `/requirements/${index}/clauses/${clauseIndex}/rust_mapping/validation_report_ids/${referenceIndex}`,
              "Validation report does not resolve with the exact Rust implementation subject and required requirement and clause scope."
            )
          );
        if (!passingScope) everyClauseHasPassingValidation = false;
        if (passingScope)
          for (const execution of report.executions ?? [])
            if (
              execution.result === "pass" &&
              requiredTestById.get(execution.test_case_id)?.requiredTest
                ?.status === "accepted" &&
              valuesEqual(
                report.test_contract_snapshots?.find(
                  (snapshot) => snapshot.test_id === execution.test_case_id
                )?.execution_contract ?? {},
                requiredTestById.get(execution.test_case_id)?.requiredTest
                  ?.execution_contract ?? {}
              )
            )
              passingRequiredTestIds.add(execution.test_case_id);
      }
      const requiredTests = clause.required_tests ?? [];
      for (const [requiredTestIndex, requiredTest] of requiredTests.entries())
        if (
          requiredTest.status === "accepted" &&
          !references.some((reportId) => {
            const report = validationReportById.get(reportId);
            return (
              report?.status === "pass" &&
              report.subject_sha === implementationSubject &&
              report.evidence_class === requiredTest.evidence_class &&
              report.covered_requirement_ids?.includes(row.id) &&
              report.covered_clause_ids?.includes(clause.id) &&
              valuesEqual(
                report.test_contract_snapshots?.find(
                  (snapshot) => snapshot.test_id === requiredTest.id
                )?.execution_contract ?? {},
                requiredTest.execution_contract ?? {}
              ) &&
              report.executions?.some(
                (execution) =>
                  execution.test_case_id === requiredTest.id &&
                  execution.result === "pass" &&
                  execution.subject_sha === implementationSubject
              )
            );
          })
        )
          errors.push(
            issue(
              "V2_ACCEPTED_TEST_EXECUTION",
              `/requirements/${index}/clauses/${clauseIndex}/required_tests/${requiredTestIndex}`,
              "An accepted required test needs an exact passing execution in a referenced passing validation report."
            )
          );
      const completeRequiredTestCoverage =
        requiredTests.length > 0 &&
        requiredTests.every(
          (requiredTest) =>
            requiredTest.status === "accepted" &&
            requiredTest.execution_contract &&
            passingRequiredTestIds.has(requiredTest.id)
        );
      if (!completeRequiredTestCoverage) {
        everyClauseRequiredTestIsValidated = false;
        if (row.rust_cutover?.readiness === "ready")
          errors.push(
            issue(
              "V2_CLAUSE_REQUIRED_TEST_VALIDATION",
              `/requirements/${index}/clauses/${clauseIndex}/required_tests`,
              "Every required test for a Rust-ready clause must be accepted and represented by a passing execution in the clause's referenced validation reports."
            )
          );
      }
    }
    if (row.rust_cutover?.readiness === "ready") {
      const rowClauseIds = new Set(clauseIdsForRow);
      const globalLifecycleBlock = [
        ...(value?.blockers ?? []).filter((blocker) =>
          isUnresolvedRiskStatus(blocker.status)
        ),
        ...(value?.architecture_decisions ?? []).filter((decision) =>
          new Set(["unresolved", "proposed"]).has(decision.status)
        ),
      ].some(
        (entity) =>
          entity.affected_requirement_ids?.includes(row.id) ||
          entity.affected_clause_ids?.some((clauseId) =>
            rowClauseIds.has(clauseId)
          )
      );
      const blocked =
        globalLifecycleBlock ||
        (row.rust_cutover.gap_ids?.length ?? 0) ||
        (row.rust_cutover.gate_ids?.length ?? 0) ||
        (row.rust_cutover.blocker_ids?.length ?? 0) ||
        !["matrix_rust_sdk", "product_only", "none"].includes(
          row.rust_cutover.matrix_owner
        ) ||
        row.rust_cutover.surviving_raw_matrix_http !== false ||
        row.rust_cutover.surviving_matrix_js_owner !== false ||
        !SAFE_SHA.test(implementationSubject ?? "") ||
        !(row.audit?.state === "accepted") ||
        !acceptedAuditScope ||
        !passingValidationScope ||
        !everyClauseHasPassingValidation ||
        !everyClauseRequiredTestIsValidated;
      if (blocked)
        errors.push(
          issue(
            "V2_RUST_READY",
            `/requirements/${index}/rust_cutover/readiness`,
            "Rust ready requires accepted audit and current validation, no local gaps/gates, and no unresolved globally affecting blocker or decision."
          )
        );
    }
  }
  const preservedDigests = new Map(
    (value?.migration_provenance?.preserved_ledger_digests ?? []).map(
      (entry) => [entry.key, entry]
    )
  );
  for (const key of PRESERVED_V1_KEYS) {
    const expected = preservedDigests.get(key);
    const actual =
      key in (value ?? {}) ? sha256(canonicalize(value[key])) : null;
    if (
      !expected ||
      actual !== expected.pre_migration_canonical_sha256 ||
      actual !== expected.post_migration_canonical_sha256 ||
      actual !== PRESERVED_V1_SHA256[key]
    )
      errors.push(
        issue(
          "V2_LEDGER_DIGEST",
          `/${key}`,
          "Preserved v1 ledger digest differs."
        )
      );
  }
  return errors;
}

export async function validateTraceability(
  value,
  {
    gitObjects,
    repoRoot = defaultRepositoryRoot(),
    durableAuditIdentity,
    durableAudit,
    skipGitEvidence = false,
    historyMode = "current",
  } = {}
) {
  try {
    assertCanonicalValue(value);
  } catch (error) {
    return [issue("CANONICAL_VALUE", "/", error.message, V1_PATH)];
  }
  const schemaErrors = validateSchema("v2", value, {
    repositoryRoot: repoRoot,
  });
  let semanticErrors;
  try {
    semanticErrors = semanticV2Errors(
      value,
      durableAuditIdentity,
      durableAudit
    );
  } catch {
    semanticErrors = [
      issue(
        "V2_SEMANTIC_INPUT",
        "/",
        "Semantic validation could not process structurally invalid input."
      ),
    ];
  }
  const errors = [...schemaErrors, ...semanticErrors];
  if (historyMode === "initial-migration") {
    if (
      value?.lifecycle?.previous_artifact !== null ||
      (value?.audit_reports ?? []).length > 0 ||
      (value?.validation_reports ?? []).length > 0 ||
      (value?.lifecycle?.entity_chains ?? []).some(
        (chain) => (chain.authorization_ids ?? []).length > 0
      )
    )
      errors.push(
        issue(
          "V2_INITIAL_MIGRATION_HISTORY",
          "/lifecycle",
          "Initial migration requires null predecessor and empty report and authorization history."
        )
      );
  } else if (historyMode !== "current")
    throw new TypeError("Unsupported internal lifecycle history mode.");
  if (skipGitEvidence) {
    const reportBackedState =
      (value?.audit_reports?.length ?? 0) > 0 ||
      (value?.validation_reports?.length ?? 0) > 0 ||
      value?.lifecycle?.previous_artifact !== null ||
      (value?.lifecycle?.entity_chains ?? []).some(
        (chain) => (chain.authorization_ids ?? []).length > 0
      ) ||
      (value?.requirements ?? []).some(
        (row) =>
          row.audit?.state === "accepted" ||
          row.rust_cutover?.readiness === "ready" ||
          (row.audit?.report_ids?.length ?? 0) > 0 ||
          (row.rust_cutover?.validation_report_ids?.length ?? 0) > 0 ||
          typeof row.rust_cutover?.implementation_subject_sha === "string" ||
          (row.clauses ?? []).some(
            (clause) =>
              (clause.rust_mapping?.validation_report_ids?.length ?? 0) > 0
          )
      );
    if (reportBackedState)
      errors.push(
        issue(
          "V2_GIT_EVIDENCE_SKIPPED",
          "/",
          "Skipping Git evidence cannot authorize accepted audit state, Rust readiness, or report-backed claims."
        )
      );
  } else if (schemaErrors.length === 0) {
    if (!gitObjects) throw new TypeError("Git object adapter is required.");
    if (historyMode === "current") {
      try {
        if (typeof gitObjects.derivePreviousArtifact !== "function")
          throw new Error(
            "Git object adapter cannot derive first-parent lifecycle history."
          );
        // One pretty-JSON materialization of the candidate: derive + chain compare
        // previously each built a full multi-megabyte buffer.
        const candidateBytes = Buffer.from(prettyJson(value));
        const previousIdentity = await gitObjects.derivePreviousArtifact(
          candidateBytes,
          V1_PATH
        );
        if (
          !valuesEqual(
            value?.lifecycle?.previous_artifact ?? null,
            previousIdentity
          )
        )
          errors.push(
            issue(
              "V2_PREVIOUS_ARTIFACT",
              "/lifecycle/previous_artifact",
              "Embedded previous artifact differs from the immediate first-parent v2 path version."
            )
          );
        if (typeof gitObjects.lifecyclePathVersions !== "function")
          throw new Error(
            "Git object adapter cannot inspect the complete first-parent v2 epoch."
          );
        const versions = await gitObjects.lifecyclePathVersions(V1_PATH);
        const committedV2 = versions.filter((version) => version.kind === "v2");
        const candidateEqualsHead = Boolean(
          committedV2[0]?.bytes && candidateBytes.equals(committedV2[0].bytes)
        );
        const chain = candidateEqualsHead
          ? committedV2
          : [
              { kind: "candidate", bytes: candidateBytes, value },
              ...committedV2,
            ];
        const versionIdentity = (version) =>
          version
            ? {
                commit_sha: version.commit_sha,
                blob_oid: version.oid,
                file_sha256: sha256(version.bytes),
                canonical_sha256: sha256(canonicalize(version.value)),
              }
            : null;
        for (const [index, version] of chain.entries()) {
          const older = chain[index + 1] ?? null;
          const expectedPrevious = versionIdentity(older);
          if (
            !valuesEqual(
              version.value?.lifecycle?.previous_artifact ?? null,
              expectedPrevious
            )
          )
            errors.push(
              issue(
                "V2_PREVIOUS_ARTIFACT",
                "/lifecycle/previous_artifact",
                "Every v2 path version must name its exact immediate first-parent predecessor."
              )
            );
          if (version.kind === "v2" && index > 0) {
            const historicalSchemaErrors = validateSchema("v2", version.value, {
              repositoryRoot: repoRoot,
            });
            const historicalSemanticErrors = semanticV2Errors(
              version.value,
              durableAuditIdentity,
              durableAudit
            );
            if (
              historicalSchemaErrors.length ||
              historicalSemanticErrors.length
            )
              throw new Error(
                "Historical v2 artifact fails closed schema or semantic lifecycle validation."
              );
          }
          if (older)
            errors.push(...lifecyclePrefixErrors(older.value, version.value));
          errors.push(
            ...terminalIntroductionErrors(older?.value ?? null, version.value)
          );
        }
      } catch (error) {
        errors.push(issue("V2_LIFECYCLE_HISTORY", "/lifecycle", error.message));
      }
    }
    errors.push(
      ...(
        await verifyRiskAuthority(
          value.provenance?.risk_register,
          durableAudit?.blockers_and_risks,
          gitObjects
        )
      ).map((entry) => ({
        ...entry,
        pointer: `/provenance/risk_register${entry.pointer}`,
      }))
    );
    for (const [index, report] of (value.audit_reports ?? []).entries())
      errors.push(
        ...(await verifyDurableReport(report, gitObjects)).map((entry) => ({
          ...entry,
          pointer: `/audit_reports/${index}${entry.pointer}`,
        }))
      );
    for (const [index, report] of (value.validation_reports ?? []).entries()) {
      errors.push(
        ...(await verifyDurableReport(report, gitObjects)).map((entry) => ({
          ...entry,
          pointer: `/validation_reports/${index}${entry.pointer}`,
        }))
      );
      for (const [executionIndex, execution] of (
        report.executions ?? []
      ).entries())
        errors.push(
          ...(
            await verifyDurableExecution(
              execution,
              gitObjects,
              report.storage_commit_sha
            )
          ).map((entry) => ({
            ...entry,
            pointer: `/validation_reports/${index}/executions/${executionIndex}${entry.pointer}`,
          }))
        );
    }
    if (durableAudit) {
      const lifecycleAnalysis = lifecycleContractAnalysis(value, durableAudit);
      for (const entries of lifecycleAnalysis.grouped.values()) {
        let previousAuthorization = null;
        let previousReport = null;
        const seenReportIds = new Set();
        for (const { authorization, report } of entries) {
          const pointer = `/validation_reports/${(
            value.validation_reports ?? []
          ).indexOf(report)}/lifecycle_authorizations/${(
            report.lifecycle_authorizations ?? []
          ).indexOf(authorization)}`;
          if (
            previousReport &&
            previousReport.id !== report.id &&
            (seenReportIds.has(report.id) ||
              typeof gitObjects.commitIsAncestorOf !== "function" ||
              !gitObjects.commitIsAncestorOf(
                previousReport.storage_commit_sha,
                report.storage_commit_sha
              ))
          )
            errors.push(
              issue(
                "V2_LIFECYCLE_REPORT_HISTORY",
                pointer,
                "Distinct lifecycle authorization reports must be contiguous and advance by Git storage-commit ancestry."
              )
            );
          if (
            previousAuthorization?.entity_kind === "requirement" &&
            authorization.entity_kind === "requirement"
          ) {
            const fromSubject =
              previousAuthorization.to_state?.rust_cutover
                ?.implementation_subject_sha;
            const toSubject =
              authorization.to_state?.rust_cutover?.implementation_subject_sha;
            if (fromSubject && toSubject && fromSubject !== toSubject) {
              const advancesByAncestry =
                typeof gitObjects.commitIsAncestorOf === "function" &&
                gitObjects.commitIsAncestorOf(fromSubject, toSubject);
              const ancestryNeutralToState = structuredClone(
                authorization.to_state
              );
              ancestryNeutralToState.rust_cutover.implementation_subject_sha =
                fromSubject;
              const semanticOperation = expectedLifecycleOperation(
                "requirement",
                previousAuthorization.to_state,
                ancestryNeutralToState,
                new Set()
              );
              const expectedOperation = advancesByAncestry
                ? semanticOperation ?? "advance"
                : "rollback";
              if (authorization.operation !== expectedOperation)
                errors.push(
                  issue(
                    "V2_LIFECYCLE_SUBJECT_HISTORY",
                    pointer,
                    `Changed-subject lifecycle transition must be classified as ${expectedOperation} from Git ancestry and state direction.`
                  )
                );
              if (
                expectedOperation === "rollback" &&
                authorization.to_state?.rust_cutover?.readiness === "ready"
              )
                errors.push(
                  issue(
                    "V2_LIFECYCLE_ROLLBACK_READY",
                    pointer,
                    "A requirement rollback must finish below ready."
                  )
                );
            }
          }
          seenReportIds.add(report.id);
          previousAuthorization = authorization;
          previousReport = report;
        }
      }
    }
    for (
      let rowIndex = 0;
      rowIndex < (value.requirements ?? []).length;
      rowIndex += 1
    ) {
      const row = value.requirements[rowIndex];
      const implementationSubject =
        row.rust_cutover?.implementation_subject_sha ?? null;
      if (implementationSubject) {
        try {
          const subject = await gitObjects.object(implementationSubject);
          if (!subject || subject.type !== "commit")
            errors.push(
              issue(
                "V2_IMPLEMENTATION_SUBJECT",
                `/requirements/${rowIndex}/rust_cutover/implementation_subject_sha`,
                "Rust implementation subject is missing or is not a commit."
              )
            );
          else if (
            typeof gitObjects.commitIsAncestorOfHead !== "function" ||
            !gitObjects.commitIsAncestorOfHead(implementationSubject)
          )
            errors.push(
              issue(
                "V2_IMPLEMENTATION_SUBJECT_HISTORY",
                `/requirements/${rowIndex}/rust_cutover/implementation_subject_sha`,
                "Rust implementation subject is not an ancestor of current HEAD."
              )
            );
        } catch (error) {
          errors.push(
            issue(
              "V2_IMPLEMENTATION_SUBJECT",
              `/requirements/${rowIndex}/rust_cutover/implementation_subject_sha`,
              error.message
            )
          );
        }
      }
      for (
        let clauseIndex = 0;
        clauseIndex < (row.clauses ?? []).length;
        clauseIndex += 1
      ) {
        const clause = row.clauses[clauseIndex];
        for (const [index, evidence] of (
          clause.source_evidence ?? []
        ).entries())
          errors.push(
            ...(await verifySourceEvidence(evidence, gitObjects)).map(
              (entry) => ({
                ...entry,
                pointer: `/requirements/${rowIndex}/clauses/${clauseIndex}/source_evidence/${index}${entry.pointer}`,
              })
            )
          );
        for (const [index, evidence] of (
          clause.absence_evidence ?? []
        ).entries())
          errors.push(
            ...(await verifyAbsenceEvidence(evidence, gitObjects)).map(
              (entry) => ({
                ...entry,
                pointer: `/requirements/${rowIndex}/clauses/${clauseIndex}/absence_evidence/${index}${entry.pointer}`,
              })
            )
          );
        for (const [index, evidence] of (clause.existing_tests ?? []).entries())
          errors.push(
            ...(await verifySourceEvidence(evidence, gitObjects)).map(
              (entry) => ({
                ...entry,
                pointer: `/requirements/${rowIndex}/clauses/${clauseIndex}/existing_tests/${index}${entry.pointer}`,
              })
            )
          );
      }
    }
  }
  return sortDiagnostics(errors);
}

function joinUtf8Lines(lines) {
  if (lines.length === 0) return Buffer.alloc(0);
  let total = lines.length - 1;
  for (const line of lines) total += Buffer.byteLength(line, "utf8");
  const bytes = Buffer.allocUnsafe(total);
  let offset = 0;
  for (let index = 0; index < lines.length; index += 1) {
    offset += bytes.write(lines[index], offset, "utf8");
    if (index < lines.length - 1) bytes[offset++] = 0x0a;
  }
  return bytes;
}

function leaves(value, pointer = "") {
  if (value === null || typeof value !== "object")
    return [{ pointer: pointer || "/", value }];
  if (Array.isArray(value)) {
    if (value.length === 0) return [{ pointer: pointer || "/", value }];
    return value.flatMap((entry, index) =>
      leaves(entry, `${pointer}/${index}`)
    );
  }
  const keys = Object.keys(value).sort();
  if (keys.length === 0) return [{ pointer: pointer || "/", value }];
  return keys.flatMap((key) =>
    leaves(value[key], `${pointer}/${escapePointer(key)}`)
  );
}

function anchorForPointer(pointer) {
  return `leaf-${sha256(Buffer.from(pointer)).slice(0, 20)}`;
}

function fencedCanonicalJson(value) {
  const content = canonicalize(value).toString("utf8");
  const runs = content.match(/`+/gu) ?? [];
  const fenceLength = Math.max(
    3,
    1 + runs.reduce((longest, run) => Math.max(longest, run.length), 0)
  );
  const fence = "`".repeat(fenceLength);
  return `${fence}json\n${content}\n${fence}`;
}

function markdownTableJson(value) {
  return canonicalize(value)
    .toString("utf8")
    .replaceAll("&", "&amp;")
    .replaceAll("|", "&#124;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll("`", "&#96;");
}

function renderMarkdown(kind, value) {
  const jsonDigest = sha256(Buffer.from(prettyJson(value)));
  const semanticLeaves = leaves(value).sort((a, b) =>
    a.pointer < b.pointer ? -1 : a.pointer > b.pointer ? 1 : 0
  );
  const lines = [
    `# ${kind}`,
    "",
    "> Generated from the authoritative JSON. Do not hand-edit semantic content.",
    "",
    `Authoritative JSON SHA-256: \`${jsonDigest}\``,
    "",
    "## Semantic leaves",
    "",
  ];
  for (const leaf of semanticLeaves) {
    const anchor = anchorForPointer(leaf.pointer);
    lines.push(
      `<a id="${anchor}"></a>`,
      "",
      `### Leaf ${anchor}`,
      "",
      "JSON Pointer (canonical JSON string):",
      "",
      fencedCanonicalJson(leaf.pointer),
      "",
      "Value (canonical JSON):",
      "",
      fencedCanonicalJson(leaf.value),
      ""
    );
  }
  lines.push(
    "## JSON Pointer coverage",
    "",
    "| Canonical JSON Pointer String | Canonical leaf-value SHA-256 | Markdown anchor |",
    "| --- | --- | --- |"
  );
  for (const leaf of semanticLeaves) {
    const digest = sha256(canonicalize(leaf.value));
    const anchor = anchorForPointer(leaf.pointer);
    lines.push(
      `| ${markdownTableJson(
        leaf.pointer
      )} | ${digest} | [${anchor}](#${anchor}) |`
    );
  }
  lines.push("");
  // Encode lines directly into one Buffer so peak RSS does not hold both the
  // giant joined string and the output bytes (33MiB+ markdown on 119-row graphs).
  const bytes = joinUtf8Lines(lines);
  return {
    bytes,
    coveredPointers: new Set(semanticLeaves.map((leaf) => leaf.pointer)),
  };
}

export function renderAuditMarkdown(value) {
  return renderMarkdown("R0.2-E 119-row audit normalization", value);
}

export function renderTraceabilityMarkdown(value) {
  return renderMarkdown(
    "Matrix Rust SDK feature-parity traceability v2",
    value
  );
}

function deepCopy(value) {
  assertCanonicalValue(value);
  return structuredClone(value);
}

export function migrateV1({ v1, audit, sourceIdentity, auditIdentity }) {
  for (const value of [v1, audit, sourceIdentity, auditIdentity])
    assertCanonicalValue(value);
  const auditRequiredTestEntries = requiredTestRegistryEntries(
    audit.rows ?? [],
    "rows"
  );
  const sourceDomainErrors = [
    ...requiredTestRegistryErrors(auditRequiredTestEntries, {
      duplicateCode: "AUDIT_REQUIRED_TEST_IDS_GLOBAL",
      duplicatePointer: "/rows",
      contractCode: "AUDIT_REQUIRED_TEST_CONTRACT",
      sourceAudit: true,
    }),
    ...preMigrationAuthorityErrors(audit.rows ?? []),
  ];
  if ((audit.rows ?? []).some((row) => row.rust_cutover?.readiness === "ready"))
    sourceDomainErrors.push(
      issue(
        "AUDIT_RUST_READY",
        "/rows",
        "R0.2 source audit cannot establish Rust-ready status."
      )
    );
  if (sourceDomainErrors.length)
    throw new Error(
      "Migration accepts only the planned, authority-free source-audit baseline."
    );
  const actualKeys = Object.keys(v1).sort();
  const expectedKeys = [...TRANSFORMED_V1_KEYS, ...PRESERVED_V1_KEYS].sort();
  if (!valuesEqual(actualKeys, expectedKeys))
    throw new Error("V1 top-level field manifest differs; migration refused.");
  if (sha256(canonicalize(v1)) !== V1_CANONICAL_SHA256)
    throw new Error("Pinned v1 canonical identity differs.");
  if (
    sourceIdentity.commit_sha !== AUDITED_SOURCE_COMMIT ||
    sourceIdentity.blob_oid !== V1_BLOB ||
    sourceIdentity.file_sha256 !== V1_SHA256
  )
    throw new Error("Pinned v1 source identity differs.");
  const auditById = new Map(audit.rows.map((row) => [row.id, row]));
  if (auditById.size !== EXPECTED_REQUIREMENT_COUNT)
    throw new Error("Durable audit coverage differs.");
  const legacyIds = v1.requirements.map((row) => row.id);
  if (
    legacyIds.length !== EXPECTED_REQUIREMENT_COUNT ||
    new Set(legacyIds).size !== EXPECTED_REQUIREMENT_COUNT ||
    !valuesEqual([...legacyIds].sort(), [...auditById.keys()].sort())
  )
    throw new Error(
      "Pinned v1 and durable-audit requirement manifests differ."
    );
  const preserved = Object.fromEntries(
    PRESERVED_V1_KEYS.map((key) => [key, deepCopy(v1[key])])
  );
  const transformedRequirements = v1.requirements.map((legacy) => {
    const audited = auditById.get(legacy.id);
    if (!audited) throw new Error(`Missing audit row ${legacy.id}.`);
    return deepCopy(audited);
  });
  const statusVocabulary = [
    "implemented",
    "partial",
    "missing",
    "not_exposed",
    "dead_path_proven",
    "unverified",
  ];
  const readinessVocabulary = [
    "not_assessed",
    "blocked",
    "implementation_planned",
    "implementation_in_progress",
    "validation_pending",
    "ready",
  ];
  const currentStatuses = transformedRequirements.map(
    (row) => row.current_product
  );
  const readiness = transformedRequirements.map((row) => row.rust_cutover);
  const preservedDigests = PRESERVED_V1_KEYS.map((key) => {
    const digest = sha256(canonicalize(v1[key]));
    return {
      key,
      pre_migration_canonical_sha256: digest,
      post_migration_canonical_sha256: digest,
    };
  });
  const output = {
    schema_version: "2.0",
    task_id: deepCopy(v1.task_id),
    title: deepCopy(v1.title),
    provenance: {
      repository: "synara-desktop",
      integration_branch: "feature/matrix-rust-sdk-full-replacement",
      plan: {
        path: AUDITED_PLAN_PATH,
        blob_oid: CURRENT_PLAN_BLOB,
        file_sha256: CURRENT_PLAN_SHA256,
      },
      migration_source: { path: V1_PATH, ...deepCopy(sourceIdentity) },
      audited_source_sha: AUDITED_SOURCE_COMMIT,
      risk_register: {
        commit_sha: RISK_REGISTER_COMMIT,
        path: RISK_REGISTER_PATH,
        blob_oid: RISK_REGISTER_BLOB,
        file_sha256: RISK_REGISTER_SHA256,
        canonical_sha256: RISK_REGISTER_CANONICAL_SHA256,
      },
      durable_audit: deepCopy(auditIdentity),
      generated_markdown_path: V1_MARKDOWN_PATH,
    },
    summary: {
      requirement_count: transformedRequirements.length,
      clause_count: transformedRequirements.reduce(
        (count, row) => count + row.clauses.length,
        0
      ),
      current_product_status_counts: countsBy(
        currentStatuses,
        (value) => value?.status,
        statusVocabulary
      ),
      rust_cutover_readiness_counts: countsBy(
        readiness,
        (value) => value?.readiness,
        readinessVocabulary
      ),
      open_blocker_count: audit.blockers_and_risks.filter((blocker) =>
        isUnresolvedRiskStatus(blocker.status)
      ).length,
      unresolved_critical_high_blocker_count: audit.blockers_and_risks.filter(
        (blocker) =>
          isUnresolvedRiskStatus(blocker.status) &&
          new Set(["critical", "high"]).has(blocker.severity)
      ).length,
      open_architecture_decision_count: audit.architecture_decisions.filter(
        (decision) =>
          decision.status !== "approved" &&
          decision.status !== "rejected" &&
          decision.status !== "superseded"
      ).length,
      accepted_audit_report_count: 0,
      accepted_validation_report_count: 0,
      derived: true,
    },
    vocabularies: deepCopy(audit.vocabularies),
    migration_provenance: {
      legacy_v1: {
        schema_version: v1.schema_version,
        source: { path: V1_PATH, ...deepCopy(sourceIdentity) },
        provenance: deepCopy(v1.provenance),
        vocabularies: deepCopy(v1.vocabularies),
        summary_canonical_sha256: sha256(canonicalize(v1.summary)),
        requirements_canonical_sha256: sha256(canonicalize(v1.requirements)),
      },
      transformed_keys: [...TRANSFORMED_V1_KEYS],
      added_keys: [...ADDED_V2_KEYS],
      preserved_keys: [...PRESERVED_V1_KEYS],
      preserved_ledger_digests: preservedDigests,
    },
    coverage_contract: {
      requirement_count: transformedRequirements.length,
      section_counts: sectionCounts(transformedRequirements),
      requirement_id_sha256: audit.digests?.requirement_id_sha256,
      clause_id_sha256: audit.digests?.clause_id_sha256,
      status_correction_count: changedStatusCorrections(transformedRequirements)
        .length,
      sections_7_3_through_7_7_status_correction_subtotal:
        statusCorrectionSubtotal(transformedRequirements),
      sections_7_3_through_7_7_non_status_correction_manifest: deepCopy(
        EXPECTED_73_77_NON_STATUS_CORRECTIONS
      ),
      sections_7_3_through_7_7_identified_correction_count:
        statusCorrectionSubtotal(transformedRequirements) +
        EXPECTED_73_77_NON_STATUS_CORRECTIONS.length,
    },
    source_audit: {
      audit_id: audit.audit_id,
      subject_sha: audit.subject?.source_commit,
      artifact: deepCopy(auditIdentity),
      canonical_payload_sha256: audit.digests?.canonical_payload_sha256,
      review_state: audit.review?.state,
    },
    blockers: deepCopy(audit.blockers_and_risks),
    architecture_decisions: audit.architecture_decisions.map((decision) => ({
      ...deepCopy(decision),
      superseded_by_id: null,
    })),
    audit_reports: [],
    validation_reports: [],
    requirements: transformedRequirements,
    ...preserved,
  };
  output.lifecycle = deriveLifecycleManifest(output, audit, null);
  return output;
}

function runGit(root, args) {
  const result = spawnSync("git", args, {
    ...gitProcessOptions(root),
    encoding: "utf8",
    maxBuffer: 256 * 1024 * 1024,
  });
  if (result.status !== 0)
    throw new Error(`Git command failed: git ${args[0]}.`);
  return result.stdout.trim();
}

function isAncestor(root, ancestor, descendant) {
  const result = spawnSync(
    "git",
    ["merge-base", "--is-ancestor", ancestor, descendant],
    gitProcessOptions(root)
  );
  if (result.status === 0) return true;
  if (result.status === 1) return false;
  throw new Error("Git ancestry check failed.");
}

function runGitBytes(root, args) {
  const result = spawnSync("git", args, {
    ...gitProcessOptions(root),
    encoding: null,
    maxBuffer: 256 * 1024 * 1024,
  });
  if (result.status !== 0)
    throw new Error(`Git command failed: git ${args[0]}.`);
  return Buffer.from(result.stdout);
}

function pathEntry(root, commit, repositoryPath) {
  const output = runGitBytes(root, [
    "ls-tree",
    "-z",
    commit,
    "--",
    repositoryPath,
  ]);
  if (!output.length) return null;
  const tab = output.indexOf(0x09);
  const nul = output.indexOf(0, tab + 1);
  if (
    tab < 0 ||
    nul !== output.length - 1 ||
    !output.subarray(tab + 1, nul).equals(Buffer.from(repositoryPath))
  )
    throw new Error("Git tree returned an ambiguous audit path.");
  const header = output.subarray(0, tab).toString("ascii");
  const match = /^(100644|100755) blob ([0-9a-f]{40,64})$/u.exec(header);
  if (!match) throw new Error("Git audit path is not a regular blob.");
  return { mode: match[1], oid: match[2] };
}

function pathBlob(root, commit, repositoryPath) {
  const entry = pathEntry(root, commit, repositoryPath);
  if (!entry) throw new Error("Git audit path is missing.");
  return entry.oid;
}

export function deriveDurableAuditIdentity(
  repositoryRoot,
  repositoryPath = AUDIT_PATH
) {
  repositoryRoot = realpathSync(repositoryRoot);
  repositoryPath = normalizeRepositoryPath(repositoryPath);
  const absolute = path.join(repositoryRoot, repositoryPath);
  assertNoSymlinkComponents(absolute);
  const stats = lstatSync(absolute);
  if (!stats.isFile() || stats.isSymbolicLink())
    throw new Error("Durable audit must be a regular non-symlink file.");
  const head = runGit(repositoryRoot, [
    "rev-parse",
    "--verify",
    "HEAD^{commit}",
  ]);
  if (!SAFE_SHA.test(head)) throw new Error("HEAD is not one full commit SHA.");
  if (
    runGit(repositoryRoot, ["rev-parse", "--is-shallow-repository"]) !== "false"
  )
    throw new Error(
      "Durable audit identity requires complete, non-shallow history."
    );
  const status = runGit(repositoryRoot, [
    "status",
    "--porcelain=v1",
    "--untracked-files=all",
    "--",
    repositoryPath,
  ]);
  if (status)
    throw new Error(
      "Durable audit path is dirty, conflicted, staged, or untracked."
    );
  const headBlob = pathBlob(repositoryRoot, head, repositoryPath);
  const headBytes = runGitBytes(repositoryRoot, [
    "show",
    `${head}:${repositoryPath}`,
  ]);
  const worktreeBytes = readFileSync(absolute);
  if (!headBytes.equals(worktreeBytes))
    throw new Error("HEAD and worktree durable-audit bytes differ.");
  const history = runGit(repositoryRoot, [
    "rev-list",
    "--parents",
    "--topo-order",
    head,
  ])
    .split("\n")
    .filter(Boolean)
    .map((line) => line.split(" "));
  const candidates = [];
  for (const [commit, ...parents] of history) {
    const commitEntry = pathEntry(repositoryRoot, commit, repositoryPath);
    if (commitEntry?.oid !== headBlob) continue;
    const parentEntries = parents.map((parent) =>
      pathEntry(repositoryRoot, parent, repositoryPath)
    );
    if (
      parents.length === 0 ||
      parentEntries.every((entry) => entry?.oid !== headBlob)
    )
      candidates.push(commit);
  }
  const maximal = candidates.filter(
    (candidate) =>
      !candidates.some(
        (other) =>
          other !== candidate && isAncestor(repositoryRoot, candidate, other)
      )
  );
  if (maximal.length !== 1)
    throw new Error(
      "Durable audit has no unique maximal introducing ancestor."
    );
  const introducedBy = maximal[0];
  const introducedBlob = pathBlob(repositoryRoot, introducedBy, repositoryPath);
  if (introducedBlob !== headBlob)
    throw new Error("Introducing ancestor bytes differ from HEAD.");
  const parsed = parseCanonicalJson(worktreeBytes);
  return {
    path: repositoryPath,
    introducing_commit_sha: introducedBy,
    blob_oid: headBlob,
    file_sha256: sha256(worktreeBytes),
    canonical_semantic_sha256: sha256(canonicalize(parsed)),
  };
}

function parsePathVersion(repositoryRoot, version) {
  if (version.oid === null) return { ...version, kind: "deleted" };
  const bytes = runGitBytes(repositoryRoot, ["cat-file", "blob", version.oid]);
  try {
    const value = parseCanonicalJson(bytes);
    if (value?.schema_version !== "2.0") {
      // pre_v2 artifacts are only classified by kind for epoch checks and
      // predecessor nulling. Drop multi-megabyte bytes/value graphs so lifecycle
      // walks do not pin every historical v1 parse in RSS.
      return { ...version, kind: "pre_v2" };
    }
    if (!bytes.equals(Buffer.from(prettyJson(value))))
      return { ...version, kind: "invalid", bytes };
    return {
      ...version,
      kind: "v2",
      bytes,
      value,
    };
  } catch {
    return { ...version, kind: "invalid", bytes };
  }
}

function firstParentPathVersions(repositoryRoot, repositoryPath) {
  repositoryRoot = realpathSync(repositoryRoot);
  repositoryPath = normalizeRepositoryPath(repositoryPath);
  if (
    runGit(repositoryRoot, ["rev-parse", "--is-shallow-repository"]) !== "false"
  )
    throw new Error(
      "Lifecycle history requires a complete non-shallow repository."
    );
  const historyLimit = 10_000;
  const commits = runGit(repositoryRoot, [
    "rev-list",
    "--first-parent",
    `--max-count=${historyLimit + 1}`,
    "HEAD",
    "--",
    repositoryPath,
  ])
    .split("\n")
    .filter(Boolean);
  if (commits.length > historyLimit)
    throw new Error(
      `Lifecycle first-parent path-version history exceeds the ${historyLimit}-version validation bound.`
    );
  const parsedVersions = commits.map((commit_sha) =>
    parsePathVersion(repositoryRoot, {
      commit_sha,
      oid: pathEntry(repositoryRoot, commit_sha, repositoryPath)?.oid ?? null,
    })
  );
  let v2EpochStarted = false;
  for (const version of [...parsedVersions].reverse()) {
    if (version.kind === "v2") v2EpochStarted = true;
    else if (v2EpochStarted)
      throw new Error(
        "Lifecycle history is broken after schema_version 2 began."
      );
  }
  return parsedVersions;
}

export function derivePreviousV2ArtifactIdentity(
  repositoryRoot,
  candidateBytes,
  repositoryPath = V1_PATH
) {
  repositoryRoot = realpathSync(repositoryRoot);
  repositoryPath = normalizeRepositoryPath(repositoryPath);
  if (!Buffer.isBuffer(candidateBytes))
    candidateBytes = Buffer.from(candidateBytes);
  const candidate = parseCanonicalJson(candidateBytes);
  if (!candidateBytes.equals(Buffer.from(prettyJson(candidate))))
    throw new Error("Candidate v2 must use canonical pretty-JSON bytes.");
  if (candidate.schema_version !== "2.0")
    throw new Error("Candidate artifact is not schema_version 2.0.");
  const parsedVersions = firstParentPathVersions(
    repositoryRoot,
    repositoryPath
  );
  const headVersion = parsedVersions[0];
  const candidateEqualsHead = Boolean(
    headVersion?.bytes && candidateBytes.equals(headVersion.bytes)
  );
  const predecessor = parsedVersions[candidateEqualsHead ? 1 : 0];
  if (!predecessor || predecessor.kind === "pre_v2") return null;
  if (predecessor.kind !== "v2")
    throw new Error(
      "Immediate lifecycle predecessor is not a valid v2 artifact."
    );
  return {
    commit_sha: predecessor.commit_sha,
    blob_oid: predecessor.oid,
    file_sha256: sha256(predecessor.bytes),
    canonical_sha256: sha256(canonicalize(predecessor.value)),
  };
}

function defaultRepositoryRoot() {
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
}

function safeRootFromScript(scriptUrl) {
  const candidate = path.resolve(path.dirname(fileURLToPath(scriptUrl)), "..");
  const top = runGit(candidate, ["rev-parse", "--show-toplevel"]);
  if (realpathSync(top) !== realpathSync(candidate))
    throw new Error("Script does not reside in repository scripts directory.");
  return candidate;
}

function assertNoSymlinkComponents(candidate, { allowMissing = false } = {}) {
  const absolute = path.resolve(candidate);
  const { root } = path.parse(absolute);
  let current = root;
  for (const component of absolute
    .slice(root.length)
    .split(path.sep)
    .filter(Boolean)) {
    current = path.join(current, component);
    let info;
    try {
      info = lstatSync(current);
    } catch (error) {
      if (allowMissing && error?.code === "ENOENT") return absolute;
      throw error;
    }
    if (info.isSymbolicLink())
      throw new Error("Path contains a symbolic-link component.");
  }
  return absolute;
}

function assertExternalEmptyDirectory(destination, repositoryRoot) {
  const absoluteDestination = assertNoSymlinkComponents(destination);
  const info = lstatSync(absoluteDestination);
  if (!info.isDirectory() || info.isSymbolicLink())
    throw new Error(
      "Output destination must be an existing non-symlink directory."
    );
  const realDestination = realpathSync(absoluteDestination);
  const realRepository = realpathSync(repositoryRoot);
  if (
    realDestination === realRepository ||
    realDestination.startsWith(`${realRepository}${path.sep}`)
  )
    throw new Error("Output destination must be outside the repository.");
  if (readdirSync(realDestination).length)
    throw new Error("Output destination must be empty.");
  return realDestination;
}

export function transactionalTwinWrite(repositoryRoot, outputs, fsOps = {}) {
  const operations = {
    lstatSync,
    mkdirSync,
    mkdtempSync,
    renameSync,
    rmSync,
    writeFileSync,
    ...fsOps,
  };
  const root = assertNoSymlinkComponents(repositoryRoot);
  const normalized = outputs.map(([relative, bytes]) => {
    relative = normalizeRepositoryPath(relative);
    const target = path.resolve(root, relative);
    if (!target.startsWith(`${root}${path.sep}`))
      throw new Error("Transactional output escapes destination root.");
    return { relative, bytes: Buffer.from(bytes), target };
  });
  if (
    new Set(normalized.map(({ target }) => target)).size !== normalized.length
  )
    throw new Error("Transactional outputs contain duplicate targets.");
  for (const { target } of normalized) {
    assertNoSymlinkComponents(path.dirname(target), { allowMissing: true });
    operations.mkdirSync(path.dirname(target), { recursive: true });
    assertNoSymlinkComponents(path.dirname(target));
    try {
      const info = operations.lstatSync(target);
      if (!info.isFile() || info.isSymbolicLink())
        throw new Error(
          "Transactional target is not a regular non-symlink file."
        );
    } catch (error) {
      if (error?.code !== "ENOENT") throw error;
    }
  }
  const transaction = operations.mkdtempSync(
    path.join(root, ".synara-traceability-write-")
  );
  const state = [];
  try {
    for (const item of normalized) {
      item.staged = path.join(
        transaction,
        `${sha256(Buffer.from(item.relative))}.staged`
      );
      item.backup = path.join(
        transaction,
        `${sha256(Buffer.from(item.relative))}.backup`
      );
      item.hadOriginal = false;
      item.installed = false;
      operations.writeFileSync(item.staged, item.bytes);
      state.push(item);
    }
    for (const item of state) {
      try {
        operations.lstatSync(item.target);
        operations.renameSync(item.target, item.backup);
        item.hadOriginal = true;
      } catch (error) {
        if (error?.code !== "ENOENT") throw error;
      }
      operations.renameSync(item.staged, item.target);
      item.installed = true;
    }
  } catch (error) {
    const rollbackErrors = [];
    for (const item of [...state].reverse()) {
      try {
        if (item.installed) operations.rmSync(item.target, { force: true });
        if (item.hadOriginal) operations.renameSync(item.backup, item.target);
      } catch (rollbackError) {
        rollbackErrors.push(rollbackError);
      }
    }
    if (rollbackErrors.length)
      throw internalError("Transactional write and rollback both failed.");
    throw error;
  } finally {
    operations.rmSync(transaction, { recursive: true, force: true });
  }
}

async function loadJsonAt(root, relative) {
  const source = readFileSync(path.join(root, relative));
  const value = parseCanonicalJson(source);
  if (!source.equals(Buffer.from(prettyJson(value))))
    throw new Error(`${relative} is not canonical committed pretty JSON.`);
  return value;
}

function cliFailure(error, stderr) {
  const safeMessage = (
    error?.code === "ENOENT"
      ? "Required authoritative input is missing."
      : String(error?.message ?? "Traceability tooling failed.")
  )
    .replaceAll(/\/(?:private\/tmp|tmp|Users|home)\/[^\s:]+/gu, "<local-path>")
    .replaceAll(/[A-Za-z]:[\\/][^\s:]+/gu, "<local-path>");
  stderr.write(`${safeMessage}\n`);
  return error?.code === "CLI_USAGE" ? 2 : error?.code === "INTERNAL" ? 3 : 1;
}

function usage(message) {
  const error = new Error(message);
  error.code = "CLI_USAGE";
  return error;
}

export async function runCli(
  argv,
  {
    kind,
    scriptUrl = import.meta.url,
    cwd,
    stdin = process.stdin,
    stdout = process.stdout,
    stderr = process.stderr,
    repositoryRoot,
    gitObjects,
    fsOps,
  } = {}
) {
  void stdin;
  try {
    const root = repositoryRoot ?? cwd ?? safeRootFromScript(scriptUrl);
    let externalDestination;
    const checkOnly = new Set(["check-audit", "check-v2"]);
    if (checkOnly.has(kind) && argv.length)
      throw usage("Checker accepts no arguments.");
    if (
      new Set(["generate-audit", "generate-v2"]).has(kind) &&
      !(argv.length === 1 && new Set(["--check", "--write"]).has(argv[0]))
    )
      throw usage("Generator requires exactly --check or --write.");
    if (
      kind === "migrate" &&
      !(
        (argv.length === 1 && new Set(["--check", "--write"]).has(argv[0])) ||
        (argv.length === 2 && argv[0] === "--output-dir")
      )
    )
      throw usage(
        "Migrator requires exactly --check, --write, or --output-dir <dir>."
      );
    if (kind === "migrate" && argv[0] === "--output-dir") {
      try {
        externalDestination = assertExternalEmptyDirectory(argv[1], root);
      } catch {
        throw usage(
          "Migrator output destination must be an existing empty non-symlink directory outside the repository."
        );
      }
    }

    if (kind === "check-audit" || kind === "generate-audit") {
      const audit = await loadJsonAt(root, AUDIT_PATH);
      const reader = gitObjects ?? new GitObjectReader(root);
      let errors;
      try {
        errors = await validateAudit(audit, {
          gitObjects: reader,
          repoRoot: root,
        });
      } finally {
        if (!gitObjects) await reader.close();
      }
      if (errors.length) throw new Error(formatDiagnostics(errors));
      const markdown = renderAuditMarkdown(audit).bytes;
      if (kind === "check-audit" || argv[0] === "--check") {
        if (
          !readFileSync(path.join(root, AUDIT_MARKDOWN_PATH)).equals(markdown)
        )
          throw new Error("Generated audit Markdown is stale.");
      } else
        transactionalTwinWrite(
          root,
          [
            [AUDIT_PATH, Buffer.from(prettyJson(audit))],
            [AUDIT_MARKDOWN_PATH, markdown],
          ],
          fsOps
        );
      stdout.write("Feature-parity audit normalization: PASS\n");
      return 0;
    }

    if (kind === "check-v2" || kind === "generate-v2") {
      const value = await loadJsonAt(root, V1_PATH);
      const audit = await loadJsonAt(root, AUDIT_PATH);
      const identity = deriveDurableAuditIdentity(root);
      const reader = gitObjects ?? new GitObjectReader(root);
      let errors;
      try {
        const auditErrors = await validateAudit(audit, {
          gitObjects: reader,
          repoRoot: root,
        });
        if (auditErrors.length) throw new Error(formatDiagnostics(auditErrors));
        errors = await validateTraceability(value, {
          gitObjects: reader,
          repoRoot: root,
          durableAuditIdentity: identity,
          durableAudit: audit,
        });
      } finally {
        if (!gitObjects) await reader.close();
      }
      if (errors.length) throw new Error(formatDiagnostics(errors));
      const markdown = renderTraceabilityMarkdown(value).bytes;
      if (kind === "check-v2" || argv[0] === "--check") {
        if (!readFileSync(path.join(root, V1_MARKDOWN_PATH)).equals(markdown))
          throw new Error("Generated v2 Markdown is stale.");
      } else
        transactionalTwinWrite(
          root,
          [
            [V1_PATH, Buffer.from(prettyJson(value))],
            [V1_MARKDOWN_PATH, markdown],
          ],
          fsOps
        );
      stdout.write("Feature-parity traceability v2: PASS\n");
      return 0;
    }

    if (kind === "migrate") {
      const sourceBytes = runGitBytes(root, [
        "show",
        `${AUDITED_SOURCE_COMMIT}:${V1_PATH}`,
      ]);
      if (
        sha256(sourceBytes) !== V1_SHA256 ||
        pathBlob(root, AUDITED_SOURCE_COMMIT, V1_PATH) !== V1_BLOB
      )
        throw new Error("Pinned v1 source identity differs.");
      const v1 = parseCanonicalJson(sourceBytes);
      const audit = await loadJsonAt(root, AUDIT_PATH);
      const auditIdentity = deriveDurableAuditIdentity(root);
      const reader = gitObjects ?? new GitObjectReader(root);
      let output;
      try {
        const auditErrors = await validateAudit(audit, {
          gitObjects: reader,
          repoRoot: root,
        });
        if (auditErrors.length) throw new Error(formatDiagnostics(auditErrors));
        output = migrateV1({
          v1,
          audit,
          sourceIdentity: {
            commit_sha: AUDITED_SOURCE_COMMIT,
            blob_oid: V1_BLOB,
            file_sha256: V1_SHA256,
          },
          auditIdentity,
        });
        const outputErrors = await validateTraceability(output, {
          gitObjects: reader,
          repoRoot: root,
          durableAuditIdentity: auditIdentity,
          durableAudit: audit,
          historyMode: "initial-migration",
        });
        if (outputErrors.length)
          throw new Error(formatDiagnostics(outputErrors));
      } finally {
        if (!gitObjects) await reader.close();
      }
      const json = Buffer.from(prettyJson(output));
      const markdown = renderTraceabilityMarkdown(output).bytes;
      if (argv[0] === "--check") {
        if (
          !readFileSync(path.join(root, V1_PATH)).equals(json) ||
          !readFileSync(path.join(root, V1_MARKDOWN_PATH)).equals(markdown)
        )
          throw new Error(
            "Authoritative v2 traceability output differs from deterministic migration."
          );
      } else if (argv[0] === "--write") {
        transactionalTwinWrite(
          root,
          [
            [V1_PATH, json],
            [V1_MARKDOWN_PATH, markdown],
          ],
          fsOps
        );
      } else {
        try {
          assertExternalEmptyDirectory(externalDestination, root);
        } catch {
          throw usage(
            "Migrator output destination changed and is no longer a safe empty directory."
          );
        }
        transactionalTwinWrite(
          externalDestination,
          [
            [path.basename(V1_PATH), json],
            [path.basename(V1_MARKDOWN_PATH), markdown],
          ],
          fsOps
        );
      }
      stdout.write(
        `Feature-parity v2 migration: PASS ${sha256(json)} ${sha256(
          markdown
        )}\n`
      );
      return 0;
    }
    throw usage("Unknown traceability command kind.");
  } catch (error) {
    return cliFailure(error, stderr);
  }
}

export function benchmarkResult({
  elapsedMs,
  peakRssBytes,
  peakRssDeltaBytes,
  gitMetrics,
  outputDigests,
}) {
  // Budget is incremental RSS during the measured operation so CI process
  // baseline (Node + loaded modules) is not billed against the 512 MiB cap.
  const deltaBytes =
    typeof peakRssDeltaBytes === "number" ? peakRssDeltaBytes : peakRssBytes;
  return {
    elapsed_ms: Math.round(elapsedMs),
    peak_rss_bytes: peakRssBytes,
    peak_rss_delta_bytes: deltaBytes,
    within_time_budget: elapsedMs <= 30_000,
    within_rss_budget: deltaBytes <= 512 * 1024 * 1024,
    git: { ...gitMetrics },
    output_digests: [...outputDigests],
  };
}
