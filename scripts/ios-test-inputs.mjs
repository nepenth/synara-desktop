// Content hash of everything the CI "iOS simulator tests" job reads, so a
// later run of the same pull request can skip the job when an earlier run
// already passed it on identical inputs (scripts/reuse-proven-ios-tests.mjs).
//
// The hash covers committed content only: `git ls-tree` blob ids for the input
// paths, the text of the ios-tests job in .github/workflows/ci.yml, and the
// "Install Rust 1.96 and Apple targets" steps of ci.yml and release.yml, which
// scripts/check-synara-nse-core-isolation.mjs reads. Add a
// path here whenever the job, synara-ios/scripts/ci-build.sh or a script they
// call starts reading a new file; scripts/__tests__/ios-test-inputs.test.mjs
// fails when a referenced script is missing from this list.
//
// Usage:
//   node scripts/ios-test-inputs.mjs paths          one input path per line
//   node scripts/ios-test-inputs.mjs hash           hash of HEAD
//   node scripts/ios-test-inputs.mjs verify <hash>  exit 1 unless HEAD matches
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { pathToFileURL } from "node:url";

export const IOS_TESTS_JOB_ID = "ios-tests";
export const IOS_TESTS_JOB_NAME = "iOS simulator tests";
export const IOS_PROOF_STEP_PREFIX = "Record proven iOS inputs";
const HASH_VERSION = "synara-ios-tests-inputs-v1";

export const IOS_TEST_INPUT_PATHS = [
  // Swift app, packages, project spec and its build scripts.
  "synara-ios",
  // The Rust Core and NSE Core built into the XCFrameworks, and the workspace
  // that resolves them (src-tauri is a member, so its manifest is read too).
  "crates",
  "Cargo.toml",
  "Cargo.lock",
  "rust-toolchain.toml",
  ".cargo",
  "src-tauri/Cargo.toml",
  // Composite actions the job uses.
  ".github/actions/setup-rust-cache",
  ".github/actions/setup-xcode-cache",
  ".github/actions/save-xcode-cache",
  // Rust compilation cache wrappers.
  ".kache.toml",
  "scripts/ci-rust-cache-identity.sh",
  "scripts/start-ci-rust-cache.mjs",
  "scripts/setup-rust-cache.sh",
  "scripts/with-rust-cache.sh",
  "scripts/lib/rust-cache.sh",
  // Generators and checkers run by the job and by ci-build.sh.
  "scripts/check-synara-core-swift-scaffold.mjs",
  "scripts/check-synara-nse-core-isolation.mjs",
  "scripts/check-synara-nse-core-production-features.mjs",
  "scripts/check-synara-core-production-features.mjs",
  "scripts/check-synara-nse-core-archive-exports.sh",
  "scripts/check-xcode-local-package-graph.mjs",
  "scripts/generate-synara-core-swift.sh",
  "scripts/generate-synara-nse-core-swift.sh",
  "scripts/generate-xcode-project.mjs",
  "scripts/lib/compare-generated-apple-pair.mjs",
  "scripts/lib/publish-generated-apple-pair.sh",
  "scripts/lib/rust-llvm-symbols.sh",
  "scripts/lib/ffi-surface.mjs",
  "scripts/lib/rust-module-sources.mjs",
  "scripts/lib/typed-recovery-boundaries.mjs",
  "scripts/lib/runtime-config.mjs",
  // The scaffold checker requires these test files to exist.
  "scripts/__tests__/xcode-local-package-graph.test.mjs",
  "scripts/__tests__/apple-pair-publication.test.mjs",
  // The definition of this hash.
  "scripts/ios-test-inputs.mjs",
];

export const IOS_INPUTS_HASH_PATTERN = /^[0-9a-f]{64}$/;

// The job's YAML block: its `  ios-tests:` line through the line before the
// next job. Returns null when the job is missing or appears twice.
export const extractWorkflowJob = (source, jobId) => {
  const lines = String(source).split(/\r?\n/);
  const header = new RegExp(`^ {2}${jobId}:\\s*$`);
  const starts = lines.flatMap((line, index) => (header.test(line) ? [index] : []));
  if (starts.length !== 1) return null;
  const block = [lines[starts[0]]];
  for (let index = starts[0] + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (/^ {0,2}\S/.test(line)) break;
    block.push(line);
  }
  while (block.length > 1 && block.at(-1).trim() === "") block.pop();
  return block.join("\n");
};

// The same step match as scripts/check-synara-nse-core-isolation.mjs.
export const appleToolchainSteps = (source) =>
  [
    ...String(source).matchAll(
      / {6}- name: Install Rust 1\.96 and Apple targets\n([\s\S]*?)(?=\n {6}- name:|\n {2}\S|$)/g
    ),
  ].map((match) => match[0]);

export const iosTestInputsHash = ({ treeListing, jobBlock, toolchainSteps = [] }) => {
  if (typeof treeListing !== "string" || treeListing.trim() === "") {
    throw new Error("iOS test inputs: empty tree listing");
  }
  if (typeof jobBlock !== "string" || jobBlock.trim() === "") {
    throw new Error(`iOS test inputs: missing ${IOS_TESTS_JOB_ID} job block`);
  }
  return createHash("sha256")
    .update(
      [
        HASH_VERSION,
        treeListing.trim(),
        `--- ${IOS_TESTS_JOB_ID} ---`,
        jobBlock,
        "--- Apple toolchain steps ---",
        ...toolchainSteps,
        "",
      ].join("\n")
    )
    .digest("hex");
};

export const proofStepName = (hash) => `${IOS_PROOF_STEP_PREFIX} ${hash}`;

const git = (args, cwd) =>
  execFileSync("git", args, { cwd, encoding: "utf8", maxBuffer: 256 * 1024 * 1024 });

export const hashCommit = (rev = "HEAD", cwd = process.cwd()) => {
  const treeListing = git(
    ["ls-tree", "-r", "--full-tree", rev, "--", ...IOS_TEST_INPUT_PATHS],
    cwd
  );
  const workflow = git(["show", `${rev}:.github/workflows/ci.yml`], cwd);
  const release = git(["show", `${rev}:.github/workflows/release.yml`], cwd);
  return iosTestInputsHash({
    treeListing,
    jobBlock: extractWorkflowJob(workflow, IOS_TESTS_JOB_ID),
    toolchainSteps: [...appleToolchainSteps(workflow), ...appleToolchainSteps(release)],
  });
};

const main = () => {
  const [command, expected] = process.argv.slice(2);
  if (command === "paths") {
    process.stdout.write(`${IOS_TEST_INPUT_PATHS.join("\n")}\n`);
  } else if (command === "hash") {
    process.stdout.write(`${hashCommit()}\n`);
  } else if (command === "verify") {
    const actual = hashCommit();
    if (!IOS_INPUTS_HASH_PATTERN.test(expected || "") || actual !== expected) {
      console.error(`iOS test inputs hash ${actual} does not match ${expected || "(none)"}.`);
      process.exit(1);
    }
    console.log(`iOS test inputs hash ${actual} proven by this job.`);
  } else {
    console.error("usage: ios-test-inputs.mjs paths | hash | verify <hash>");
    process.exit(2);
  }
};

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
