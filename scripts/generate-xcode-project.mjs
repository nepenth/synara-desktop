import { createHash } from "node:crypto";
import {
  lstatSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  renameSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

// XcodeGen's native cache covers its version, resolved spec and source names.
// It does not check the generated files: a Git checkout or deleted scheme can
// otherwise leave stale outputs behind even when the spec cache matches.
function outputFingerprint(projectDirectory) {
  const digest = createHash("sha256");
  const required = [
    "Synara.xcodeproj/project.pbxproj",
    "Synara.xcodeproj/project.xcworkspace/contents.xcworkspacedata",
    "Synara.xcodeproj/xcshareddata/xcschemes/Synara.xcscheme",
    "Synara/App/Info.plist",
    "SynaraNotificationService/Info.plist",
  ];
  try {
    for (const path of required) {
      if (!lstatSync(join(projectDirectory, path)).isFile()) return null;
    }
    function visit(relativePath) {
      const path = join(projectDirectory, relativePath);
      const entry = lstatSync(path);
      // Include names, lengths and bytes, but not timestamps. Native XcodeGen
      // regeneration may recreate an identical project after a source is added.
      if (entry.isDirectory()) {
        for (const name of readdirSync(path).sort()) {
          // Personal state and remote resolution locks are not generated input.
          if (
            name === "xcuserdata" ||
            name === "Package.resolved" ||
            name.endsWith(".xcuserstate")
          )
            continue;
          visit(join(relativePath, name));
        }
      } else if (entry.isFile()) {
        const bytes = readFileSync(path);
        digest.update(`${relativePath}\0file\0${bytes.length}\0`);
        digest.update(bytes);
      } else {
        throw new Error(`Unexpected generated output type: ${path}`);
      }
    }
    visit("Synara.xcodeproj");
    visit("Synara/App/Info.plist");
    visit("SynaraNotificationService/Info.plist");
    return digest.digest("hex");
  } catch (error) {
    if (error.code === "ENOENT") return null;
    throw error;
  }
}

function main() {
  const values = new Map();
  for (let index = 2; index < process.argv.length; index += 2) {
    const argument = process.argv[index];
    if (
      !["--project-dir", "--cache-path"].includes(argument) ||
      !process.argv[index + 1] ||
      values.has(argument)
    ) {
      throw new Error(
        "usage: generate-xcode-project.mjs --project-dir <synara-ios> --cache-path <cache-file>",
      );
    }
    values.set(argument, resolve(process.argv[index + 1]));
  }
  const projectDirectory = values.get("--project-dir");
  const cachePath = values.get("--cache-path");
  if (!projectDirectory || !cachePath)
    throw new Error("Both --project-dir and --cache-path are required");
  const versionResult = spawnSync("xcodegen", ["--version"], {
    encoding: "utf8",
  });
  if (versionResult.error) throw versionResult.error;
  if (versionResult.status !== 0 || !versionResult.stdout.trim())
    throw new Error("Unable to identify the XcodeGen version");
  const generatorVersion = versionResult.stdout.trim();
  const statePath = `${cachePath}.outputs.json`;
  let previousFingerprint;
  try {
    const state = JSON.parse(readFileSync(statePath, "utf8"));
    if (state.version === 1 && state.generatorVersion === generatorVersion)
      previousFingerprint = state.fingerprint;
  } catch (error) {
    if (error.code !== "ENOENT" && !(error instanceof SyntaxError)) throw error;
  }
  const currentFingerprint = outputFingerprint(projectDirectory);
  if (!currentFingerprint || currentFingerprint !== previousFingerprint) {
    // Only invalidate this checkout's native spec cache, never Swift build data.
    rmSync(cachePath, { force: true });
    rmSync(statePath, { force: true });
  }
  mkdirSync(dirname(cachePath), { recursive: true });
  const result = spawnSync(
    "xcodegen",
    [
      "generate",
      "--spec",
      "project.yml",
      "--use-cache",
      "--cache-path",
      cachePath,
    ],
    { cwd: projectDirectory, stdio: "inherit" },
  );
  if (result.error) throw result.error;
  if (result.status !== 0) {
    // A failed generation must not seed a cache hit on its next invocation.
    rmSync(cachePath, { force: true });
    rmSync(statePath, { force: true });
    process.exitCode = result.status ?? 1;
    return;
  }
  const fingerprint = outputFingerprint(projectDirectory);
  if (!fingerprint)
    throw new Error(
      "XcodeGen did not produce the required Synara project, scheme and Info.plists",
    );
  const temporary = `${statePath}.${process.pid}.tmp`;
  try {
    writeFileSync(
      temporary,
      `${JSON.stringify({ version: 1, generatorVersion, fingerprint })}\n`,
      { mode: 0o600 },
    );
    renameSync(temporary, statePath);
  } finally {
    rmSync(temporary, { force: true });
  }
}

try {
  main();
} catch (error) {
  console.error(`generate-xcode-project: ${error.message}`);
  process.exitCode = 1;
}
