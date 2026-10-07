// Keep only the newest Actions cache entry per family and ref.
//
// Cache keys end in content or time suffixes (lockfile hash, commit SHA,
// compiler identity digest, ISO week). Entries that differ only in those
// suffixes are superseded snapshots of the same family; the newest one is the
// only one a later restore would choose. Deleting the rest keeps the release
// families inside the repository cache limit.
import { execFileSync } from "node:child_process";
import { pathToFileURL } from "node:url";

const SUFFIX = /-(?:[0-9a-f]{40,64}|\d{4}-W\d{2})$/;

export function cacheFamily(key) {
  let family = String(key);
  while (SUFFIX.test(family)) family = family.replace(SUFFIX, "");
  return family;
}

export function selectSupersededCaches(caches) {
  const newest = new Map();
  for (const cache of caches) {
    const group = `${cache.ref}\u0000${cacheFamily(cache.key)}`;
    const current = newest.get(group);
    if (!current || Date.parse(cache.created_at) > Date.parse(current.created_at)) {
      newest.set(group, cache);
    }
  }
  const keep = new Set([...newest.values()].map((cache) => cache.id));
  return caches.filter((cache) => !keep.has(cache.id));
}

function gh(args) {
  return execFileSync("gh", args, { encoding: "utf8" });
}

function listCaches(repo) {
  const raw = gh([
    "api",
    "--paginate",
    `repos/${repo}/actions/caches?per_page=100`,
    "--jq",
    ".actions_caches[] | {id, key, ref, created_at, size_in_bytes}",
  ]);
  return raw
    .split("\n")
    .filter(Boolean)
    .map((line) => JSON.parse(line));
}

function main() {
  const repo = process.env.GITHUB_REPOSITORY;
  if (!repo) throw new Error("GITHUB_REPOSITORY is required");
  const caches = listCaches(repo);
  const stale = selectSupersededCaches(caches);
  const mib = (bytes) => Math.round(bytes / 1048576);
  for (const cache of stale) {
    console.log(`Deleting ${cache.ref} ${cache.key} (${mib(cache.size_in_bytes)} MiB)`);
    gh(["api", "--method", "DELETE", `repos/${repo}/actions/caches/${cache.id}`]);
  }
  const kept = caches.filter((cache) => !stale.includes(cache));
  const total = kept.reduce((sum, cache) => sum + cache.size_in_bytes, 0);
  console.log(`Kept ${kept.length} cache entries, ${mib(total)} MiB in total.`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
