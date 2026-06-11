#!/usr/bin/env node
/**
 * Validates docs/mip1-commit-evidence.md:
 * - All MIP1-01..46 items are mapped
 * - Cited commit SHAs exist and are reachable from HEAD
 * - Evidence files appear in each cited commit's diff
 *
 * Usage:
 *   node scripts/check-mip1-commit-evidence.mjs [--base <ref>] [--doc <path>]
 */

import { readFileSync, existsSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { resolve, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));

function parseArgs(argv) {
  const options = {
    base: "main",
    doc: join(root, "docs/mip1-commit-evidence.md"),
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--base") {
      options.base = argv[index + 1];
      index += 1;
    } else if (arg === "--doc") {
      options.doc = resolve(argv[index + 1]);
      index += 1;
    } else if (arg === "--help" || arg === "-h") {
      console.log(`Usage: node scripts/check-mip1-commit-evidence.mjs [--base <ref>] [--doc <path>]`);
      process.exit(0);
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }

  return options;
}

function git(args, options = {}) {
  return execFileSync("git", args, {
    cwd: options.cwd ?? root,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}

function gitLines(args) {
  const output = git(args);
  return output ? output.split("\n") : [];
}

function resolveCommit(abbrev) {
  try {
    return git(["rev-parse", "--verify", `${abbrev}^{commit}`]);
  } catch {
    return null;
  }
}

function isAncestor(commit, ancestor) {
  try {
    git(["merge-base", "--is-ancestor", ancestor, commit]);
    return true;
  } catch {
    return false;
  }
}

function commitTouchesFile(commit, filePath) {
  const files = gitLines(["show", "--name-only", "--format=", commit]);
  return files.includes(filePath);
}

function parseEvidenceBlocks(markdown) {
  const pattern = /<!--\s*mip1-evidence:(MIP1-\d{2})\s*([\s\S]*?)-->/g;
  const entries = new Map();
  let match;

  while ((match = pattern.exec(markdown)) !== null) {
    const id = match[1];
    const body = match[2];
    const field = (name) => {
      const line = body
        .split("\n")
        .map((value) => value.trim())
        .find((value) => value.startsWith(`${name}:`));
      return line ? line.slice(name.length + 1).trim() : "";
    };

    const commits = field("commits")
      .split(/[\s,]+/)
      .map((value) => value.trim())
      .filter(Boolean);

    const evidence = [];
    const evidenceSection = body.split(/^evidence:\s*$/m)[1] ?? "";
    for (const line of evidenceSection.split("\n")) {
      const fileMatch = line.match(/^\s*-\s*file:\s*(.+)$/);
      if (fileMatch) {
        evidence.push({ file: fileMatch[1].trim(), note: "" });
        continue;
      }
      const noteMatch = line.match(/^\s*note:\s*(.+)$/);
      if (noteMatch && evidence.length > 0) {
        evidence[evidence.length - 1].note = noteMatch[1].trim();
      }
    }

    entries.set(id, {
      id,
      title: field("title"),
      kind: field("kind") || "unknown",
      commits,
      evidence,
    });
  }

  return entries;
}

function expectedIds() {
  return Array.from({ length: 46 }, (_, index) => `MIP1-${String(index + 1).padStart(2, "0")}`);
}

function fail(messages) {
  for (const message of messages) {
    console.error(`[mip1-commit-evidence] ${message}`);
  }
  process.exitCode = 1;
}

function main() {
  const options = parseArgs(process.argv.slice(2));

  if (!existsSync(options.doc)) {
    fail([`Evidence document not found: ${options.doc}`]);
    process.exit(process.exitCode ?? 1);
  }

  const markdown = readFileSync(options.doc, "utf8");
  const entries = parseEvidenceBlocks(markdown);
  const errors = [];
  const warnings = [];

  const head = git(["rev-parse", "HEAD"]);
  let baseRef = options.base;
  const baseCandidates = [baseRef, `origin/${baseRef}`];
  let resolvedBaseRef = null;
  for (const candidate of baseCandidates) {
    try {
      git(["rev-parse", "--verify", `${candidate}^{commit}`]);
      resolvedBaseRef = candidate;
      break;
    } catch {
      // try next candidate
    }
  }
  if (!resolvedBaseRef) {
    errors.push(
      `Base ref "${baseRef}" is not a valid commit (also tried origin/${baseRef}).`
    );
    baseRef = null;
  } else {
    baseRef = resolvedBaseRef;
  }

  let branchCommitCount = null;
  if (baseRef) {
    branchCommitCount = Number.parseInt(git(["rev-list", "--count", `${baseRef}..HEAD`]), 10);
  }

  for (const id of expectedIds()) {
    if (!entries.has(id)) {
      errors.push(`Missing evidence block for ${id}.`);
    }
  }

  for (const [id, entry] of entries) {
    if (!/^MIP1-\d{2}$/.test(id)) {
      warnings.push(`Unexpected evidence id format: ${id}`);
    }

    if (entry.commits.length === 0) {
      errors.push(`${id}: no commits listed.`);
      continue;
    }

    if (entry.evidence.length === 0) {
      errors.push(`${id}: no evidence files listed.`);
    }

    for (const abbrev of entry.commits) {
      const full = resolveCommit(abbrev);
      if (!full) {
        errors.push(`${id}: commit "${abbrev}" not found in repository.`);
        continue;
      }

      if (!isAncestor(head, full)) {
        errors.push(`${id}: commit ${abbrev} (${full.slice(0, 7)}) is not reachable from HEAD.`);
      }

      if (baseRef && !isAncestor(full, baseRef) && !isAncestor(head, full)) {
        errors.push(`${id}: commit ${abbrev} is neither on branch nor ancestral as expected.`);
      }

    }

    for (const item of entry.evidence) {
      const touchedBy = entry.commits.filter((abbrev) => {
        const full = resolveCommit(abbrev);
        return full && commitTouchesFile(full, item.file);
      });

      if (touchedBy.length === 0) {
        errors.push(
          `${id}: evidence file "${item.file}" not touched by any cited commit (${entry.commits.join(", ")}).`,
        );
      }
    }
  }

  const dedicated = [...entries.values()].filter((entry) => entry.kind === "dedicated").length;
  const bundled = [...entries.values()].filter((entry) => entry.kind === "bundled").length;
  const uniqueCommits = new Set([...entries.values()].flatMap((entry) => entry.commits)).size;

  if (errors.length > 0) {
    fail(errors);
    process.exit(process.exitCode ?? 1);
  }

  console.log("[mip1-commit-evidence] OK");
  console.log(`  Items mapped: ${entries.size}/46`);
  console.log(`  Dedicated: ${dedicated}; bundled: ${bundled}`);
  console.log(`  Unique implementation commits cited: ${uniqueCommits}`);
  if (branchCommitCount !== null) {
    console.log(`  Branch commits (${options.base}..HEAD): ${branchCommitCount} (plan target: 47 incl. mip1-00)`);
    if (branchCommitCount !== 47) {
      warnings.push(
        `Commit count ${branchCommitCount} differs from plan target 47; evidence map compensates for bundled history.`,
      );
    }
  }

  for (const warning of warnings) {
    console.warn(`[mip1-commit-evidence] warning: ${warning}`);
  }
}

main();