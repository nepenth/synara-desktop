import { spawnSync } from "node:child_process";
import { readFileSync, realpathSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const repository = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);

function sizeBytes(value) {
  const match = /^(\d+(?:\.\d+)?)\s*(B|KiB|MiB|GiB|TiB|KB|MB|GB|TB)$/i.exec(
    value ?? "",
  );
  if (!match)
    throw new Error("Expected an explicit Kache cache size with units.");
  const unit = match[2].toLowerCase();
  const exponent = {
    b: 0,
    kb: 1,
    kib: 1,
    mb: 2,
    mib: 2,
    gb: 3,
    gib: 3,
    tb: 4,
    tib: 4,
  }[unit];
  const bytes =
    Number(match[1]) * (unit.includes("i") ? 1024 : 1000) ** exponent;
  if (!Number.isSafeInteger(bytes) || bytes <= 0)
    throw new Error("Invalid Kache cache size.");
  return bytes;
}

function absolutePath(value, name) {
  if (!value || !path.isAbsolute(value))
    throw new Error(`${name} must identify an absolute owned path.`);
  return value;
}

function samePath(actual, expected) {
  if (typeof actual !== "string" || !path.isAbsolute(actual)) return false;
  const normalize = (value) => {
    try {
      return realpathSync(value);
    } catch {
      return path.resolve(value);
    }
  };
  return normalize(actual) === normalize(expected);
}

export function startCiRustCache({ env = process.env } = {}) {
  const binary = absolutePath(env.RUSTC_WRAPPER, "RUSTC_WRAPPER");
  const cache = absolutePath(env.KACHE_CACHE_DIR, "KACHE_CACHE_DIR");
  const runtime = absolutePath(env.KACHE_RUNTIME_DIR, "KACHE_RUNTIME_DIR");
  const config = absolutePath(env.KACHE_CONFIG, "KACHE_CONFIG");
  if (samePath(cache, runtime))
    throw new Error("Daemon runtime must be separate from the compiler store.");
  const expectedBytes = sizeBytes(env.SYNARA_CACHE_MAX_SIZE);
  if (sizeBytes(env.KACHE_MAX_SIZE) !== expectedBytes)
    throw new Error(
      "Kache action exported a different cache budget than requested.",
    );
  const expectedVersion = readFileSync(
    path.join(repository, "scripts/lib/rust-cache.sh"),
    "utf8",
  ).match(/^SYNARA_KACHE_VERSION=([0-9.]+)$/m)?.[1];
  if (!expectedVersion) throw new Error("The shared Kache version is missing.");
  const command = (args) => {
    const result = spawnSync(binary, args, {
      env,
      encoding: "utf8",
      timeout: 45000,
      maxBuffer: 1024 * 1024,
    });
    if (result.error || result.status !== 0)
      throw new Error(
        `Kache ${args.join(" ")} failed: ${result.error?.message ?? result.stderr.trim()}`,
      );
    if (result.stderr) process.stderr.write(result.stderr);
    return result.stdout;
  };
  if (command(["--version"]).trim() !== `kache ${expectedVersion}`)
    throw new Error("The installed Kache does not match the shared version.");
  // Local-only upstream setup does not start a daemon, but its strict post step
  // always stops the owned runtime. Establish a real owner before compilation.
  command(["daemon", "start"]);
  const json = (args) => {
    const value = JSON.parse(command(["--json", ...args]));
    if (value.schema_version !== 1 || value.success !== true)
      throw new Error(
        "Kache returned an unsupported or failed diagnostic response.",
      );
    return value;
  };
  const status = json(["daemon", "status"]);
  if (
    !status.daemon_running ||
    status.daemon_version !== expectedVersion ||
    !samePath(status.socket, path.join(runtime, "daemon.sock")) ||
    !samePath(status.daemon_config_path, config)
  ) {
    throw new Error(
      "Kache daemon did not confirm the expected version, runtime socket and repository configuration.",
    );
  }
  const stats = json(["stats"]);
  if (
    !stats.daemon_connected ||
    stats.daemon_version !== expectedVersion ||
    stats.disk?.store_limit_bytes !== expectedBytes ||
    stats.stores?.length !== 1 ||
    !samePath(stats.stores[0].path, cache) ||
    stats.stores[0].max_size !== expectedBytes ||
    stats.stores[0].error
  ) {
    throw new Error(
      "Kache daemon did not confirm the expected compiler store and cache budget.",
    );
  }
  return {
    version: expectedVersion,
    cache,
    runtime,
    budgetBytes: expectedBytes,
  };
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href
) {
  try {
    const result = startCiRustCache();
    console.log(
      `Verified Kache ${result.version} job daemon and ${result.budgetBytes}-byte cache budget.`,
    );
  } catch (error) {
    console.error(`[rust-cache-start] ${error.message}`);
    process.exitCode = 1;
  }
}
