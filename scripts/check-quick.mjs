// Fast local pre-push check for what CI's "Rust format and lint" and
// "Validate Node desktop runtime: Check repository contracts" steps catch most
// often. It is scoped to the diff against origin/main (committed, staged,
// unstaged and untracked files), so it runs in a few minutes:
//   - cargo fmt --all --check
//   - cargo clippy -D warnings for only the workspace crates the diff touches
//   - check:matrix-boundaries, check:versions, check:release-updater,
//     check:workflows
//   - prettier, eslint and typecheck when renderer files under synara/ changed
// It does not replace CI: tests, the iOS lanes and the Synapse proofs still run
// there.
//
// Usage: npm run check:quick [-- --base <ref>]
import { execFileSync, spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const WORKSPACE_CRATES = {
  "crates/synara-core": "synara-core",
  "crates/synara-nse-core": "synara-nse-core",
  "crates/synara-core-bindgen": "synara-core-bindgen",
};
// Edits here can change every crate's build.
const WORKSPACE_WIDE = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".cargo/", "clippy.toml"];

const RENDERER_PRETTIER = /\.(?:[cm]?[jt]sx?|json|css|scss|html|md|ya?ml)$/;
const RENDERER_ESLINT = /\.(?:[cm]?[jt]sx?)$/;
const RENDERER_TYPES = /\.(?:tsx?|d\.ts)$|(?:^|\/)tsconfig[^/]*\.json$/;

// The clippy and renderer commands for one changed-file list, mirroring the
// matching CI steps. Exported for scripts/__tests__/check-quick.test.mjs.
export const planQuickChecks = (changedFiles) => {
  const files = [...new Set(changedFiles.filter(Boolean))];
  const workspaceWide = files.some((file) =>
    WORKSPACE_WIDE.some((prefix) => (prefix.endsWith("/") ? file.startsWith(prefix) : file === prefix))
  );
  const packages = Object.entries(WORKSPACE_CRATES)
    .filter(([dir]) => workspaceWide || files.some((file) => file.startsWith(`${dir}/`)))
    .map(([, name]) => name);
  const desktop = workspaceWide || files.some((file) => file.startsWith("src-tauri/"));

  const clippy = [];
  if (packages.length > 0) {
    clippy.push({
      label: `clippy ${packages.join(", ")}`,
      args: ["clippy", "--locked", ...packages.flatMap((name) => ["-p", name]), "--all-targets", "--", "-D", "warnings"],
      cwd: ".",
    });
  }
  if (packages.includes("synara-core")) {
    clippy.push({
      label: "clippy synara-core desktop-only features",
      args: ["clippy", "--locked", "-p", "synara-core", "--features", "search-index,x509-identity", "--all-targets", "--", "-D", "warnings"],
      cwd: ".",
    });
  }
  if (desktop) {
    clippy.push({
      label: "clippy desktop shell",
      args: ["clippy", "--locked", "--all-targets", "--", "-D", "warnings"],
      cwd: "src-tauri",
    });
  }

  const renderer = files
    .filter((file) => file.startsWith("synara/") && !file.startsWith("synara/node_modules/"))
    .map((file) => file.slice("synara/".length));
  const prettier = renderer.filter((file) => RENDERER_PRETTIER.test(file));
  // The same globs as synara's check:eslint script.
  const eslint = renderer.filter(
    (file) =>
      RENDERER_ESLINT.test(file) &&
      (file.startsWith("src/") || file.startsWith("e2e/") || file === "playwright.config.ts")
  );
  const typecheck = renderer.some((file) => RENDERER_TYPES.test(file));
  return { clippy, prettier, eslint, typecheck };
};

const git = (...args) => execFileSync("git", args, { cwd: root, encoding: "utf8" }).trim();

const changedFiles = (base) => {
  const mergeBase = git("merge-base", "HEAD", base);
  const lines = [
    git("diff", "--name-only", mergeBase),
    git("ls-files", "--others", "--exclude-standard"),
  ].join("\n");
  return lines.split("\n").filter(Boolean);
};

