#!/usr/bin/env node

import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  REVIEW_REPORT_SCHEMA_ID,
  REVIEW_REPORT_ROOT,
  TASK_PACKET_SCHEMA_ID,
  TASK_PACKET_ROOT,
  auditSupportedSchema,
  discoverCanonicalRoot,
  formatDiagnostics,
  inventoryTrackedJsonFiles,
  parseJsonStrict,
  validateCanonicalInstanceSchemaId,
  validateProductionReview,
  validateReviewReport,
  validateSchemaInstance,
  validateTaskPacket,
} from "./lib/matrix-rust-governance.mjs";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDirectory, "..");
const contracts = [
  {
    kind: "task packet",
    schema: "docs/matrix-rust-sdk/schemas/native-agent-task-packet.schema.json",
    template: "docs/matrix-rust-sdk/native-agent-task-packet-template.md",
    schemaId: TASK_PACKET_SCHEMA_ID,
    templateMarkers: [
      "writer.git_pr_authority",
      "feature/matrix-rust-sdk-full-replacement",
      "scope-dependency",
      "authority-approval",
      "orchestrator alone",
    ],
  },
  {
    kind: "review report",
    schema: "docs/matrix-rust-sdk/schemas/review-report.schema.json",
    template: "docs/matrix-rust-sdk/review-report-template.md",
    schemaId: REVIEW_REPORT_SCHEMA_ID,
    templateMarkers: [
      "packet_conformance",
      "reviewed_base_sha",
      "exact base/head",
      "accepted-risk",
      "implementation writer",
    ],
  },
];
function relative(file) {
  return path.relative(repositoryRoot, file).replaceAll(path.sep, "/");
}

function add(errors, code, file, message) {
  errors.push({ code, path: relative(file), message });
}

function inspectClosedObjects(node, location, errors, file) {
  if (!node || typeof node !== "object") return;
  if (
    node.type === "object" &&
    node.properties &&
    node.additionalProperties !== false
  ) {
    add(
      errors,
      "SCHEMA_OBJECT_OPEN",
      file,
      `${location} must set additionalProperties to false.`
    );
  }
  for (const [key, child] of Object.entries(node)) {
    if (typeof child === "object")
      inspectClosedObjects(child, `${location}.${key}`, errors, file);
  }
}

async function parseJson(file, errors) {
  try {
    return parseJsonStrict(await readFile(file, "utf8"));
  } catch (error) {
    add(
      errors,
      /Duplicate JSON object key/u.test(error?.message ?? "")
        ? "JSON_DUPLICATE_KEY"
        : "JSON_PARSE",
      file,
      "JSON is malformed or contains duplicate object keys."
    );
    return null;
  }
}

function nearGovernanceSchemaId(value) {
  return (
    typeof value === "string" &&
    value.includes("synara.invalid/schemas/matrix-rust-sdk/") &&
    (value.includes("task-packet") || value.includes("review-report"))
  );
}

