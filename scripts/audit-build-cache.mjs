import { execFileSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const reusableFamilies = [
  "validate-rust-desktop",
  "ci-synara-core-apple-simulator-arm64",
  "release-linux-deb",
  "release-macos",
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
  const missingMainFamilies = reusableFamilies.filter(
    (family) =>
      !mainCaches.some((cache) => cache.key.startsWith(`v0-rust-${family}-`)),
  );
  const limitBytes = storageLimit.max_cache_size_gb * 1_000_000_000;
  return {
    activeBytes: usage.active_caches_size_in_bytes,
    activeCount: usage.active_caches_count,
    limitBytes,
    retentionDays: retentionLimit.max_cache_retention_days,
    utilization: usage.active_caches_size_in_bytes / limitBytes,
    missingMainFamilies,
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
  const gb = (bytes) => `${(bytes / 1_000_000_000).toFixed(2)} GB`;
  console.log(
    `${nameWithOwner}: ${gb(report.activeBytes)} / ${gb(report.limitBytes)} (${Math.round(report.utilization * 100)}%), ${report.activeCount} caches, ${report.retentionDays}-day retention.`,
  );
  console.log("Scope                         Caches    Size");
  for (const scope of report.scopes) {
    console.log(
      `${scope.ref.padEnd(30)} ${String(scope.count).padStart(5)}    ${gb(scope.bytes)}`,
    );
  }
  if (report.missingMainFamilies.length) {
    console.log(
      `Missing reusable main seeds: ${report.missingMainFamilies.join(", ")}.`,
    );
  }
  if (report.utilization >= 0.8) {
    console.log(
      "Storage is above 80%: inspect generations and PR/tag scopes before adding compiled-artifact families.",
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