const hasCommand = (command) =>
  spawnSync(process.platform === "win32" ? "where" : "which", [command], { stdio: "ignore" }).status === 0;

const main = () => {
  const baseIndex = process.argv.indexOf("--base");
  const base = baseIndex >= 0 ? process.argv[baseIndex + 1] : "origin/main";
  let files;
  try {
    files = changedFiles(base);
  } catch {
    console.error(`[check:quick] cannot diff against ${base}; run \`git fetch origin\` or pass --base <ref>.`);
    process.exit(2);
  }
  const existing = files.filter((file) => existsSync(path.join(root, file)));
  const plan = planQuickChecks(files);
  console.log(`[check:quick] ${files.length} file(s) differ from ${base}.`);

  const results = [];
  const run = (label, command, args, cwd = ".") => {
    console.log(`\n[check:quick] ${label}: ${command} ${args.join(" ")}`);
    const started = Date.now();
    const { status } = spawnSync(command, args, { cwd: path.join(root, cwd), stdio: "inherit" });
    results.push({ label, ok: status === 0, seconds: Math.round((Date.now() - started) / 1000) });
  };
  const skip = (label, reason) => results.push({ label, ok: true, skipped: reason });

  run("cargo fmt", "cargo", ["fmt", "--all", "--", "--check"]);
  if (plan.clippy.length === 0) skip("clippy", "no workspace crate changed");
  for (const step of plan.clippy) {
    // The repository's compilation cache wrapper, as `npm run cargo` uses.
    run(step.label, path.join(root, "scripts/with-rust-cache.sh"), ["cargo", ...step.args], step.cwd);
  }

  run("check:matrix-boundaries", "node", ["scripts/check-matrix-boundaries.mjs"]);
  run("check:versions", "node", ["scripts/check-version-consistency.mjs"]);
  run("check:release-updater", "node", ["scripts/check-release-updater.mjs"]);
  if (hasCommand("actionlint")) {
    run("check:workflows", "npm", ["run", "--silent", "check:workflows"]);
  } else {
    // CI installs a pinned actionlint; run the rest of check:workflows here.
    run("check:workflows (no actionlint installed)", "node", ["scripts/check-workflows.mjs"]);
    run("check:docs", "node", ["scripts/check-documentation-hygiene.mjs"]);
  }

  const existingRenderer = new Set(existing.filter((file) => file.startsWith("synara/")).map((file) => file.slice(7)));
  const prettier = plan.prettier.filter((file) => existingRenderer.has(file));
  const eslint = plan.eslint.filter((file) => existingRenderer.has(file));
  const rendererInstalled = existsSync(path.join(root, "synara/node_modules/.bin"));
  if (prettier.length + eslint.length > 0 || plan.typecheck) {
    if (!rendererInstalled) {
      results.push({ label: "renderer checks", ok: false, skipped: "run `npm ci` in synara/ first" });
    } else {
      if (prettier.length > 0) run("prettier", "npx", ["--no-install", "prettier", "--check", "--ignore-unknown", ...prettier], "synara");
      if (eslint.length > 0) run("eslint", "npx", ["--no-install", "eslint", ...eslint], "synara");
      if (plan.typecheck) run("typecheck", "npm", ["run", "--silent", "typecheck"], "synara");
    }
  } else {
    skip("renderer checks", "no renderer file changed");
  }

  console.log("\n[check:quick] summary");
  for (const result of results) {
    const status = result.skipped ? (result.ok ? "skip" : "FAIL") : result.ok ? "ok  " : "FAIL";
    const detail = result.skipped ? ` (${result.skipped})` : ` (${result.seconds}s)`;
    console.log(`  ${status} ${result.label}${detail}`);
  }
  if (results.some((result) => !result.ok)) process.exit(1);
};

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
