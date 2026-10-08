#!/usr/bin/env node
// Print the public Swift API surface of a UniFFI-generated binding.
//
// The surface keeps what Swift callers compile against: public types, record
// fields in order, initializer labels, enum cases, protocol and method
// signatures. It drops FFI converters, checksums, comments and bodies, and it
// sorts top-level blocks (and members of classes, protocols and extensions)
// because proc-macro metadata order differs from UDL order. Struct fields and
// enum cases keep their order: Swift memberwise initializers depend on it.
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

const INTERNAL_BLOCK = /\b(FfiConverter\w*|Uniffi\w*|uniffi\w*|RustBuffer\w*|ForeignBytes|RustCallStatus\w*)\b/;
const INTERNAL_MEMBER = /\b(uniffiCloneHandle|NoHandle|FfiConverter\w*|uniffi\w*)\b|init\(noHandle:|init\(unsafeFromHandle:/;
const ORDERED_KINDS = new Set(["struct", "enum"]);

function stripStrings(line) {
  return line.replace(/"(?:\\.|[^"\\])*"/g, '""');
}

function braceDelta(line) {
  const code = stripStrings(line).replace(/\/\/.*$/, "");
  let delta = 0;
  for (const char of code) {
    if (char === "{") delta += 1;
    else if (char === "}") delta -= 1;
  }
  return delta;
}

function signature(line) {
  return line
    .replace(/\{.*$/, "")
    .replace(/\s+/g, " ")
    .replace(/\s*\(\s*/g, "(")
    .replace(/\s*\)\s*/g, ") ")
    .replace(/\s*->\s*/g, " -> ")
    .replace(/\)(async|throws)/g, ") $1")
    .replace(/\s+,/g, ",")
    .trim();
}

function blockKind(header) {
  const match = header.match(/\b(struct|enum|class|protocol|extension|func|var|let|typealias)\b/);
  return match ? match[1] : "other";
}

// Join declarations the generator splits across lines: a bare `public`
// modifier line, and parameter lists whose parentheses close on a later line.
function logicalLines(source) {
  const out = [];
  let pending = "";
  let parens = 0;
  for (const raw of source.split("\n")) {
    const line = raw.trim();
    if (!pending && (line === "public" || line === "open")) {
      pending = line;
      continue;
    }
    pending = pending ? `${pending} ${line}` : line;
    const code = stripStrings(line).replace(/\/\/.*$/, "");
    for (const char of code) {
      if (char === "(") parens += 1;
      else if (char === ")") parens -= 1;
    }
    if (parens > 0) continue;
    parens = 0;
    out.push(pending);
    pending = "";
  }
  if (pending) out.push(pending);
  return out;
}

// Doc comments carry Rust `///` text, which may contain braces or parentheses.
// Drop block comments before any structural scan.
function stripBlockComments(source) {
  return source.replace(/\/\*[\s\S]*?\*\//g, (comment) => comment.replace(/[^\n]/g, " "));
}

export function swiftApiSurface(source) {
  const lines = logicalLines(stripBlockComments(source));
  const blocks = [];
  let depth = 0;
  let current = null;
  for (const raw of lines) {
    const line = raw.trim();
    const isPublicTop =
      depth === 0 && (/^(public|open)\b/.test(line) || /^extension\b/.test(line));
    if (isPublicTop) {
      const header = signature(line);
      const internal = INTERNAL_BLOCK.test(header) && !/^extension\b/.test(line)
        ? true
        : /^extension\s+(FfiConverter|Uniffi|RustBuffer|ForeignBytes)/.test(line);
      current = { header, kind: blockKind(header), members: [], internal };
      blocks.push(current);
    } else if (current && depth === 1 && !current.internal) {
      const isMember =
        (/^(public|open)\b/.test(line) && !/\b(private|fileprivate|internal)\b/.test(line)) ||
        (current.kind === "enum" && /^case\s+[A-Za-z_`]/.test(line)) ||
        (current.kind === "protocol" && /^(func|var|init|associatedtype)\b/.test(line));
      if (isMember && !INTERNAL_MEMBER.test(line)) {
        current.members.push(signature(line));
      }
    }
    depth += braceDelta(line);
    if (depth === 0) current = null;
  }

  const rendered = blocks
    .filter((block) => !block.internal)
    .map((block) => {
      const members = ORDERED_KINDS.has(block.kind)
        ? block.members
        : [...new Set(block.members)].sort();
      return [block.header, ...members.map((member) => `  ${member}`)].join("\n");
    });
  return [...new Set(rendered)].sort().join("\n") + "\n";
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  const file = process.argv[2];
  if (!file) {
    console.error("usage: swift-api-surface.mjs <generated.swift>");
    process.exit(2);
  }
  process.stdout.write(swiftApiSurface(readFileSync(file, "utf8")));
}
