import { execFileSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const rustFamilies = [
  "validate-rust-desktop",
  "ci-synara-core-apple-simulator-arm64",
  "release-linux-deb",
  "release-macos",
  "release-synara-core-apple-device",
];

export function summarizeCaches({
  caches,
  usage,
  storageLimit,
  retentionLimit,
}) {
  const scopes = new Map();
  for (const cache of caches) {
    const scope = cache.ref ?? "unknown";
    const summary = scopes.get(scope) ?? { ref: scope, count: 0, bytes: 0 };
    summary.count += 1;
    summary.bytes += cache.size_in_bytes;
    scopes.set(scope, summary);
  }
  const mainCaches = caches.filter((cache) => cache.ref === "refs/heads/main");
  // Select the longest family prefix so one family name that prefixes
  // another cannot satisfy the other's seed requirement.
  const longestFirst = [...rustFamilies].sort((a, b) => b.length - a.length);
  const mainRustFamilies = new Set(
    mainCaches.map((cache) =>
      longestFirst.find((family) =>
        cache.key.startsWith(`synara-kache-v1-${family}-`),
      ),
    ),
  );
  const missingMainFamilies = rustFamilies.filter(
    (family) => !mainRustFamilies.has(family),
  );
  if (!mainCaches.some((cache) => cache.key.startsWith("synara-cargo-v1-")))
    missingMainFamilies.push("cargo-downloads");
  if (
    !mainCaches.some((cache) => cache.key.startsWith("xcode-compilation-v1-"))
  )
    missingMainFamilies.push("xcode-compilation");
  const legacyRustCaches = caches.filter((cache) =>
    cache.key.startsWith("v0-rust-"),
  );
  // GitHub Actions names this setting GB but measures binary gigabytes (GiB).
  const limitBytes = storageLimit.max_cache_size_gb * 2 ** 30;
  return {
    activeBytes: usage.active_caches_size_in_bytes,
    activeCount: usage.active_caches_count,
    limitBytes,
    retentionDays: retentionLimit.max_cache_retention_days,
    utilization: usage.active_caches_size_in_bytes / limitBytes,
    missingMainFamilies,
    legacyRustCount: legacyRustCaches.length,
    legacyRustBytes: legacyRustCaches.reduce(
      (total, cache) => total + cache.size_in_bytes,
      0,
    ),
    scopes: [...scopes.values()].sort((a, b) => b.bytes - a.bytes),
    caches: [...caches].sort((a, b) => b.size_in_bytes - a.size_in_bytes),
  };
}

function audit() {
  const json = (args) =>
    JSON.parse(execFileSync("gh", args, { cwd: root, encoding: "utf8" }));
  const { nameWithOwner } = json(["repo", "view", "--json", "nameWithOwner"]);
  const base = `repos/${nameWithOwner}/actions`;
  const pages = json([
    "api",
    "--paginate",
    "--slurp",
    `${base}/caches?per_page=100`,
  ]);
  const report = summarizeCaches({
    caches: pages.flatMap((page) => page.actions_caches),
    usage: json(["api", `${base}/cache/usage`]),
    storageLimit: json(["api", `${base}/cache/storage-limit`]),
    retentionLimit: json(["api", `${base}/cache/retention-limit`]),
  });
  if (process.argv.includes("--json")) {
    console.log(
      JSON.stringify({ repository: nameWithOwner, ...report }, null, 2),
    );
    return;
  }
  const gib = (bytes) => `${(bytes / 2 ** 30).toFixed(2)} GiB`;
  console.log(
    `${nameWithOwner}: ${gib(report.activeBytes)} / ${gib(report.limitBytes)} (${Math.round(report.utilization * 100)}%), ${report.activeCount} caches, ${report.retentionDays}-day retention.`,
  );
  console.log("Scope                         Caches    Size");
  for (const scope of report.scopes) {
    console.log(
      `${scope.ref.padEnd(30)} ${String(scope.count).padStart(5)}    ${gib(scope.bytes)}`,
    );
  }
  if (report.missingMainFamilies.length) {
    console.log(
      `Missing reusable main seeds: ${report.missingMainFamilies.join(", ")}.`,
    );
  }
  if (report.legacyRustCount) {
    console.log(
      `Legacy Cargo target snapshots: ${report.legacyRustCount} caches, ${gib(report.legacyRustBytes)}. The Kache workflows do not reuse them.`,
    );
  }
  if (report.utilization >= 0.8) {
    console.log(
      "Storage is above 80%: inspect generations and PR/tag scopes and obsolete Cargo target snapshots before increasing compiler cache budgets.",
    );
  }
  console.log(
    "Read-only audit; no caches or repository settings were changed. API snapshots can differ while jobs run.",
  );
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href
) {
  audit();
}
