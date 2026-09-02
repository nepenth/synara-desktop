#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { readFileSync, realpathSync, statSync } from "node:fs";
import { dirname, resolve } from "node:path";

const [projectPathArgument, packageBaseArgument] = process.argv.slice(2);
if (!projectPathArgument || !packageBaseArgument) {
  throw new Error(
    "usage: check-xcode-local-package-graph.mjs <project.pbxproj> <package-base-directory>",
  );
}

const projectPath = resolve(projectPathArgument);
const packageBase = resolve(packageBaseArgument);
const project = readFileSync(projectPath, "utf8");
if (project.includes("XCRemoteSwiftPackageReference")) {
  console.log("remote");
  process.exit(0);
}

const localReferencePattern =
  /isa = XCLocalSwiftPackageReference;\s*relativePath = "?([^";]+)"?;/g;
const pending = [];
for (const match of project.matchAll(localReferencePattern)) {
  pending.push(resolve(packageBase, match[1]));
}
if (pending.length === 0) {
  throw new Error(`no local Swift package references found in ${projectPath}`);
}

const visited = new Set();
let hasRemoteDependency = false;
while (pending.length > 0) {
  const candidate = pending.pop();
  const packagePath = realpathSync(candidate);
  if (visited.has(packagePath)) continue;
  visited.add(packagePath);
  if (!statSync(packagePath).isDirectory()) {
    throw new Error(`Swift package reference is not a directory: ${packagePath}`);
  }

  const manifest = JSON.parse(
    execFileSync(
      "swift",
      ["package", "dump-package", "--package-path", packagePath],
      { encoding: "utf8" },
    ),
  );
  for (const dependency of manifest.dependencies ?? []) {
    const variants = ["sourceControl", "registry", "fileSystem"].filter((variant) =>
      Object.hasOwn(dependency, variant),
    );
    if (variants.length !== 1) {
      throw new Error(`unrecognized Swift dependency variant in ${packagePath}`);
    }
    const variant = variants[0];
    if (variant === "sourceControl" || variant === "registry") {
      const payload = dependency[variant];
      if (!Array.isArray(payload) || payload.length === 0) {
        throw new Error(`malformed ${variant} Swift dependency in ${packagePath}`);
      }
      hasRemoteDependency = true;
      break;
    }
    const localDependencies = dependency.fileSystem;
    if (!Array.isArray(localDependencies) || localDependencies.length === 0) {
      throw new Error(`malformed fileSystem Swift dependency in ${packagePath}`);
    }
    for (const local of localDependencies) {
      const localPath = typeof local === "string" ? local : local.path;
      if (typeof localPath !== "string" || localPath.trim().length === 0) {
        throw new Error(`unrecognized local Swift dependency in ${packagePath}`);
      }
      pending.push(resolve(packagePath, localPath));
    }
  }
  if (hasRemoteDependency) break;
}

console.log(hasRemoteDependency ? "remote" : "local");
