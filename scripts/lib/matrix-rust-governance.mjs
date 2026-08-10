import { lstatSync, readdirSync, realpathSync } from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

export const TASK_PACKET_SCHEMA_ID =
  "https://synara.invalid/schemas/matrix-rust-sdk/native-agent-task-packet.schema.json";
export const REVIEW_REPORT_SCHEMA_ID =
  "https://synara.invalid/schemas/matrix-rust-sdk/review-report.schema.json";
export const INTEGRATION_BRANCH = "feature/matrix-rust-sdk-full-replacement";
export const UPSTREAM_REPOSITORY_URL =
  "https://github.com/matrix-org/matrix-rust-sdk";
export const TASK_PACKET_ROOT = "docs/matrix-rust-sdk/governance/task-packets";
export const REVIEW_REPORT_ROOT =
  "docs/matrix-rust-sdk/governance/review-reports";

const SHA = /^[0-9a-f]{40}$/;
const STOP_CATEGORIES = [
  "authority-approval",
  "git-base-target",
  "invariant",
  "prerequisite-validation-evidence",
  "scope-dependency",
  "upstream-architecture",
];
const WRITER_AUTHORITIES = [
  "commit",
  "delete_unrelated_files",
  "merge_pr",
  "modify_program_plan",
  "open_pr",
  "push",
  "rebase",
  "switch_branch",
];
const EXTERNAL_RISK_ROLES = new Set([
  "product_owner",
  "program_owner",
  "security_owner",
  "user",
]);

function diagnostic(code, path, message) {
  return { code, path, message };
}

function sorted(diagnostics) {
  return diagnostics.sort((left, right) =>
    `${left.path}\0${left.code}\0${left.message}`.localeCompare(
      `${right.path}\0${right.code}\0${right.message}`
    )
  );
}

function list(value) {
  return Array.isArray(value) ? value : [];
}

function text(value) {
  return typeof value === "string" ? value.trim() : "";
}

function sameSet(left, right) {
  const a = [...new Set(list(left))].sort();
  const b = [...new Set(list(right))].sort();
  return a.length === b.length && a.every((value, index) => value === b[index]);
}

function duplicates(values, key = (value) => value) {
  const seen = new Set();
  const repeated = new Set();
  for (const value of list(values)) {
    const id = key(value);
    if (seen.has(id)) repeated.add(id);
    else seen.add(id);
  }
  return [...repeated].sort();
}

export function normalizeRepositoryPath(value) {
  if (typeof value !== "string" || value !== value.trim()) return null;
  if (!value || value !== value.normalize("NFC")) return null;
  if (/[\\\u0000-\u001f\u007f]/u.test(value)) return null;
  if (
    value.startsWith("/") ||
    value.startsWith("//") ||
    /^[A-Za-z]:/u.test(value) ||
    value.endsWith("/")
  )
    return null;
  const segments = value.split("/");
  if (
    segments.some((segment) => !segment || segment === "." || segment === "..")
  )
    return null;
  return value;
}

function normalizeScopeEntry(value) {
  if (
    !value ||
    typeof value !== "object" ||
    Array.isArray(value) ||
    !["file", "directory"].includes(value.kind)
  )
    return null;
  const normalized = normalizeRepositoryPath(value.path);
  return normalized ? { path: normalized, kind: value.kind } : null;
}

function scopeKey(value) {
  const entry = normalizeScopeEntry(value);
  return entry ? `${entry.kind}:${entry.path}` : "";
}

function sameTypedScope(left, right) {
  const a = list(left).map(scopeKey).sort();
  const b = list(right).map(scopeKey).sort();
  return (
    a.length === b.length &&
    a.every((value, index) => value && value === b[index]) &&
    duplicates(a).length === 0 &&
    duplicates(b).length === 0
  );
}

function pathMatchesScope(pathValue, scopeValue) {
  const candidate = normalizeRepositoryPath(pathValue);
  const scope = normalizeScopeEntry(scopeValue);
  if (!candidate || !scope) return false;
  return scope.kind === "file"
    ? candidate === scope.path
    : candidate === scope.path || candidate.startsWith(`${scope.path}/`);
}

function normalizedPaths(values) {
  return list(values).map(normalizeRepositoryPath).filter(Boolean);
}

function canonicalPrincipal(value) {
  return text(value).normalize("NFKC").toLocaleLowerCase("en-US");
}

function validDate(value) {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/u.exec(value ?? "");
  if (!match) return false;
  const [, year, month, day] = match.map(Number);
  const date = new Date(Date.UTC(year, month - 1, day));
  return (
    date.getUTCFullYear() === year &&
    date.getUTCMonth() === month - 1 &&
    date.getUTCDate() === day
  );
}

function dateTimeEpoch(value) {
  const match =
    /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.(\d+))?(Z|([+-])(\d{2}):(\d{2}))$/u.exec(
      value ?? ""
    );
  if (!match) return null;
  const [, y, mo, d, h, mi, s, fraction = "", zone, sign, oh, om] = match;
  const year = Number(y);
  const month = Number(mo);
  const day = Number(d);
  const hour = Number(h);
  const minute = Number(mi);
  const second = Number(s);
  if (!validDate(`${y}-${mo}-${d}`) || hour > 23 || minute > 59 || second > 59)
    return null;
  let offset = 0;
  if (zone !== "Z") {
    const offsetHour = Number(oh);
    const offsetMinute = Number(om);
    if (
      offsetHour > 14 ||
      offsetMinute > 59 ||
      (offsetHour === 14 && offsetMinute !== 0)
    )
      return null;
    offset = (offsetHour * 60 + offsetMinute) * (sign === "+" ? 1 : -1);
  }
  const milliseconds = Number(`0.${fraction || "0"}`) * 1000;
  return (
    Date.UTC(year, month - 1, day, hour, minute, second, milliseconds) -
    offset * 60_000
  );
}

function utcDate(epoch) {
  return new Date(epoch).toISOString().slice(0, 10);
}

const SYNARA_PR_URL =
  /^https:\/\/github\.com\/nepenth\/synara-desktop\/pull\/([1-9]\d*)$/u;