async function main() {
  const errors = [];
  const schemasById = new Map();
  if (process.argv.length !== 2) {
    errors.push({
      code: "ARGUMENTS_FORBIDDEN",
      path: "scripts/check-matrix-rust-governance-artifacts.mjs",
      message: "This checker accepts no arguments or bypass flags.",
    });
  }

  for (const contract of contracts) {
    const schemaFile = path.join(repositoryRoot, contract.schema);
    const templateFile = path.join(repositoryRoot, contract.template);
    const schema = await parseJson(schemaFile, errors);
    if (schema) {
      schemasById.set(contract.schemaId, schema);
      if (schema.$schema !== "https://json-schema.org/draft/2020-12/schema")
        add(
          errors,
          "SCHEMA_DRAFT",
          schemaFile,
          `${contract.kind} schema must use Draft 2020-12.`
        );
      if (schema.$id !== contract.schemaId)
        add(
          errors,
          "SCHEMA_ID",
          schemaFile,
          `${contract.kind} schema ID is not exact.`
        );
      if (schema.additionalProperties !== false)
        add(
          errors,
          "SCHEMA_ROOT_OPEN",
          schemaFile,
          `${contract.kind} root schema must be closed.`
        );
      inspectClosedObjects(schema, "$", errors, schemaFile);
      for (const error of auditSupportedSchema(schema))
        add(errors, error.code, schemaFile, `${error.path}: ${error.message}`);
    }
    let template = "";
    try {
      template = await readFile(templateFile, "utf8");
    } catch {
      add(
        errors,
        "TEMPLATE_READ",
        templateFile,
        `${contract.kind} template cannot be read.`
      );
    }
    for (const marker of contract.templateMarkers) {
      if (!template.includes(marker))
        add(
          errors,
          "TEMPLATE_CONTRACT",
          templateFile,
          `Required contract marker is absent: ${marker}`
        );
    }
  }

  const packetDiscovery = discoverCanonicalRoot(
    repositoryRoot,
    TASK_PACKET_ROOT,
    ".task-packet.json",
    "task packet"
  );
  const reportDiscovery = discoverCanonicalRoot(
    repositoryRoot,
    REVIEW_REPORT_ROOT,
    ".review-report.json",
    "review report"
  );
  errors.push(...packetDiscovery.errors, ...reportDiscovery.errors);
  const packetFiles = packetDiscovery.files;
  const reportFiles = reportDiscovery.files;
  const canonicalFiles = new Set([...packetFiles, ...reportFiles]);
  const instances = [];
  for (const file of packetFiles) {
    const value = await parseJson(file, errors);
    if (!value) continue;
    const schemaErrors = validateCanonicalInstanceSchemaId(
      "task packet",
      value
    );
    if (schemaErrors.length) {
      for (const error of schemaErrors)
        add(errors, error.code, file, `${error.path}: ${error.message}`);
      continue;
    }
    instances.push({ file, value });
  }
  for (const file of reportFiles) {
    const value = await parseJson(file, errors);
    if (!value) continue;
    const schemaErrors = validateCanonicalInstanceSchemaId(
      "review report",
      value
    );
    if (schemaErrors.length) {
      for (const error of schemaErrors)
        add(errors, error.code, file, `${error.path}: ${error.message}`);
      continue;
    }
    instances.push({ file, value });
  }
  const authoritativeSchemas = new Set(
    contracts.map((contract) => path.join(repositoryRoot, contract.schema))
  );
  const inventory = inventoryTrackedJsonFiles(repositoryRoot);
  errors.push(...inventory.errors);
  for (const file of inventory.files) {
    if (canonicalFiles.has(file) || authoritativeSchemas.has(file)) continue;
    let value;
    try {
      value = parseJsonStrict(await readFile(file, "utf8"));
    } catch {
      continue;
    }
    if (nearGovernanceSchemaId(value?.schema_id))
      add(
        errors,
        "INSTANCE_OUTSIDE_CANONICAL_ROOT",
        file,
        "Governance schema_id is forbidden outside canonical instance roots."
      );
  }
  const packetsByFile = new Map(
    instances
      .filter(({ value }) => value.schema_id === TASK_PACKET_SCHEMA_ID)
      .map(({ file, value }) => [file, value])
  );
  for (const instance of instances) {
    for (const error of validateSchemaInstance(
      schemasById.get(instance.value.schema_id),
      instance.value
    ))
      add(errors, error.code, instance.file, `${error.path}: ${error.message}`);
    if (instance.value.schema_id === TASK_PACKET_SCHEMA_ID) {
      for (const error of validateTaskPacket(instance.value))
        add(
          errors,
          error.code,
          instance.file,
          `${error.path}: ${error.message}`
        );
      continue;
    }
    const packetFile =
      typeof instance.value.task_packet_path === "string"
        ? path.join(repositoryRoot, instance.value.task_packet_path)
        : null;
    const packet = packetFile ? packetsByFile.get(packetFile) : null;
    if (!packet || packet.schema_id !== TASK_PACKET_SCHEMA_ID) {
      add(
        errors,
        "REPORT_PACKET",
        instance.file,
        "Review report task_packet_path does not resolve to an exact-schema task packet inside the repository."
      );
      continue;
    }
    for (const error of validateReviewReport(instance.value, packet))
      add(errors, error.code, instance.file, `${error.path}: ${error.message}`);
    for (const error of validateProductionReview(
      instance.value,
      packet,
      repositoryRoot
    ))
      add(errors, error.code, instance.file, `${error.path}: ${error.message}`);
  }

  const output = formatDiagnostics(errors);
  if (output.length) {
    process.stderr.write(`${output.join("\n")}\n`);
    process.exitCode = 1;
  } else {
    if (instances.length === 0)
      process.stdout.write(
        "Matrix Rust governance contracts are valid; no governed-instance acceptance is claimed because canonical roots contain zero instances.\n"
      );
    else
      process.stdout.write(
        `Matrix Rust governance contracts and ${
          instances.length
        } canonical governed instance${
          instances.length === 1 ? "" : "s"
        } are valid.\n`
      );
  }
}

await main();
