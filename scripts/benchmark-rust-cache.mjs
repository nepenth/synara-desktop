#!/usr/bin/env node

import {
  appendFileSync,
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  statfsSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";

const repository = resolve(fileURLToPath(new URL("..", import.meta.url)));
const limits =
  "In-job host NSE dev build only. Excludes dependency downloads, GitHub cache restore/save, source changes, Apple cross-compilation, Xcode, signing and full desktop builds. Fixed order can favor later runs through filesystem warming.";

function sourceFingerprint(root) {
  const files = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml"];
  function collect(directory) {
    if (!existsSync(join(root, directory))) return;
    for (const entry of readdirSync(join(root, directory), {
      withFileTypes: true,
    })) {
      const path = join(directory, entry.name);
      // --lib never compiles integration test harnesses. Their edits must not
      // invalidate a production codegen experiment during parallel test work.
      if (
        entry.isDirectory() &&
        !["target", "tests", "benches", "examples"].includes(entry.name)
      )
        collect(path);
      else if (entry.isFile() && /\.(rs|toml|udl)$/.test(path))
        files.push(path);
    }
  }
  collect("crates");
  const hash = createHash("sha256");
  for (const path of files.sort()) {
    if (!existsSync(join(root, path))) continue;
    hash
      .update(path)
      .update("\0")
      .update(readFileSync(join(root, path)))
      .update("\0");
  }
  return hash.digest("hex");
}

function bytes(path) {
  if (!existsSync(path)) return 0;
  try {
    const info = lstatSync(path);
    if (info.isSymbolicLink()) return 0;
    if (!info.isDirectory()) return info.size;
    return readdirSync(path).reduce(
      (total, name) => total + bytes(join(path, name)),
      0,
    );
  } catch (error) {
    if (error.code === "ENOENT") return 0;
    throw error;
  }
}

function execute(
  command,
  args,
  { root, env, scratch, maxBytes, minFreeBytes },
) {
  return new Promise((resolvePromise, reject) => {
    const available = () => {
      const filesystem = statfsSync(scratch);
      return filesystem.bavail * filesystem.bsize;
    };
    if (scratch && available() < minFreeBytes) {
      reject(new Error("benchmark filesystem is below its free-space reserve"));
      return;
    }
    const started = performance.now();
    const child = spawn(command, args, {
      cwd: root,
      env,
      detached: process.platform !== "win32",
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    let resourceFailure;
    const terminate = () => {
      if (process.platform === "win32") child.kill("SIGTERM");
      else {
        try {
          process.kill(-child.pid, "SIGTERM");
        } catch {}
      }
    };
    const interrupted = (signal) => {
      resourceFailure = new Error(`benchmark interrupted by ${signal}`);
      terminate();
    };
    const onInterrupt = () => interrupted("SIGINT");
    const onTerminate = () => interrupted("SIGTERM");
    process.once("SIGINT", onInterrupt);
    process.once("SIGTERM", onTerminate);
    const cleanup = () => {
      clearInterval(monitor);
      process.removeListener("SIGINT", onInterrupt);
      process.removeListener("SIGTERM", onTerminate);
    };
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    const monitor = scratch
      ? setInterval(() => {
          if (resourceFailure) return;
          try {
            if (available() < minFreeBytes) {
              resourceFailure = new Error(
                "benchmark filesystem is below its free-space reserve",
              );
            } else if (bytes(scratch) <= maxBytes) return;
            else
              resourceFailure = new Error(
                "benchmark scratch storage exceeded its limit",
              );
          } catch (error) {
            resourceFailure = error;
          }
          terminate();
        }, 1000)
      : undefined;
    child.on("error", (error) => {
      cleanup();
      reject(error);
    });
    child.on("close", (status, signal) => {
      cleanup();
      const result = {
        status,
        signal,
        seconds: (performance.now() - started) / 1000,
        stdout,
        stderr,
      };
      if (resourceFailure) reject(resourceFailure);
      else if (status !== 0)
        reject(
          new Error(
            `${command} ${args.join(" ")} failed (${status ?? signal}): ${stderr}`,
          ),
        );
      else resolvePromise(result);
    });
  });
}

export async function benchmark({
  root = repository,
  output = join(tmpdir(), "synara-build-cache-benchmark.json"),
  cargo = "cargo",
  kache = "kache",
  scratchParent = tmpdir(),
  env = process.env,
  maxBytes = 1.8 * 1024 ** 3,
  minFreeBytes = 3 * 1024 ** 3,
} = {}) {
  // All destructive cleanup is confined to this freshly created directory.
  const scratch = mkdtempSync(join(scratchParent, "synara-cache-bench-"));
  const target = join(scratch, "target");
  const cache = join(scratch, "cache");
  const runtime = join(scratch, "run");
  const config = join(scratch, "kache.toml");
  const isolatedEnv = Object.fromEntries(
    Object.entries(env).filter(([name]) => !name.startsWith("KACHE_")),
  );
  Object.assign(isolatedEnv, {
    CARGO_TARGET_DIR: target,
    CARGO_INCREMENTAL: "0",
    CARGO_PROFILE_DEV_DEBUG: "0",
    CARGO_PROFILE_DEV_CODEGEN_UNITS: "16",
    CARGO_BUILD_JOBS: "2",
    RUSTC_WRAPPER: "",
    RUSTC_WORKSPACE_WRAPPER: "",
    KACHE_CONFIG: config,
    KACHE_HOST_CONFIG: "",
    KACHE_CACHE_DIR: cache,
    KACHE_RUNTIME_DIR: runtime,
    KACHE_LOCAL_ONLY: "1",
    KACHE_MAX_SIZE: "512MiB",
    KACHE_CACHE_EXECUTABLES: "1",
  });
  writeFileSync(
    config,
    `[cache]\nlocal_only = true\nlocal_store = ${JSON.stringify(cache)}\nruntime_dir = ${JSON.stringify(runtime)}\nlocal_max_size = "512MiB"\ncache_executables = true\n`,
  );
  const commandOptions = {
    root,
    env: isolatedEnv,
    scratch,
    maxBytes,
    minFreeBytes,
  };
  const report = {
    schemaVersion: 1,
    startedAt: new Date().toISOString(),
    status: "running",
    scope: limits,
    sourceFingerprint: sourceFingerprint(root),
    spaceMeasurement:
      "Logical file sizes; hardlinks count per path. A separate filesystem free-space reserve protects shared disk.",
    resourceLimits: { maxScratchBytes: maxBytes, minFreeBytes },
    workload: {
      package: "synara-nse-core",
      profile: "dev",
      target: "host",
      incremental: false,
      debug: 0,
      codegenUnits: 16,
      jobs: 2,
    },
    cache: {
      version: "0.28.1",
      action: "1a33fb2ff51be23eb9e87abeae6edb65be78f71c",
      maxBytes: 512 * 1024 ** 2,
      remote: false,
      githubCache: false,
      limitation:
        "The deliberately small 512MiB local store may evict entries; measure larger budgets separately before adopting a backend.",
    },
    runs: [],
  };
  let failure;
  let kacheStarted = false;
  try {
    report.toolchain = (
      await execute(cargo, ["--version"], commandOptions)
    ).stdout.trim();
    report.rustc = (
      await execute("rustc", ["-vV"], commandOptions)
    ).stdout.trim();
    report.commit = (
      await execute("git", ["rev-parse", "HEAD"], commandOptions)
    ).stdout.trim();
    report.dirty = Boolean(
      (
        await execute("git", ["status", "--porcelain"], commandOptions)
      ).stdout.trim(),
    );
    report.cache.installedVersion = (
      await execute(kache, ["--version"], commandOptions)
    ).stdout.trim();
    if (report.cache.installedVersion !== "kache 0.28.1")
      throw new Error("benchmark requires Kache 0.28.1");
    // Fetch before timing: both strategies use the same available source graph.
    await execute(
      cargo,
      [
        "fetch",
        "--locked",
        "--manifest-path",
        join(root, "crates/synara-nse-core/Cargo.toml"),
      ],
      commandOptions,
    );
    const buildArgs = [
      "build",
      "--locked",
      "--package",
      "synara-nse-core",
      "--lib",
      "--manifest-path",
      join(root, "Cargo.toml"),
    ];
    for (const name of [
      "baseline-first",
      "baseline-repeat",
      "kache-cold",
      "kache-warm",
    ]) {
      if (sourceFingerprint(root) !== report.sourceFingerprint)
        throw new Error("benchmark source graph changed between measurements");
      report.activeRun = name;
      rmSync(target, { recursive: true, force: true });
      if (existsSync(target))
        throw new Error("benchmark target was not removed");
      const wrapped = name.startsWith("kache-");
      isolatedEnv.RUSTC_WRAPPER = wrapped ? kache : "";
      console.error(`Benchmark: ${name} (fresh Cargo target)`);
      kacheStarted ||= wrapped;
      const before = wrapped
        ? JSON.parse(
            (await execute(kache, ["stats", "--json"], commandOptions)).stdout,
          )
        : undefined;
      const result = await execute(cargo, buildArgs, commandOptions);
      if (sourceFingerprint(root) !== report.sourceFingerprint)
        throw new Error("benchmark source graph changed during measurement");
      const after = wrapped
        ? JSON.parse(
            (await execute(kache, ["stats", "--json"], commandOptions)).stdout,
          )
        : undefined;
      const counters = [
        "local_hits",
        "prefetch_hits",
        "remote_hits",
        "misses",
        "dups",
      ];
      report.runs.push({
        name,
        seconds: result.seconds,
        targetBytes: bytes(target),
        cacheBytes: bytes(cache),
        scratchBytes: bytes(scratch),
        ...(wrapped
          ? {
              stats: after,
              statsDelta: Object.fromEntries(
                counters.map((key) => [
                  key,
                  (after[key] ?? 0) - (before[key] ?? 0),
                ]),
              ),
            }
          : {}),
        buildStatus: result.status,
      });
      if (bytes(scratch) > maxBytes)
        throw new Error("benchmark scratch storage exceeded its limit");
    }
    report.status = "passed";
    delete report.activeRun;
    report.verdict =
      report.runs.at(-1).statsDelta.local_hits > 0
        ? "warm-cache-hits-observed"
        : "no-warm-cache-hits-observed";
  } catch (error) {
    failure = error;
    report.status = "failed";
    report.error = error.message;
  } finally {
    if (kacheStarted) {
      try {
        await execute(kache, ["daemon", "stop"], {
          ...commandOptions,
          scratch: undefined,
        });
      } catch (error) {
        failure ??= error;
        report.status = "failed";
        report.cleanupError = error.message;
      }
    }
    rmSync(scratch, { recursive: true, force: true });
    report.completedAt = new Date().toISOString();
    mkdirSync(dirname(resolve(output)), { recursive: true });
    writeFileSync(output, `${JSON.stringify(report, null, 2)}\n`);
    if (env.GITHUB_STEP_SUMMARY) {
      const rows = report.runs.map(
        (run) =>
          `| ${run.name} | ${run.seconds.toFixed(2)} | ${(run.targetBytes / 1024 ** 2).toFixed(1)} | ${(run.cacheBytes / 1024 ** 2).toFixed(1)} | ${run.statsDelta?.local_hits ?? "—"} | ${run.statsDelta?.misses ?? "—"} |`,
      );
      appendFileSync(
        env.GITHUB_STEP_SUMMARY,
        `## Rust cache benchmark: ${report.status}\n\n${limits}\n\nPackage: synara-nse-core; host dev profile; debug/incremental disabled; 16 codegen units; 2 jobs. Commit: ${report.commit}; dirty: ${report.dirty}.\n\nToolchain: ${report.toolchain}; ${report.rustc?.split("\n")[0]}; ${report.rustc?.split("\n").find((line) => line.startsWith("commit-hash:"))}. Kache: ${report.cache.installedVersion}. Source fingerprint: ${report.sourceFingerprint}.\n\n${report.cache.limitation}\n\n| Run | Seconds | Target MiB | Cache MiB | Local hits | Misses |\n|---|---:|---:|---:|---:|---:|\n${rows.join("\n")}\n\n${report.error ?? report.cleanupError ?? ""}\n`,
      );
    }
  }
  if (failure) throw failure;
  return report;
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const args = process.argv.slice(2);
  if (args.length !== 0 && !(args.length === 2 && args[0] === "--output")) {
    console.error(
      "Usage: node scripts/benchmark-rust-cache.mjs [--output path]",
    );
    process.exitCode = 64;
  } else {
    try {
      const report = await benchmark({ output: args[1] });
      console.log(
        JSON.stringify(
          report.runs.map(({ name, seconds, statsDelta }) => ({
            name,
            seconds,
            statsDelta,
          })),
          null,
          2,
        ),
      );
    } catch (error) {
      console.error(error.message);
      process.exitCode = 1;
    }
  }
}