const SYNARA_DURABLE_URL =
  /^https:\/\/github\.com\/nepenth\/synara-desktop\/(?:pull\/\d+(?:#pullrequestreview-\d+)?|issues\/\d+)$/u;
const SYNARA_REVIEW_URL =
  /^https:\/\/github\.com\/nepenth\/synara-desktop\/pull\/([1-9]\d*)#pullrequestreview-[1-9]\d*$/u;
const SYNARA_CI_URL =
  /^https:\/\/github\.com\/nepenth\/synara-desktop\/actions\/runs\/[1-9]\d*(?:\/job\/[1-9]\d*)?$/u;

function repositoryReference(value) {
  if (typeof value !== "string") return null;
  const marker = value.indexOf("#");
  if (marker <= 0 || marker === value.length - 1) return null;
  const file = normalizeRepositoryPath(value.slice(0, marker));
  const fragment = value.slice(marker + 1);
  if (!file || !/^[A-Za-z0-9._:-]+$/u.test(fragment)) return null;
  return { file, fragment };
}

function durableRiskReference(value) {
  return SYNARA_DURABLE_URL.test(value ?? "") || repositoryReference(value);
}

function pinnedUpstreamPermalink(value, commitSha) {
  if (typeof value !== "string" || !SHA.test(commitSha ?? "")) return false;
  if (
    /(?:^|\/)(?:\.{1,2})(?:\/|$)/u.test(value) ||
    /%(?:2e|2f|5c)/iu.test(value) ||
    value.includes("\\")
  )
    return false;
  try {
    const parsed = new URL(value);
    return (
      parsed.protocol === "https:" &&
      parsed.origin === "https://github.com" &&
      parsed.username === "" &&
      parsed.password === "" &&
      parsed.search === "" &&
      parsed.hash === "" &&
      parsed.pathname.startsWith(
        `/matrix-org/matrix-rust-sdk/blob/${commitSha}/`
      ) &&
      parsed.pathname.length >
        `/matrix-org/matrix-rust-sdk/blob/${commitSha}/`.length
    );
  } catch {
    return false;
  }
}

export function discoverCanonicalRoot(
  repositoryRoot,
  rootRelative,
  suffix,
  kind
) {
  const errors = [];
  const files = [];
  const normalizedRoot = normalizeRepositoryPath(rootRelative);
  if (!normalizedRoot || !suffix || !kind) {
    return {
      files,
      errors: [
        diagnostic(
          "DISCOVERY_ROOT",
          String(rootRelative),
          "Canonical discovery configuration is invalid."
        ),
      ],
    };
  }
  const root = path.join(repositoryRoot, normalizedRoot);
  let current = repositoryRoot;
  for (const segment of normalizedRoot.split("/")) {
    current = path.join(current, segment);
    try {
      const status = lstatSync(current);
      if (status.isSymbolicLink() || !status.isDirectory()) {
        errors.push(
          diagnostic(
            "DISCOVERY_ROOT",
            normalizedRoot,
            `${kind} root and every component must be real directories.`
          )
        );
        return { files, errors };
      }
    } catch (error) {
      if (error?.code === "ENOENT") return { files, errors };
      errors.push(
        diagnostic(
          "DISCOVERY_ROOT",
          normalizedRoot,
          `${kind} root cannot be inspected.`
        )
      );
      return { files, errors };
    }
  }
  const visit = (directory) => {
    const entries = readdirSync(directory, { withFileTypes: true }).sort(
      (a, b) => a.name.localeCompare(b.name)
    );
    for (const entry of entries) {
      const target = path.join(directory, entry.name);
      const location = path
        .relative(repositoryRoot, target)
        .replaceAll(path.sep, "/");
      const status = lstatSync(target);
      if (status.isSymbolicLink())
        errors.push(
          diagnostic(
            "DISCOVERY_SYMLINK",
            location,
            `${kind} entries must not be symlinks.`
          )
        );
      else if (status.isDirectory()) visit(target);
      else if (!status.isFile())
        errors.push(
          diagnostic(
            "DISCOVERY_NON_REGULAR",
            location,
            `${kind} entry must be regular.`
          )
        );
      else if (!entry.name.endsWith(suffix))
        errors.push(
          diagnostic(
            "DISCOVERY_SUFFIX",
            location,
            `${kind} file has the wrong suffix.`
          )
        );
      else files.push(target);
    }
  };
  visit(root);
  return { files, errors: sorted(errors) };
}

export function validateCanonicalInstanceSchemaId(kind, value) {
  const expected =
    kind === "task packet"
      ? TASK_PACKET_SCHEMA_ID
      : kind === "review report"
      ? REVIEW_REPORT_SCHEMA_ID
      : null;
  return value?.schema_id === expected
    ? []
    : [
        diagnostic(
          "INSTANCE_SCHEMA_ID",
          "schema_id",
          `Canonical ${kind} has a missing, unknown, or wrong schema_id.`
        ),
      ];
}

export function inventoryTrackedJsonFiles(repositoryRoot, runner = spawnSync) {
  const result = runner("git", ["ls-files", "-z", "--", "*.json"], {
    cwd: repositoryRoot,
    encoding: "utf8",
    shell: false,
    windowsHide: true,
  });
  if (result.status !== 0)
    return {
      files: [],
      errors: [
        diagnostic(
          "INVENTORY_GIT",
          "repository inventory",
          "Tracked JSON inventory could not be established."
        ),
      ],
    };
  return {
    files: result.stdout
      .split("\0")
      .filter(Boolean)
      .map((entry) => path.join(repositoryRoot, entry)),
    errors: [],
  };
}

function requiredValidationIds(packet) {
  const ids = [];
  for (const group of Object.values(packet?.validations ?? {})) {
    if (group?.required)
      ids.push(...list(group.cases).map((entry) => entry?.id));
  }
  return ids.filter(Boolean);
}

function criterionIds(packet) {
  return list(packet?.behavior?.requirements).flatMap((requirement) =>
    list(requirement?.acceptance_criteria_ids)
  );
}

export function parseJsonStrict(source) {
  if (typeof source !== "string") throw new Error("JSON source must be text.");
  let cursor = 0;
  const skip = () => {
    while (/\s/u.test(source[cursor] ?? "")) cursor += 1;
  };
  const stringToken = () => {
    const start = cursor;
    if (source[cursor] !== '"') throw new Error("JSON string expected.");
    cursor += 1;
    while (cursor < source.length) {
      const character = source[cursor];
      if (character === '"') {
        cursor += 1;
        return JSON.parse(source.slice(start, cursor));
      }
      if (character === "\\") cursor += 2;
      else cursor += 1;
    }
    throw new Error("Unterminated JSON string.");
  };
  const value = () => {
    skip();
    if (source[cursor] === "{") {
      cursor += 1;
      skip();
      const keys = new Set();
      if (source[cursor] === "}") {
        cursor += 1;
        return;
      }
      for (;;) {
        skip();
        const key = stringToken();
        if (keys.has(key)) throw new Error("Duplicate JSON object key.");
        keys.add(key);
        skip();
        if (source[cursor] !== ":") throw new Error("JSON colon expected.");
        cursor += 1;
        value();
        skip();
        if (source[cursor] === "}") {
          cursor += 1;
          return;
        }
        if (source[cursor] !== ",") throw new Error("JSON comma expected.");
        cursor += 1;
      }
    }
    if (source[cursor] === "[") {
      cursor += 1;
      skip();
      if (source[cursor] === "]") {
        cursor += 1;
        return;
      }
      for (;;) {
        value();
        skip();
        if (source[cursor] === "]") {
          cursor += 1;
          return;
        }
        if (source[cursor] !== ",") throw new Error("JSON comma expected.");
        cursor += 1;
      }
    }
    if (source[cursor] === '"') {
      stringToken();
      return;
    }
    const token =
      /^(?:true|false|null|-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?)/u.exec(
        source.slice(cursor)
      )?.[0];
    if (!token) throw new Error("JSON value expected.");
    cursor += token.length;
  };
  value();
  skip();
  if (cursor !== source.length) throw new Error("Trailing JSON content.");
  return JSON.parse(source);
}

const SUPPORTED_SCHEMA_KEYWORDS = new Set([
  "$schema",
  "$id",
  "$ref",
  "$defs",
  "$comment",
  "title",
  "description",
  "default",
  "examples",
  "deprecated",
  "readOnly",
  "writeOnly",
  "type",
  "const",
  "enum",
  "pattern",
  "format",
  "minLength",
  "maxLength",
  "minimum",
  "minItems",
  "maxItems",
  "uniqueItems",
  "items",
  "properties",
  "required",
  "additionalProperties",
  "allOf",
  "if",
  "then",
  "else",
  "not",
]);

export function auditSupportedSchema(schema) {
  const errors = [];
  const schemaTypes = new Set([
    "null",
    "boolean",
    "object",
    "array",
    "number",
    "string",
    "integer",
  ]);
  const invalid = (location, keyword, message) =>
    errors.push(
      diagnostic("SCHEMA_KEYWORD_VALUE", `${location}.${keyword}`, message)
    );
  const isSchema = (value) =>
    typeof value === "boolean" ||
    (value !== null && typeof value === "object" && !Array.isArray(value));
  const inspect = (node, location) => {
    if (typeof node === "boolean") return;
    if (!node || typeof node !== "object" || Array.isArray(node)) {
      errors.push(
        diagnostic(
          "SCHEMA_NODE",
          location,
          "Schema node must be an object or boolean."
        )
      );
      return;
    }
    for (const key of Object.keys(node)) {
      if (!SUPPORTED_SCHEMA_KEYWORDS.has(key))
        errors.push(
          diagnostic(
            "SCHEMA_KEYWORD_UNSUPPORTED",
            `${location}.${key}`,
            "Schema keyword is outside the implemented strict subset."
          )
        );
    }
    for (const key of [
      "$schema",
      "$id",
      "$ref",
      "$comment",
      "title",
      "description",
    ])
      if (node[key] !== undefined && typeof node[key] !== "string")
        invalid(location, key, `${key} must be a string.`);
    for (const key of ["deprecated", "readOnly", "writeOnly", "uniqueItems"])
      if (node[key] !== undefined && typeof node[key] !== "boolean")
        invalid(location, key, `${key} must be a boolean.`);
    for (const key of ["minLength", "maxLength", "minItems", "maxItems"])
      if (
        node[key] !== undefined &&
        (!Number.isInteger(node[key]) || node[key] < 0)
      )
        invalid(location, key, `${key} must be a nonnegative integer.`);
    if (
      node.minimum !== undefined &&
      (typeof node.minimum !== "number" || !Number.isFinite(node.minimum))
    )
      invalid(location, "minimum", "minimum must be a finite number.");
    if (
      Number.isInteger(node.minLength) &&
      Number.isInteger(node.maxLength) &&
      node.minLength > node.maxLength
    )
      invalid(location, "minLength", "minLength must not exceed maxLength.");
    if (
      Number.isInteger(node.minItems) &&
      Number.isInteger(node.maxItems) &&
      node.minItems > node.maxItems
    )
      invalid(location, "minItems", "minItems must not exceed maxItems.");
    const types = Array.isArray(node.type) ? node.type : [node.type];
    if (
      node.type !== undefined &&
      (types.length === 0 ||
        types.some((entry) => !schemaTypes.has(entry)) ||
        duplicates(types).length)
    )
      invalid(
        location,
        "type",
        "type must contain unique supported JSON Schema type names."
      );
    if (
      node.enum !== undefined &&
      (!Array.isArray(node.enum) || node.enum.length === 0)
    )
      invalid(location, "enum", "enum must be a nonempty array.");
    if (node.examples !== undefined && !Array.isArray(node.examples))
      invalid(location, "examples", "examples must be an array.");
    if (
      node.required !== undefined &&
      (!Array.isArray(node.required) ||
        node.required.some((entry) => typeof entry !== "string" || !entry) ||
        duplicates(node.required).length)
    )
      invalid(
        location,
        "required",
        "required must contain unique nonempty strings."
      );
    for (const key of ["properties", "$defs"])
      if (
        node[key] !== undefined &&
        (!node[key] ||
          typeof node[key] !== "object" ||
          Array.isArray(node[key]))
      )
        invalid(location, key, `${key} must be an object.`);
    if (node.items !== undefined && !isSchema(node.items))
      invalid(location, "items", "items must be a schema.");
    if (
      node.additionalProperties !== undefined &&
      !isSchema(node.additionalProperties)
    )
      invalid(
        location,
        "additionalProperties",
        "additionalProperties must be a schema or boolean."
      );
    if (
      node.allOf !== undefined &&
      (!Array.isArray(node.allOf) ||
        node.allOf.length === 0 ||
        node.allOf.some((entry) => !isSchema(entry)))
    )
      invalid(location, "allOf", "allOf must be a nonempty array of schemas.");
    for (const key of ["if", "then", "else", "not"])
      if (node[key] !== undefined && !isSchema(node[key]))
        invalid(location, key, `${key} must be a schema.`);
    if (node.pattern !== undefined) {
      if (typeof node.pattern !== "string")
        invalid(location, "pattern", "pattern must be a string.");
      else
        try {
          new RegExp(node.pattern, "u");
        } catch {
          invalid(
            location,
            "pattern",
            "pattern must compile as a Unicode regular expression."
          );
        }
    }
    for (const [key, child] of Object.entries(
      node.properties &&
        typeof node.properties === "object" &&
        !Array.isArray(node.properties)
        ? node.properties
        : {}
    ))
      inspect(child, `${location}.properties.${key}`);
    for (const [key, child] of Object.entries(
      node.$defs && typeof node.$defs === "object" && !Array.isArray(node.$defs)
        ? node.$defs
        : {}
    ))
      inspect(child, `${location}.$defs.${key}`);
    if (node.items !== undefined) inspect(node.items, `${location}.items`);
    if (
      node.additionalProperties &&
      typeof node.additionalProperties === "object"
    )
      inspect(node.additionalProperties, `${location}.additionalProperties`);
    for (const keyword of ["allOf"]) {
      list(node[keyword]).forEach((child, index) =>
        inspect(child, `${location}.${keyword}[${index}]`)
      );
    }
    for (const keyword of ["if", "then", "else", "not"])
      if (node[keyword] !== undefined)
        inspect(node[keyword], `${location}.${keyword}`);
    if (node.format && !["uri", "date", "date-time"].includes(node.format))
      errors.push(
        diagnostic(
          "SCHEMA_FORMAT_UNSUPPORTED",
          `${location}.format`,
          "Schema format is outside the implemented strict subset."
        )
      );
    if (
      node.$ref !== undefined &&
      (typeof node.$ref !== "string" ||
        node.$ref.length === 0 ||
        !node.$ref.startsWith("#/$defs/") ||
        !resolveLocalReference(schema, node.$ref))
    )
      errors.push(
        diagnostic(
          "SCHEMA_REF_UNSUPPORTED",
          `${location}.$ref`,
          "Only local $defs references are supported."
        )
      );
  };
  inspect(schema, "$");
  return sorted(errors);
}

function schemaTypeMatches(value, expected) {
  if (expected === "null") return value === null;
  if (expected === "array") return Array.isArray(value);
  if (expected === "object")
    return value !== null && typeof value === "object" && !Array.isArray(value);
  if (expected === "integer") return Number.isInteger(value);
  return typeof value === expected;
}

function resolveLocalReference(rootSchema, reference) {
  if (!reference.startsWith("#/")) return null;
  return reference
    .slice(2)
    .split("/")
    .reduce(
      (value, part) =>
        value?.[part.replaceAll("~1", "/").replaceAll("~0", "~")],
      rootSchema
    );
}

function schemaErrors(schema, value, rootSchema, instancePath) {
  if (typeof schema === "boolean")
    return schema
      ? []
      : [
          diagnostic(
            "SCHEMA_FALSE",
            instancePath,
            "Value is forbidden by schema."
          ),
        ];
  if (!schema || typeof schema !== "object") return [];
  const errors = [];
  if (schema.$ref !== undefined) {
    const target =
      typeof schema.$ref === "string" && schema.$ref.length > 0
        ? resolveLocalReference(rootSchema, schema.$ref)
        : null;
    if (!target)
      errors.push(
        diagnostic(
          "SCHEMA_REF",
          instancePath,
          "Schema contains an unresolved local reference."
        )
      );
    else errors.push(...schemaErrors(target, value, rootSchema, instancePath));
  }
  if (
    schema.const !== undefined &&
    JSON.stringify(value) !== JSON.stringify(schema.const)
  )
    errors.push(
      diagnostic(
        "SCHEMA_CONST",
        instancePath,
        "Value does not equal the required constant."
      )
    );
  if (
    schema.enum &&
    !schema.enum.some(
      (entry) => JSON.stringify(entry) === JSON.stringify(value)
    )
  )
    errors.push(
      diagnostic(
        "SCHEMA_ENUM",
        instancePath,
        "Value is not in the allowed enumeration."
      )
    );
  const expectedTypes =
    schema.type === undefined
      ? []
      : Array.isArray(schema.type)
      ? schema.type
      : [schema.type];
  if (
    expectedTypes.length &&
    !expectedTypes.some((expected) => schemaTypeMatches(value, expected))
  ) {
    errors.push(
      diagnostic("SCHEMA_TYPE", instancePath, "Value has the wrong JSON type.")
    );
    return errors;
  }
  if (typeof value === "string") {
    if (schema.minLength !== undefined && value.length < schema.minLength)
      errors.push(
        diagnostic(
          "SCHEMA_MIN_LENGTH",
          instancePath,
          "String is shorter than allowed."
        )
      );
    if (schema.maxLength !== undefined && value.length > schema.maxLength)
      errors.push(
        diagnostic(
          "SCHEMA_MAX_LENGTH",
          instancePath,
          "String is longer than allowed."
        )
      );
    if (schema.pattern && !new RegExp(schema.pattern, "u").test(value))
      errors.push(
        diagnostic(
          "SCHEMA_PATTERN",
          instancePath,
          "String does not match the required pattern."
        )
      );
    if (schema.format === "uri") {
      try {
        new URL(value);
      } catch {
        errors.push(
          diagnostic(
            "SCHEMA_FORMAT",
            instancePath,
            "String is not a valid URI."
          )
        );
      }
    } else if (schema.format === "date-time" && dateTimeEpoch(value) === null) {
      errors.push(
        diagnostic(
          "SCHEMA_FORMAT",
          instancePath,
          "String is not an ISO 8601 date-time."
        )
      );
    } else if (schema.format === "date" && !validDate(value)) {
      errors.push(
        diagnostic(
          "SCHEMA_FORMAT",
          instancePath,
          "String is not an ISO 8601 date."
        )
      );
    }
  }
  if (typeof value === "number") {
    if (schema.minimum !== undefined && value < schema.minimum)
      errors.push(
        diagnostic(
          "SCHEMA_MINIMUM",
          instancePath,
          "Number is below the allowed minimum."
        )
      );
  }
  if (Array.isArray(value)) {
    if (schema.minItems !== undefined && value.length < schema.minItems)
      errors.push(
        diagnostic("SCHEMA_MIN_ITEMS", instancePath, "Array has too few items.")
      );
    if (schema.maxItems !== undefined && value.length > schema.maxItems)
      errors.push(
        diagnostic(
          "SCHEMA_MAX_ITEMS",
          instancePath,
          "Array has too many items."
        )
      );
    if (
      schema.uniqueItems &&
      new Set(value.map((entry) => JSON.stringify(entry))).size !== value.length
    )
      errors.push(
        diagnostic(
          "SCHEMA_UNIQUE_ITEMS",
          instancePath,
          "Array items must be unique."
        )
      );
    if (schema.items)
      value.forEach((entry, index) =>
        errors.push(
          ...schemaErrors(
            schema.items,
            entry,
            rootSchema,
            `${instancePath}[${index}]`
          )
        )
      );
  }
  if (value !== null && typeof value === "object" && !Array.isArray(value)) {
    for (const required of schema.required ?? []) {
      if (!Object.hasOwn(value, required))
        errors.push(
          diagnostic(
            "SCHEMA_REQUIRED",
            `${instancePath}.${required}`,
            "Required property is missing."
          )
        );
    }
    for (const [key, entry] of Object.entries(value)) {
      if (schema.properties?.[key])
        errors.push(
          ...schemaErrors(
            schema.properties[key],
            entry,
            rootSchema,
            `${instancePath}.${key}`
          )
        );
      else if (schema.additionalProperties === false)
        errors.push(
          diagnostic(
            "SCHEMA_ADDITIONAL_PROPERTY",
            `${instancePath}.${key}`,
            "Additional property is forbidden."
          )
        );
      else if (
        schema.additionalProperties &&
        typeof schema.additionalProperties === "object"
      )
        errors.push(
          ...schemaErrors(
            schema.additionalProperties,
            entry,
            rootSchema,
            `${instancePath}.${key}`
          )
        );
    }
  }
  for (const member of schema.allOf ?? [])
    errors.push(...schemaErrors(member, value, rootSchema, instancePath));
  if (schema.if) {
    const conditionMatches =
      schemaErrors(schema.if, value, rootSchema, instancePath).length === 0;
    if (conditionMatches && schema.then)
      errors.push(
        ...schemaErrors(schema.then, value, rootSchema, instancePath)
      );
    if (!conditionMatches && schema.else)
      errors.push(
        ...schemaErrors(schema.else, value, rootSchema, instancePath)
      );
  }
  if (
    schema.not &&
    schemaErrors(schema.not, value, rootSchema, instancePath).length === 0
  )
    errors.push(
      diagnostic(
        "SCHEMA_NOT",
        instancePath,
        "Value matches a forbidden schema."
      )
    );
  return errors;
}

export function validateSchemaInstance(schema, instance) {
  return sorted(schemaErrors(schema, instance, schema, "$"));
}

export function validateTaskPacket(packet) {
  const errors = [];
  if (!packet || typeof packet !== "object" || Array.isArray(packet)) {
    return [diagnostic("PACKET_OBJECT", "$", "Task packet must be an object.")];
  }
  if (packet.schema_id !== TASK_PACKET_SCHEMA_ID)
    errors.push(
      diagnostic(
        "PACKET_SCHEMA_ID",
        "schema_id",
        "Task packet schema_id is not the exact governance schema ID."
      )
    );
  if (packet.schema_version !== "1.0")
    errors.push(
      diagnostic(
        "PACKET_SCHEMA_VERSION",
        "schema_version",
        "Task packet schema_version must be 1.0."
      )
    );
  if (
    !text(packet?.writer?.identity) ||
    packet?.writer?.role !== "implementation_writer"
  )
    errors.push(
      diagnostic(
        "PACKET_WRITER",
        "writer",
        "Writer identity is required and role must be implementation_writer."
      )
    );
  for (const field of WRITER_AUTHORITIES) {
    if (packet?.writer?.git_pr_authority?.[field] !== false)
      errors.push(
        diagnostic(
          "PACKET_WRITER_AUTHORITY",
          `writer.git_pr_authority.${field}`,
          "Writer authority must be explicitly false."
        )
      );
  }
  if (packet?.git_context?.integration_branch !== INTEGRATION_BRANCH)
    errors.push(
      diagnostic(
        "PACKET_INTEGRATION_BRANCH",
        "git_context.integration_branch",
        "Integration branch must be the migration integration branch."
      )
    );
  if (packet?.git_context?.pr_target !== INTEGRATION_BRANCH)
    errors.push(
      diagnostic(
        "PACKET_PR_TARGET",
        "git_context.pr_target",
        "PR target must equal the migration integration branch and must not be main."
      )
    );
  if (!SHA.test(packet?.git_context?.base_sha ?? ""))
    errors.push(
      diagnostic(
        "PACKET_BASE_SHA",
        "git_context.base_sha",
        "Base SHA must be one exact lowercase 40-character SHA."
      )
    );
  const requiredChecks = list(packet?.git_context?.required_ci_checks);
  if (
    requiredChecks.length === 0 ||
    requiredChecks.some((entry) => !text(entry)) ||
    duplicates(requiredChecks).length
  )
    errors.push(
      diagnostic(
        "PACKET_REQUIRED_CI",
        "git_context.required_ci_checks",
        "Required CI checks must be a nonempty unique list of stable names."
      )
    );
  if (
    !text(packet?.git_context?.work_branch) ||
    packet.git_context.work_branch === INTEGRATION_BRANCH ||
    packet.git_context.work_branch === "main"
  )
    errors.push(
      diagnostic(
        "PACKET_WORK_BRANCH",
        "git_context.work_branch",
        "Work branch must be a distinct non-main task branch."
      )
    );
  for (const field of [
    "allowed_paths",
    "prohibited_paths",
    "out_of_scope_paths",
  ]) {
    const values = packet?.file_scope?.[field];
    list(values).forEach((entry, index) => {
      if (!normalizeScopeEntry(entry))
        errors.push(
          diagnostic(
            "PACKET_SCOPE_PATH",
            `file_scope.${field}[${index}]`,
            "Scope entries must be typed normalized repository paths."
          )
        );
    });
    if (duplicates(values, scopeKey).length)
      errors.push(
        diagnostic(
          "PACKET_SCOPE_DUPLICATE",
          `file_scope.${field}`,
          "Typed scope entries must be globally unique within the field."
        )
      );
  }
  list(packet?.file_scope?.generated_paths).forEach((entry, index) => {
    if (!normalizeRepositoryPath(entry))
      errors.push(
        diagnostic(
          "PACKET_GENERATED_PATH",
          `file_scope.generated_paths[${index}]`,
          "Generated paths must be exact normalized repository files."
        )
      );
  });
  const allowedScopes = list(packet?.file_scope?.allowed_paths);
  for (const generated of normalizedPaths(
    packet?.file_scope?.generated_paths
  )) {
    if (!allowedScopes.some((scope) => pathMatchesScope(generated, scope)))
      errors.push(
        diagnostic(
          "PACKET_GENERATED_OUTSIDE_SCOPE",
          "file_scope.generated_paths",
          "Every generated file must be inside typed allowed scope."
        )
      );
  }

  if (packet?.upstream_evidence?.repository_url !== UPSTREAM_REPOSITORY_URL)
    errors.push(
      diagnostic(
        "PACKET_UPSTREAM_REPOSITORY",
        "upstream_evidence.repository_url",
        "Upstream repository URL must be the exact Matrix Rust SDK repository."
      )
    );
  const upstreamSha = packet?.upstream_evidence?.commit_sha;
  for (const [index, permalink] of list(
    packet?.upstream_evidence?.permalinks
  ).entries()) {
    if (
      !SHA.test(upstreamSha ?? "") ||
      !pinnedUpstreamPermalink(permalink?.url, upstreamSha)
    )
      errors.push(
        diagnostic(
          "PACKET_UPSTREAM_PERMALINK",
          `upstream_evidence.permalinks[${index}].url`,
          "Every upstream permalink must pin the packet commit SHA."
        )
      );
  }

  const categories = list(packet.stop_escalate_conditions).map(
    (entry) => entry?.category
  );
  for (const category of STOP_CATEGORIES) {
    const entries = list(packet.stop_escalate_conditions).filter(
      (entry) => entry?.category === category
    );
    if (entries.length !== 1)
      errors.push(
        diagnostic(
          "PACKET_STOP_CATEGORY",
          "stop_escalate_conditions",
          `Exactly one ${category} stop/escalation condition is required.`
        )
      );
    else if (
      !text(entries[0].condition) ||
      !text(entries[0].required_action) ||
      !text(entries[0].decision_authority)
    )
      errors.push(
        diagnostic(
          "PACKET_STOP_CONCRETE",
          `stop_escalate_conditions.${category}`,
          "Stop condition, required action, and decision authority must be concrete."
        )
      );
  }
  if (categories.some((value) => !STOP_CATEGORIES.includes(value)))
    errors.push(
      diagnostic(
        "PACKET_STOP_UNKNOWN",
        "stop_escalate_conditions",
        "Stop/escalation category is not recognized."
      )
    );

  for (const [groupName, group] of Object.entries(packet.validations ?? {})) {
    if (group?.required && list(group.cases).length === 0)
      errors.push(
        diagnostic(
          "PACKET_REQUIRED_VALIDATION_EMPTY",
          `validations.${groupName}.cases`,
          "A required validation group needs at least one case."
        )
      );
    if (group?.required && group.waiver !== null)
      errors.push(
        diagnostic(
          "PACKET_REQUIRED_VALIDATION_WAIVER",
          `validations.${groupName}.waiver`,
          "A required validation group cannot have a waiver."
        )
      );
    if (!group?.required && !text(group?.waiver))
      errors.push(
        diagnostic(
          "PACKET_OPTIONAL_VALIDATION_WAIVER",
          `validations.${groupName}.waiver`,
          "A non-required validation group needs a concrete waiver."
        )
      );
    list(group?.cases).forEach((entry, index) => {
      if (
        !["command", "procedure"].includes(entry?.execution_kind) ||
        !text(entry?.execution)
      )
        errors.push(
          diagnostic(
            "PACKET_VALIDATION_EXECUTION",
            `validations.${groupName}.cases[${index}]`,
            "Every validation requires an execution kind and exact execution text."
          )
        );
      if (groupName === "automated" && entry?.execution_kind !== "command")
        errors.push(
          diagnostic(
            "PACKET_AUTOMATED_COMMAND",
            `validations.automated.cases[${index}].execution_kind`,
            "Automated validation execution_kind must be command."
          )
        );
    });
  }

  const requirementIds = list(packet?.behavior?.requirements).map(
    (entry) => entry?.id
  );
  const acceptanceIds = criterionIds(packet);
  const validationIds = Object.values(packet.validations ?? {}).flatMap(
    (group) => list(group?.cases).map((entry) => entry?.id)
  );
  const uniqueness = [
    ["PACKET_REQUIREMENT_DUPLICATE", "behavior.requirements", requirementIds],
    ["PACKET_CRITERION_DUPLICATE", "behavior.requirements", acceptanceIds],
    ["PACKET_VALIDATION_DUPLICATE", "validations", validationIds],
    [
      "PACKET_CRITERION_MAP_DUPLICATE",
      "criterion_evidence_map",
      list(packet.criterion_evidence_map).map((entry) => entry?.criterion_id),
    ],
    [
      "PACKET_GATE_DUPLICATE",
      "prerequisites.gates",
      list(packet?.prerequisites?.gates).map((entry) => entry?.id),
    ],
    [
      "PACKET_ARTIFACT_DUPLICATE",
      "prerequisites.required_artifacts",
      list(packet?.prerequisites?.required_artifacts).map(
        (entry) => entry?.path
      ),
    ],
    [
      "PACKET_STOP_DUPLICATE",
      "stop_escalate_conditions",
      list(packet.stop_escalate_conditions).map((entry) => entry?.id),
    ],
    [
      "PACKET_PERMALINK_LABEL_DUPLICATE",
      "upstream_evidence.permalinks",
      list(packet?.upstream_evidence?.permalinks).map((entry) => entry?.label),
    ],
    [
      "PACKET_PERMALINK_URL_DUPLICATE",
      "upstream_evidence.permalinks",
      list(packet?.upstream_evidence?.permalinks).map((entry) => entry?.url),
    ],
  ];
  for (const [code, location, values] of uniqueness)
    if (duplicates(values).length)
      errors.push(
        diagnostic(code, location, "Identifiers must be globally unique.")
      );

  const mappedCriteria = list(packet.criterion_evidence_map).map(
    (entry) => entry?.criterion_id
  );
  if (!sameSet(criterionIds(packet), mappedCriteria))
    errors.push(
      diagnostic(
        "PACKET_CRITERION_MAP",
        "criterion_evidence_map",
        "Criterion map must contain every and only declared acceptance criterion."
      )
    );
  const knownValidationIds = Object.values(packet.validations ?? {}).flatMap(
    (group) => list(group?.cases).map((entry) => entry?.id)
  );
  for (const [index, entry] of list(packet.criterion_evidence_map).entries()) {
    for (const id of list(entry?.validation_ids)) {
      if (!knownValidationIds.includes(id))
        errors.push(
          diagnostic(
            "PACKET_VALIDATION_REFERENCE",
            `criterion_evidence_map[${index}].validation_ids`,
            "Criterion refers to an unknown validation ID."
          )
        );
    }
  }
  for (const id of requiredValidationIds(packet)) {
    if (
      !list(packet.criterion_evidence_map).some((entry) =>
        list(entry?.validation_ids).includes(id)
      )
    )
      errors.push(
        diagnostic(
          "PACKET_REQUIRED_VALIDATION_UNMAPPED",
          "criterion_evidence_map",
          "Every required validation must support a criterion."
        )
      );
  }
  if (list(packet?.architecture?.material_questions_remaining).length)
    errors.push(
      diagnostic(
        "PACKET_ARCHITECTURE_OPEN",
        "architecture.material_questions_remaining",
        "A delegable packet cannot retain a material architecture question."
      )
    );
  return sorted(errors);
}

export function validateReviewReport(report, packet) {
  const errors = [...validateTaskPacket(packet)];
  if (!report || typeof report !== "object" || Array.isArray(report))
    return sorted([
      ...errors,
      diagnostic("REPORT_OBJECT", "$", "Review report must be an object."),
    ]);
  if (report.schema_id !== REVIEW_REPORT_SCHEMA_ID)
    errors.push(
      diagnostic(
        "REPORT_SCHEMA_ID",
        "schema_id",
        "Review report schema_id is not the exact governance schema ID."
      )
    );
  if (report.task_id !== packet?.task?.id)
    errors.push(
      diagnostic(
        "REPORT_TASK_ID",
        "task_id",
        "Review task ID must equal the packet task ID."
      )
    );
  const base = report?.review_context?.base_sha;
  const head = report?.review_context?.head_sha;
  if (base !== packet?.git_context?.base_sha)
    errors.push(
      diagnostic(
        "REPORT_BASE_PACKET",
        "review_context.base_sha",
        "Review base SHA must equal the packet base SHA."
      )
    );
  if (!SHA.test(head ?? ""))
    errors.push(
      diagnostic(
        "REPORT_HEAD_SHA",
        "review_context.head_sha",
        "Review head must be one exact lowercase 40-character SHA."
      )
    );
  if (head === base)
    errors.push(
      diagnostic(
        "REPORT_EMPTY_RANGE",
        "review_context.head_sha",
        "Review head must differ from base SHA."
      )
    );
  if (report?.review_context?.integration_branch !== INTEGRATION_BRANCH)
    errors.push(
      diagnostic(
        "REPORT_INTEGRATION_BRANCH",
        "review_context.integration_branch",
        "Review integration branch must be the migration integration branch."
      )
    );
  if (report?.review_context?.work_branch !== packet?.git_context?.work_branch)
    errors.push(
      diagnostic(
        "REPORT_WORK_BRANCH",
        "review_context.work_branch",
        "Review work branch must equal the packet work branch."
      )
    );
  const prMatch = SYNARA_PR_URL.exec(report?.review_context?.pr_url ?? "");
  if (!prMatch)
    errors.push(
      diagnostic(
        "REPORT_PR_URL",
        "review_context.pr_url",
        "Review PR must be an exact Synara pull-request URL."
      )
    );
  const exactRange = `${base}..${head}`;
  const subjects = [
    ["subject.base_sha", report?.subject?.base_sha, base],
    ["subject.head_sha", report?.subject?.head_sha, head],
    ["final_diff_review.base_sha", report?.final_diff_review?.base_sha, base],
    ["final_diff_review.head_sha", report?.final_diff_review?.head_sha, head],
    ["signature.reviewed_base_sha", report?.signature?.reviewed_base_sha, base],
    ["signature.reviewed_head_sha", report?.signature?.reviewed_head_sha, head],
  ];
  for (const [path, actual, expected] of subjects) {
    if (actual !== expected)
      errors.push(
        diagnostic(
          "REPORT_SUBJECT_BINDING",
          path,
          "Artifact subject does not match the exact review base/head."
        )
      );
  }
  if (report?.final_diff_review?.reviewed_range !== exactRange)
    errors.push(
      diagnostic(
        "REPORT_RANGE_BINDING",
        "final_diff_review.reviewed_range",
        "Reviewed range must be the exact review base..head."
      )
    );

  const writerIdentity = text(packet?.writer?.identity);
  const writerRole = text(packet?.writer?.role);
  const reviewerIdentity = text(report?.reviewer?.identity);
  const reviewerRole = text(report?.reviewer?.role);
  if (
    !reviewerIdentity ||
    reviewerRole !== "independent_reviewer" ||
    canonicalPrincipal(reviewerIdentity) ===
      canonicalPrincipal(writerIdentity) ||
    canonicalPrincipal(reviewerRole) === canonicalPrincipal(writerRole)
  )
    errors.push(
      diagnostic(
        "REPORT_REVIEWER_INDEPENDENCE",
        "reviewer",
        "Reviewer identity and role must both differ from the packet writer."
      )
    );
  if (report?.reviewer?.independent_of_implementation !== true)
    errors.push(
      diagnostic(
        "REPORT_REVIEWER_ATTESTATION",
        "reviewer.independent_of_implementation",
        "Reviewer must attest independence."
      )
    );
  if (
    canonicalPrincipal(report?.signature?.identity) !==
      canonicalPrincipal(reviewerIdentity) ||
    canonicalPrincipal(report?.signature?.role) !==
      canonicalPrincipal(reviewerRole)
  )
    errors.push(
      diagnostic(
        "REPORT_SIGNATURE_IDENTITY",
        "signature",
        "Signature identity and role must agree with the reviewer."
      )
    );
  if (report?.signature?.decision !== report?.verdict)
    errors.push(
      diagnostic(
        "REPORT_SIGNATURE_DECISION",
        "signature.decision",
        "Signature decision must equal report verdict."
      )
    );

  const reviewedAt = dateTimeEpoch(report?.reviewer?.reviewed_at);
  const signedAt = dateTimeEpoch(report?.signature?.signed_at);
  if (reviewedAt === null || signedAt === null || reviewedAt > signedAt)
    errors.push(
      diagnostic(
        "REPORT_TEMPORAL_REVIEW_SIGNATURE",
        "signature.signed_at",
        "Reviewer and signature timestamps must be valid and review must precede signature."
      )
    );

  const allowed = list(packet?.file_scope?.allowed_paths);
  const generated = normalizedPaths(packet?.file_scope?.generated_paths);
  const prohibited = list(packet?.file_scope?.prohibited_paths);
  const outOfScope = list(packet?.file_scope?.out_of_scope_paths);
  if (
    !sameTypedScope(
      report?.scope_audit?.allowed_paths,
      packet?.file_scope?.allowed_paths
    )
  )
    errors.push(
      diagnostic(
        "REPORT_ALLOWED_SCOPE_PARITY",
        "scope_audit.allowed_paths",
        "Reported allowed paths must exactly match the packet."
      )
    );
  list(report?.scope_audit?.actual_changed_paths).forEach((entry, index) => {
    if (!normalizeRepositoryPath(entry))
      errors.push(
        diagnostic(
          "REPORT_SCOPE_PATH",
          `scope_audit.actual_changed_paths[${index}]`,
          "Changed paths must be relative repository paths without traversal."
        )
      );
  });
  const actual = normalizedPaths(report?.scope_audit?.actual_changed_paths);
  const reportedGenerated = normalizedPaths(
    report?.scope_audit?.generated_paths
  );
  if (
    duplicates(report?.scope_audit?.actual_changed_paths).length ||
    duplicates(report?.scope_audit?.generated_paths).length ||
    !sameSet(
      reportedGenerated,
      generated.filter((entry) => actual.includes(entry))
    )
  )
    errors.push(
      diagnostic(
        "REPORT_GENERATED_SCOPE_PARITY",
        "scope_audit.generated_paths",
        "Generated paths must exactly mirror changed packet-declared generated files."
      )
    );
  if (
    reportedGenerated.some(
      (path) =>
        !actual.includes(path) ||
        !generated.includes(path) ||
        !allowed.some((scope) => pathMatchesScope(path, scope))
    )
  )
    errors.push(
      diagnostic(
        "REPORT_GENERATED_SCOPE_PARITY",
        "scope_audit.generated_paths",
        "Every reported generated path must be changed, packet-declared, and inside allowed scope."
      )
    );
  const violations = actual.filter(
    (path) =>
      !allowed.some((scope) => pathMatchesScope(path, scope)) ||
      prohibited.some((scope) => pathMatchesScope(path, scope)) ||
      outOfScope.some((scope) => pathMatchesScope(path, scope))
  );
  if (!sameSet(report?.scope_audit?.prohibited_changed_paths, violations))
    errors.push(
      diagnostic(
        "REPORT_SCOPE_TRUTH",
        "scope_audit.prohibited_changed_paths",
        "Scope violations must truthfully enumerate every changed path outside allowed scope or inside prohibited/out-of-scope paths."
      )
    );
  if (report?.scope_audit?.verdict === "pass" && violations.length !== 0)
    errors.push(
      diagnostic(
        "REPORT_SCOPE_VERDICT",
        "scope_audit.verdict",
        "Scope verdict must agree with the actual path audit."
      )
    );

  const dependency = report?.packet_conformance?.dependency_policy;
  const actualChanges = list(dependency?.actual_changes);
  const expectedDetected = actualChanges.length > 0;
  const allowedDependencyChanges = list(
    packet?.dependency_policy?.allowed_changes
  );
  const prohibitedDependencyChanges = list(
    packet?.dependency_policy?.prohibited_changes
  );
  const dependencyViolation =
    (!packet?.dependency_policy?.changes_allowed && expectedDetected) ||
    actualChanges.some(
      (entry) =>
        !allowedDependencyChanges.includes(entry) ||
        prohibitedDependencyChanges.includes(entry)
    ) ||
    actual.some((entry) => prohibitedDependencyChanges.includes(entry));
  if (
    dependency?.changes_detected !== expectedDetected ||
    (dependency?.verdict === "pass" && dependencyViolation)
  )
    errors.push(
      diagnostic(
        "REPORT_DEPENDENCY_TRUTH",
        "packet_conformance.dependency_policy",
        "Dependency evidence/verdict must agree with the packet and actual changes."
      )
    );
  const prerequisite = report?.packet_conformance?.prerequisites;
  const packetGates = list(packet?.prerequisites?.gates);
  const reviewedGates = list(prerequisite?.gates);
  const packetArtifacts = list(packet?.prerequisites?.required_artifacts);
  const reviewedArtifacts = list(prerequisite?.required_artifacts);
  const packetAssumptions = list(packet?.prerequisites?.blocking_assumptions);
  const reviewedAssumptions = list(prerequisite?.blocking_assumptions);
  const gatesMatch =
    packetGates.length === reviewedGates.length &&
    packetGates.every((expected) =>
      reviewedGates.some(
        (actual) =>
          actual?.id === expected?.id &&
          actual?.required_state === expected?.required_state &&
          actual?.packet_evidence === expected?.evidence
      )
    );
  const artifactsMatch =
    packetArtifacts.length === reviewedArtifacts.length &&
    packetArtifacts.every((expected) =>
      reviewedArtifacts.some(
        (actual) =>
          actual?.path === expected?.path &&
          actual?.purpose === expected?.purpose &&
          actual?.sha256 === expected?.sha256
      )
    );
  const assumptionsMatch =
    packetAssumptions.length === reviewedAssumptions.length &&
    packetAssumptions.every((expected) =>
      reviewedAssumptions.some((actual) => actual?.assumption === expected)
    );
  const allPrerequisitesVerified = [
    ...reviewedGates,
    ...reviewedArtifacts,
    ...reviewedAssumptions,
  ].every((entry) => entry?.verified === true);
  if (!gatesMatch || !artifactsMatch || !assumptionsMatch)
    errors.push(
      diagnostic(
        "REPORT_PREREQUISITE_PARITY",
        "packet_conformance.prerequisites",
        "Prerequisite evidence must exactly mirror packet gates, artifacts, and blocking assumptions."
      )
    );
  if (prerequisite?.verdict === "pass" && !allPrerequisitesVerified)
    errors.push(
      diagnostic(
        "REPORT_PREREQUISITE_VERDICT",
        "packet_conformance.prerequisites.verdict",
        "Passing prerequisite verdict requires every gate, artifact, and assumption to be verified."
      )
    );

  const expectedCriteria = criterionIds(packet);
  if (
    duplicates(expectedCriteria).length ||
    duplicates(
      list(report.requirement_matrix).map((entry) => entry?.criterion_id)
    ).length
  )
    errors.push(
      diagnostic(
        "REPORT_REQUIREMENT_DUPLICATE",
        "requirement_matrix",
        "Requirement criterion IDs must be unique before parity comparison."
      )
    );
  const reportValidationIds = list(report.validation_runs).map(
    (entry) => entry?.id
  );
  if (duplicates(reportValidationIds).length)
    errors.push(
      diagnostic(
        "REPORT_VALIDATION_DUPLICATE",
        "validation_runs",
        "Validation run IDs must be unique before parity comparison."
      )
    );
  if (
    !sameSet(
      list(report.requirement_matrix).map((entry) => entry?.criterion_id),
      expectedCriteria
    )
  )
    errors.push(
      diagnostic(
        "REPORT_REQUIREMENT_PARITY",
        "requirement_matrix",
        "Requirement matrix must contain every and only packet criterion."
      )
    );
  if (
    !sameSet(
      list(report.validation_runs)
        .filter((entry) => entry?.required)
        .map((entry) => entry?.id),
      requiredValidationIds(packet)
    )
  )
    errors.push(
      diagnostic(
        "REPORT_VALIDATION_PARITY",
        "validation_runs",
        "Required validation reruns must exactly match required packet validation IDs."
      )
    );
  const packetValidationCases = new Map();
  for (const group of Object.values(packet.validations ?? {}))
    for (const entry of list(group?.cases))
      packetValidationCases.set(entry?.id, {
        ...entry,
        required: group?.required === true,
      });
  list(report.validation_runs).forEach((entry, index) => {
    if (entry?.base_sha !== base || entry?.head_sha !== head)
      errors.push(
        diagnostic(
          "REPORT_VALIDATION_SUBJECT",
          `validation_runs[${index}]`,
          "Validation run must bind to the exact review base/head."
        )
      );
    if (
      entry?.execution_kind === "command" &&
      entry?.result === "pass" &&
      entry?.exit_code !== 0
    )
      errors.push(
        diagnostic(
          "REPORT_VALIDATION_RESULT",
          `validation_runs[${index}]`,
          "Command validation pass requires exit_code 0."
        )
      );
    if (
      (entry?.execution_kind === "procedure" && entry?.exit_code !== null) ||
      (entry?.execution_kind === "command" &&
        !Number.isInteger(entry?.exit_code))
    )
      errors.push(
        diagnostic(
          "REPORT_VALIDATION_EXIT_KIND",
          `validation_runs[${index}].exit_code`,
          "Command exit_code must be an integer and procedure exit_code must be null."
        )
      );
    const packetCase = packetValidationCases.get(entry?.id);
    if (
      !packetCase ||
      entry?.required !== packetCase?.required ||
      entry?.execution_kind !== packetCase?.execution_kind ||
      entry?.execution !== packetCase?.execution ||
      !sameSet(entry?.environment, packetCase?.environment)
    )
      errors.push(
        diagnostic(
          "REPORT_VALIDATION_EVIDENCE",
          `validation_runs[${index}]`,
          "Validation ID, required state, execution kind/text, and environment must exactly agree with the packet case."
        )
      );
    const started = dateTimeEpoch(entry?.started_at);
    const finished = dateTimeEpoch(entry?.finished_at);
    if (
      started === null ||
      finished === null ||
      reviewedAt === null ||
      started > finished ||
      finished > reviewedAt
    )
      errors.push(
        diagnostic(
          "REPORT_VALIDATION_TIME",
          `validation_runs[${index}]`,
          "Validation start must precede finish and finish must not follow review."
        )
      );
  });
  if (
    !sameSet(
      reportValidationIds.filter(
        (id) => packetValidationCases.get(id)?.required
      ),
      requiredValidationIds(packet)
    )
  )
    errors.push(
      diagnostic(
        "REPORT_VALIDATION_PARITY",
        "validation_runs",
        "Every required packet validation must have exactly one matching report run."
      )
    );
  const ciNames = list(report.ci_checks).map((entry) => entry?.name);
  if (duplicates(ciNames).length)
    errors.push(
      diagnostic(
        "REPORT_CI_DUPLICATE",
        "ci_checks",
        "CI check names must be globally unique."
      )
    );
  const requiredCiNames = list(packet?.git_context?.required_ci_checks);
  const requiredReportedCi = list(report.ci_checks)
    .filter((entry) => entry?.required === true)
    .map((entry) => entry?.name);
  if (
    !sameSet(requiredCiNames, requiredReportedCi) ||
    requiredCiNames.length === 0
  )
    errors.push(
      diagnostic(
        "REPORT_CI_PARITY",
        "ci_checks",
        "Required CI rows must exactly match the packet required CI set."
      )
    );
  list(report.ci_checks).forEach((entry, index) => {
    if (entry?.base_sha !== base || entry?.head_sha !== head)
      errors.push(
        diagnostic(
          "REPORT_CI_SUBJECT",
          `ci_checks[${index}]`,
          "CI check must bind to the exact review base/head."
        )
      );
    if (
      requiredCiNames.includes(entry?.name) &&
      !SYNARA_CI_URL.test(entry?.url ?? "")
    )
      errors.push(
        diagnostic(
          "REPORT_CI_URL",
          `ci_checks[${index}]`,
          "Every packet-required CI check must use an exact Synara Actions run/job URL."
        )
      );
  });

  const packetLinks = list(packet?.upstream_evidence?.permalinks);
  const reportLinks = list(report?.upstream_api_verification?.permalinks);
  if (
    duplicates(reportLinks.map((entry) => entry?.label)).length ||
    duplicates(reportLinks.map((entry) => entry?.url)).length
  )
    errors.push(
      diagnostic(
        "REPORT_UPSTREAM_DUPLICATE",
        "upstream_api_verification.permalinks",
        "Upstream permalink labels and URLs must be unique."
      )
    );
  const linksMatch =
    packetLinks.length === reportLinks.length &&
    packetLinks.every((expected) =>
      reportLinks.some(
        (actualLink) =>
          actualLink?.label === expected?.label &&
          actualLink?.url === expected?.url &&
          actualLink?.claim === expected?.claim &&
          actualLink?.required === expected?.required
      )
    );
  if (
    report?.upstream_api_verification?.repository_url !==
      packet?.upstream_evidence?.repository_url ||
    report?.upstream_api_verification?.release_tag !==
      packet?.upstream_evidence?.release_tag ||
    report?.upstream_api_verification?.commit_sha !==
      packet?.upstream_evidence?.commit_sha ||
    !linksMatch
  )
    errors.push(
      diagnostic(
        "REPORT_UPSTREAM_PARITY",
        "upstream_api_verification",
        "Upstream verification must exactly mirror packet repository, release, commit, permalink labels/URLs, claims, and required flags."
      )
    );
  const requiredLinksVerified = reportLinks
    .filter((entry) => entry?.required)
    .every((entry) => entry?.verified === true);
  if (
    report?.upstream_api_verification?.verdict === "pass" &&
    !requiredLinksVerified
  )
    errors.push(
      diagnostic(
        "REPORT_UPSTREAM_VERDICT",
        "upstream_api_verification.verdict",
        "Passing upstream verdict requires every required permalink to be verified."
      )
    );

  if (duplicates(list(report.findings).map((entry) => entry?.id)).length)
    errors.push(
      diagnostic(
        "REPORT_FINDING_DUPLICATE",
        "findings",
        "Finding IDs must be unique."
      )
    );
  if (duplicates(list(report.residuals).map((entry) => entry?.id)).length)
    errors.push(
      diagnostic(
        "REPORT_RESIDUAL_DUPLICATE",
        "residuals",
        "Residual IDs must be unique."
      )
    );
  list(report.findings).forEach((finding, index) => {
    const acceptance = finding?.risk_acceptance;
    if (finding?.status === "accepted-risk") {
      const durable = text(acceptance?.approval_reference);
      if (!durableRiskReference(durable))
        errors.push(
          diagnostic(
            "REPORT_RISK_REFERENCE",
            `findings[${index}].risk_acceptance.approval_reference`,
            "Accepted risk requires a durable URL or repository path with fragment."
          )
        );
      if (
        !text(acceptance?.bounded_rationale) ||
        !validDate(acceptance?.review_by)
      )
        errors.push(
          diagnostic(
            "REPORT_RISK_BOUNDS",
            `findings[${index}].risk_acceptance`,
            "Accepted risk requires bounded rationale and a review/expiry date."
          )
        );
      if (
        canonicalPrincipal(acceptance?.authority_identity) ===
          canonicalPrincipal(writerIdentity) ||
        canonicalPrincipal(acceptance?.authority_role) ===
          canonicalPrincipal(writerRole)
      )
        errors.push(
          diagnostic(
            "REPORT_RISK_WRITER",
            `findings[${index}].risk_acceptance`,
            "Risk authority must be independent of the implementation writer."
          )
        );
      if (
        ["critical", "high"].includes(finding?.severity) &&
        (!EXTERNAL_RISK_ROLES.has(
          canonicalPrincipal(acceptance?.authority_role)
        ) ||
          canonicalPrincipal(acceptance?.authority_identity) ===
            canonicalPrincipal(reviewerIdentity) ||
          canonicalPrincipal(acceptance?.authority_role) ===
            canonicalPrincipal(reviewerRole))
      )
        errors.push(
          diagnostic(
            "REPORT_HIGH_RISK_AUTHORITY",
            `findings[${index}].risk_acceptance`,
            "Critical/high risk requires external program authority and cannot be reviewer self-acceptance."
          )
        );
      if (
        validDate(acceptance?.review_by) &&
        reviewedAt !== null &&
        signedAt !== null &&
        (acceptance.review_by < utcDate(reviewedAt) ||
          acceptance.review_by < utcDate(signedAt))
      )
        errors.push(
          diagnostic(
            "REPORT_RISK_EXPIRED",
            `findings[${index}].risk_acceptance.review_by`,
            "Risk review date must not precede the UTC date of review or signature."
          )
        );
    } else if (acceptance !== undefined) {
      errors.push(
        diagnostic(
          "REPORT_RISK_EXCLUSIVITY",
          `findings[${index}].risk_acceptance`,
          "Open/resolved findings must not carry accepted-risk payload."
        )
      );
    }
  });

  const signatureMethod = report?.signature?.method;
  const signatureReference = report?.signature?.reference;
  if (
    (signatureMethod === "github-review" &&
      (!SYNARA_REVIEW_URL.test(signatureReference ?? "") ||
        SYNARA_REVIEW_URL.exec(signatureReference)?.[1] !== prMatch?.[1])) ||
    (signatureMethod === "git-signed-commit" &&
      !SHA.test(signatureReference ?? "")) ||
    (signatureMethod === "document-attestation" &&
      !repositoryReference(signatureReference))
  )
    errors.push(
      diagnostic(
        "REPORT_SIGNATURE_REFERENCE",
        "signature.reference",
        "Signature reference must satisfy its method-specific durable form."
      )
    );

  if (report.verdict === "accept") {
    const domains = Object.values(report?.audit_domains ?? {});
    if (
      errors.length ||
      report?.scope_audit?.verdict !== "pass" ||
      report?.packet_conformance?.dependency_policy?.verdict !== "pass" ||
      report?.packet_conformance?.prerequisites?.verdict !== "pass" ||
      report?.upstream_api_verification?.verdict !== "pass" ||
      report?.final_diff_review?.reviewed_complete_final_diff !== true ||
      report?.final_diff_review?.last_correction_reviewed !== true ||
      domains.some(
        (entry) => !["pass", "not-applicable"].includes(entry?.verdict)
      ) ||
      list(report.requirement_matrix).some(
        (entry) => entry?.verdict !== "pass"
      ) ||
      list(report.validation_runs).some(
        (entry) => entry?.required && entry?.result !== "pass"
      ) ||
      list(report.ci_checks).some(
        (entry) =>
          entry?.required && (entry?.status !== "success" || entry?.cancelled)
      ) ||
      list(report.findings).some((entry) => entry?.status === "open")
    )
      errors.push(
        diagnostic(
          "REPORT_ACCEPT_INVALID",
          "verdict",
          "Accept verdict is forbidden while any binding, scope, requirement, validation, CI, finding, or packet-conformance check fails."
        )
      );
  }
  return sorted(errors);
}

function git(repositoryRoot, arguments_, runner = spawnSync) {
  return runner("git", arguments_, {
    cwd: repositoryRoot,
    encoding: null,
    shell: false,
    windowsHide: true,
  });
}

function confinedRegularFile(repositoryRoot, repositoryPath) {
  const normalized = normalizeRepositoryPath(repositoryPath);
  if (!normalized) return false;
  let current = repositoryRoot;
  try {
    for (const segment of normalized.split("/")) {
      current = path.join(current, segment);
      const status = lstatSync(current);
      if (status.isSymbolicLink()) return false;
    }
    const status = lstatSync(current);
    return (
      status.isFile() &&
      realpathSync(current).startsWith(
        `${realpathSync(repositoryRoot)}${path.sep}`
      )
    );
  } catch {
    return false;
  }
}

export function validateProductionReview(
  report,
  packet,
  repositoryRoot,
  runner = spawnSync
) {
  const errors = [];
  const base = report?.review_context?.base_sha;
  const head = report?.review_context?.head_sha;
  for (const [label, sha] of [
    ["base", base],
    ["head", head],
  ]) {
    const result = SHA.test(sha ?? "")
      ? git(repositoryRoot, ["cat-file", "-e", `${sha}^{commit}`], runner)
      : { status: 1 };
    if (result.status !== 0)
      errors.push(
        diagnostic(
          "PRODUCTION_GIT_COMMIT",
          `review_context.${label}_sha`,
          "Review subject must resolve to an exact repository commit."
        )
      );
  }
  if (!errors.some((entry) => entry.code === "PRODUCTION_GIT_COMMIT")) {
    const result = git(
      repositoryRoot,
      ["diff", "--no-renames", "--name-only", "-z", base, head, "--"],
      runner
    );
    if (result.status !== 0) {
      errors.push(
        diagnostic(
          "PRODUCTION_GIT_DIFF",
          "scope_audit",
          "Exact Git diff failed."
        )
      );
    } else {
      const actual = result.stdout.toString("utf8").split("\0").filter(Boolean);
      if (
        actual.some((entry) => !normalizeRepositoryPath(entry)) ||
        duplicates(actual).length ||
        !sameSet(actual, report?.scope_audit?.actual_changed_paths)
      )
        errors.push(
          diagnostic(
            "PRODUCTION_GIT_DIFF_PARITY",
            "scope_audit.actual_changed_paths",
            "Reported changed paths must exactly equal git diff --name-only -z."
          )
        );
    }
  }

  const method = report?.signature?.method;
  const reference = report?.signature?.reference;
  if (method === "git-signed-commit") {
    if (
      git(repositoryRoot, ["verify-commit", reference ?? ""], runner).status !==
      0
    )
      errors.push(
        diagnostic(
          "PRODUCTION_SIGNATURE",
          "signature.reference",
          "Referenced commit must exist and pass git verify-commit."
        )
      );
  } else if (method === "document-attestation") {
    const parsed = repositoryReference(reference);
    if (!parsed || !confinedRegularFile(repositoryRoot, parsed.file))
      errors.push(
        diagnostic(
          "PRODUCTION_SIGNATURE",
          "signature.reference",
          "Document attestation must resolve to a confined regular non-symlink file."
        )
      );
  }
  list(report?.findings).forEach((finding, index) => {
    const referenceValue = finding?.risk_acceptance?.approval_reference;
    const parsed = repositoryReference(referenceValue);
    if (
      finding?.status === "accepted-risk" &&
      parsed &&
      !confinedRegularFile(repositoryRoot, parsed.file)
    )
      errors.push(
        diagnostic(
          "PRODUCTION_RISK_REFERENCE",
          `findings[${index}].risk_acceptance.approval_reference`,
          "Repository risk approval must resolve to a confined regular non-symlink file."
        )
      );
  });
  return sorted(errors);
}

export function formatDiagnostics(diagnostics) {
  return sorted([...diagnostics]).map(
    (entry) => `${entry.code} ${entry.path}: ${entry.message}`
  );
}
