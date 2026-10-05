import assert from "node:assert/strict";
import {
  chmodSync,
  cpSync,
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  readlinkSync,
  rmSync,
  symlinkSync,
  utimesSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { once } from "node:events";
import test from "node:test";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("../..", import.meta.url)));
const helper = resolve(root, "scripts/lib/publish-generated-apple-pair.sh");
const failpoints = [
  "after_stage_swift",
  "after_stage_framework",
  "after_backup_swift",
  "after_backup_framework",
  "after_install_swift",
  "after_install_framework",
];

const command = [helper];

function createPair(directory, generationName, { oldPair = false } = {}) {
  const source = join(directory, `source-${generationName}`);
  const destination = join(directory, "destination");
  const sourceFramework = join(source, "Core.xcframework");
  const destinationFramework = join(
    destination,
    "Artifacts",
    "Core.xcframework",
  );
  const sourceSwift = join(source, "core.swift");
  const destinationSwift = join(destination, "Generated", "core.swift");
  mkdirSync(sourceFramework, { recursive: true });
  writeFileSync(sourceSwift, `${generationName}-swift`);
  writeFileSync(
    join(sourceFramework, "generation"),
    `${generationName}-framework`,
  );
  if (oldPair) {
    mkdirSync(destinationFramework, { recursive: true });
    mkdirSync(resolve(destinationSwift, ".."), { recursive: true });
    writeFileSync(destinationSwift, "old-swift");
    writeFileSync(join(destinationFramework, "generation"), "old-framework");
  }
  return {
    sourceSwift,
    destinationSwift,
    sourceFramework,
    destinationFramework,
  };
}

function invocation(pair) {
  return [
    ...command,
    pair.sourceSwift,
    pair.destinationSwift,
    pair.sourceFramework,
    pair.destinationFramework,
  ];
}

function publicationResidue(pair) {
  return [
    ...readdirSync(resolve(pair.destinationSwift, ".."), {
      withFileTypes: true,
    }),
    ...readdirSync(resolve(pair.destinationFramework, ".."), {
      withFileTypes: true,
    }),
  ].filter(
    (entry) =>
      entry.name.includes(".new.") ||
      entry.name.includes(".previous.") ||
      entry.name.endsWith(".publication.lock"),
  );
}

function publicationLock(pair) {
  return join(
    resolve(pair.destinationFramework, ".."),
    `.${pair.destinationFramework.split("/").at(-1)}.publication.lock`,
  );
}

async function waitForFile(path) {
  for (let attempt = 0; attempt < 500; attempt += 1) {
    if (existsSync(path)) return;
    await new Promise((resolvePromise) => setTimeout(resolvePromise, 10));
  }
  throw new Error(`timed out waiting for ${path}`);
}

async function runHeld(pair, directory, holdpoint) {
  const ready = join(directory, `${holdpoint}.ready`);
  const release = join(directory, `${holdpoint}.release`);
  const child = spawn("bash", invocation(pair), {
    detached: true,
    encoding: "utf8",
    env: {
      ...process.env,
      SYNARA_APPLE_PUBLICATION_TEST_HOLDPOINT: holdpoint,
      SYNARA_APPLE_PUBLICATION_TEST_READY_FILE: ready,
      SYNARA_APPLE_PUBLICATION_TEST_RELEASE_FILE: release,
    },
  });
  let stderr = "";
  child.stderr.setEncoding("utf8");
  child.stderr.on("data", (chunk) => {
    stderr += chunk;
  });
  await waitForFile(ready);
  return {
    child,
    release: () => writeFileSync(release, "release"),
    result: async () => {
      const [status, signal] = await once(child, "close");
      return { status, signal, stderr };
    },
  };
}

function transaction({ oldPair = true, failpoint } = {}) {
  const directory = mkdtempSync(
    join(tmpdir(), "synara-apple-publication-test."),
  );
  const pair = createPair(directory, "new", { oldPair });
  const result = spawnSync("bash", invocation(pair), {
    encoding: "utf8",
    env: {
      ...process.env,
      SYNARA_APPLE_PUBLICATION_TEST_FAILPOINT: failpoint ?? "",
    },
  });
  return {
    directory,
    ...pair,
    result,
    residue: publicationResidue(pair),
  };
}

function generation(pair) {
  return {
    swift: readFileSync(pair.destinationSwift, "utf8"),
    framework: readFileSync(
      join(pair.destinationFramework, "generation"),
      "utf8",
    ),
  };
}

function destinationSnapshot(pair) {
  function snapshot(path) {
    const info = lstatSync(path, { bigint: true });
    const entry = { inode: info.ino, mode: info.mode, mtime: info.mtimeNs };
    if (info.isSymbolicLink()) entry.link = readlinkSync(path);
    else if (info.isDirectory()) {
      entry.children = Object.fromEntries(
        readdirSync(path)
          .sort()
          .map((name) => [name, snapshot(join(path, name))]),
      );
    } else entry.bytes = readFileSync(path);
    return entry;
  }
  return [snapshot(pair.destinationSwift), snapshot(pair.destinationFramework)];
}

function matchingPair(directory) {
  const pair = createPair(directory, "old", { oldPair: true });
  mkdirSync(join(pair.destinationFramework, "Headers"));
  writeFileSync(
    join(pair.destinationFramework, "Headers", "core.h"),
    "ffi-header",
  );
  symlinkSync("Headers/core.h", join(pair.destinationFramework, "header-link"));
  symlinkSync(
    "missing-target",
    join(pair.destinationFramework, "dangling-link"),
  );
  chmodSync(join(pair.destinationFramework, "Headers", "core.h"), 0o640);
  cpSync(pair.destinationFramework, pair.sourceFramework, {
    recursive: true,
    verbatimSymlinks: true,
  });
  const oldTime = new Date("2001-01-01T00:00:00Z");
  utimesSync(pair.destinationSwift, oldTime, oldTime);
  utimesSync(pair.destinationFramework, oldTime, oldTime);
  utimesSync(
    join(pair.destinationFramework, "Headers", "core.h"),
    oldTime,
    oldTime,
  );
  return pair;
}

test("identical complete pairs preserve destination inodes, modes, links and modification times", () => {
  const directory = mkdtempSync(
    join(tmpdir(), "synara-apple-publication-noop-test."),
  );
  const pair = matchingPair(directory);
  const before = destinationSnapshot(pair);
  try {
    const result = spawnSync("bash", invocation(pair), { encoding: "utf8" });
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /generated pair unchanged/);
    assert.deepEqual(destinationSnapshot(pair), before);
    assert.equal(existsSync(pair.sourceSwift), false);
    assert.equal(existsSync(pair.sourceFramework), false);
    assert.deepEqual(publicationResidue(pair), []);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

const pairDifferences = {
  "Swift contents": (pair) => writeFileSync(pair.sourceSwift, "new-swift"),
  "Swift permissions": (pair) => chmodSync(pair.sourceSwift, 0o600),
  "archive contents": (pair) =>
    writeFileSync(join(pair.sourceFramework, "generation"), "new-framework"),
  "header contents": (pair) =>
    writeFileSync(
      join(pair.sourceFramework, "Headers", "core.h"),
      "new-header",
    ),
  "nested file permissions": (pair) =>
    chmodSync(join(pair.sourceFramework, "Headers", "core.h"), 0o600),
  "directory permissions": (pair) =>
    chmodSync(join(pair.sourceFramework, "Headers"), 0o700),
  "symlink targets": (pair) => {
    rmSync(join(pair.sourceFramework, "header-link"));
    symlinkSync("./Headers/core.h", join(pair.sourceFramework, "header-link"));
  },
  "symlink replaced by regular file": (pair) => {
    rmSync(join(pair.sourceFramework, "header-link"));
    writeFileSync(join(pair.sourceFramework, "header-link"), "ffi-header");
  },
  "additional hidden files": (pair) =>
    writeFileSync(join(pair.sourceFramework, ".metadata"), "new"),
  "missing files": (pair) =>
    rmSync(join(pair.sourceFramework, "Headers", "core.h")),
};

for (const [difference, change] of Object.entries(pairDifferences)) {
  test(`a difference in ${difference} republishes both outputs`, () => {
    const directory = mkdtempSync(
      join(tmpdir(), "synara-apple-publication-change-test."),
    );
    const pair = matchingPair(directory);
    const before = destinationSnapshot(pair);
    try {
      change(pair);
      const expected = destinationSnapshot({
        destinationSwift: pair.sourceSwift,
        destinationFramework: pair.sourceFramework,
      });
      const result = spawnSync("bash", invocation(pair), { encoding: "utf8" });
      assert.equal(result.status, 0, result.stderr);
      assert.doesNotMatch(result.stdout, /generated pair unchanged/);
      const after = destinationSnapshot(pair);
      assert.deepEqual(after, expected);
      assert.notEqual(after[0].inode, before[0].inode);
      assert.notEqual(after[1].inode, before[1].inode);
      assert.deepEqual(publicationResidue(pair), []);
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });
}

test("reads all bytes of large archives when checking unchanged pairs", () => {
  const directory = mkdtempSync(
    join(tmpdir(), "synara-apple-publication-large-test."),
  );
  const pair = matchingPair(directory);
  try {
    const original = Buffer.alloc(2 * 1024 * 1024 + 17, 3);
    writeFileSync(join(pair.destinationFramework, "generation"), original);
    const changed = Buffer.from(original);
    changed[changed.length - 1] = 4;
    writeFileSync(join(pair.sourceFramework, "generation"), changed);
    const result = spawnSync("bash", invocation(pair), { encoding: "utf8" });
    assert.equal(result.status, 0, result.stderr);
    assert.deepEqual(
      readFileSync(join(pair.destinationFramework, "generation")),
      changed,
    );
    assert.doesNotMatch(result.stdout, /generated pair unchanged/);
    assert.deepEqual(publicationResidue(pair), []);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("comparison failure preserves both destinations and cleans transaction residue", () => {
  const directory = mkdtempSync(
    join(tmpdir(), "synara-apple-publication-compare-failure-test."),
  );
  const pair = matchingPair(directory);
  const before = destinationSnapshot(pair);
  try {
    const bin = join(directory, "bin");
    mkdirSync(bin);
    writeFileSync(join(bin, "node"), "#!/bin/sh\nexit 2\n", { mode: 0o755 });
    const result = spawnSync("bash", invocation(pair), {
      encoding: "utf8",
      env: { ...process.env, PATH: `${bin}:${process.env.PATH}` },
    });
    assert.equal(result.status, 2, result.stderr);
    assert.deepEqual(destinationSnapshot(pair), before);
    assert.deepEqual(publicationResidue(pair), []);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("unsupported artifact types fail closed without treating comparison errors as a cache miss", () => {
  const directory = mkdtempSync(
    join(tmpdir(), "synara-apple-publication-unsupported-test."),
  );
  const pair = matchingPair(directory);
  try {
    const sourceFifo = join(pair.sourceFramework, "unexpected-fifo");
    const destinationFifo = join(pair.destinationFramework, "unexpected-fifo");
    const fixture = spawnSync("mkfifo", [sourceFifo, destinationFifo], {
      encoding: "utf8",
    });
    assert.equal(fixture.status, 0, fixture.stderr);
    const beforeSwift = lstatSync(pair.destinationSwift, { bigint: true });
    const beforeFramework = lstatSync(pair.destinationFramework, {
      bigint: true,
    });
    const result = spawnSync("bash", invocation(pair), {
      encoding: "utf8",
      timeout: 5000,
    });
    assert.equal(result.status, 2, result.stderr);
    assert.match(result.stderr, /unsupported generated artifact type/);
    assert.equal(
      lstatSync(pair.destinationSwift, { bigint: true }).ino,
      beforeSwift.ino,
    );
    assert.equal(
      lstatSync(pair.destinationFramework, { bigint: true }).ino,
      beforeFramework.ino,
    );
    assert.equal(lstatSync(destinationFifo).isFIFO(), true);
    assert.deepEqual(generation(pair), {
      swift: "old-swift",
      framework: "old-framework",
    });
    assert.deepEqual(publicationResidue(pair), []);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("unchanged pairs remain serialized while staged contents are being compared", async () => {
  const directory = mkdtempSync(
    join(tmpdir(), "synara-apple-publication-noop-lock-test."),
  );
  const pair = matchingPair(directory);
  const before = destinationSnapshot(pair);
  try {
    const contender = createPair(directory, "contender");
    const held = await runHeld(pair, directory, "after_stage_framework");
    const competingResult = spawnSync("bash", invocation(contender), {
      encoding: "utf8",
    });
    assert.equal(competingResult.status, 75, competingResult.stderr);
    held.release();
    const result = await held.result();
    assert.equal(result.status, 0, result.stderr);
    assert.deepEqual(destinationSnapshot(pair), before);
    assert.deepEqual(publicationResidue(pair), []);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("publishes a coherent new Swift and XCFramework pair", () => {
  const pair = transaction();
  try {
    assert.equal(pair.result.status, 0, pair.result.stderr);
    assert.deepEqual(generation(pair), {
      swift: "new-swift",
      framework: "new-framework",
    });
    assert.deepEqual(pair.residue, []);
  } finally {
    rmSync(pair.directory, { recursive: true, force: true });
  }
});

for (const failpoint of failpoints) {
  test(`rolls both outputs back at ${failpoint}`, () => {
    const pair = transaction({ failpoint });
    try {
      assert.notEqual(pair.result.status, 0);
      assert.deepEqual(generation(pair), {
        swift: "old-swift",
        framework: "old-framework",
      });
      assert.deepEqual(pair.residue, []);
    } finally {
      rmSync(pair.directory, { recursive: true, force: true });
    }
  });
}

test("removes a partially installed pair when no prior generation exists", () => {
  const pair = transaction({
    oldPair: false,
    failpoint: "after_install_swift",
  });
  try {
    assert.notEqual(pair.result.status, 0);
    assert.throws(() => readFileSync(pair.destinationSwift));
    assert.throws(() =>
      readFileSync(join(pair.destinationFramework, "generation")),
    );
    assert.deepEqual(pair.residue, []);
  } finally {
    rmSync(pair.directory, { recursive: true, force: true });
  }
});

test("keeps the coherent new pair if interrupted after commit", () => {
  const pair = transaction({ failpoint: "after_commit" });
  try {
    assert.notEqual(pair.result.status, 0);
    assert.deepEqual(generation(pair), {
      swift: "new-swift",
      framework: "new-framework",
    });
    assert.deepEqual(pair.residue, []);
  } finally {
    rmSync(pair.directory, { recursive: true, force: true });
  }
});

test("serializes concurrent publishers for the same destination pair", async () => {
  const directory = mkdtempSync(
    join(tmpdir(), "synara-apple-publication-concurrency-test."),
  );
  const first = createPair(directory, "first", { oldPair: true });
  const second = createPair(directory, "second");
  try {
    const held = await runHeld(first, directory, "after_stage_swift");
    const contender = spawnSync("bash", invocation(second), {
      encoding: "utf8",
    });
    assert.equal(contender.status, 75, contender.stderr);
    assert.deepEqual(generation(first), {
      swift: "old-swift",
      framework: "old-framework",
    });

    held.release();
    const completed = await held.result();
    assert.equal(completed.status, 0, completed.stderr);
    assert.deepEqual(generation(first), {
      swift: "first-swift",
      framework: "first-framework",
    });
    assert.deepEqual(publicationResidue(first), []);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("SIGTERM before commit restores the old coherent pair", async () => {
  const directory = mkdtempSync(
    join(tmpdir(), "synara-apple-publication-signal-test."),
  );
  const pair = createPair(directory, "new", { oldPair: true });
  try {
    const held = await runHeld(pair, directory, "after_install_swift");
    process.kill(-held.child.pid, "SIGTERM");
    const completed = await held.result();
    assert.equal(completed.status, 143, completed.stderr);
    assert.deepEqual(generation(pair), {
      swift: "old-swift",
      framework: "old-framework",
    });
    assert.deepEqual(publicationResidue(pair), []);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("SIGTERM after commit preserves the new coherent pair", async () => {
  const directory = mkdtempSync(
    join(tmpdir(), "synara-apple-publication-signal-test."),
  );
  const pair = createPair(directory, "new", { oldPair: true });
  try {
    const held = await runHeld(pair, directory, "after_commit");
    process.kill(-held.child.pid, "SIGTERM");
    const completed = await held.result();
    assert.equal(completed.status, 143, completed.stderr);
    assert.deepEqual(generation(pair), {
      swift: "new-swift",
      framework: "new-framework",
    });
    assert.deepEqual(publicationResidue(pair), []);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("a later publisher recovers a SIGKILL during partial installation", async () => {
  const directory = mkdtempSync(
    join(tmpdir(), "synara-apple-publication-kill-test."),
  );
  const pair = createPair(directory, "first", { oldPair: true });
  try {
    const held = await runHeld(pair, directory, "after_install_swift");
    process.kill(-held.child.pid, "SIGKILL");
    const killed = await held.result();
    assert.equal(killed.signal, "SIGKILL", killed.stderr);

    const recovery = createPair(directory, "recovery");
    const recoveryResult = spawnSync("bash", invocation(recovery), {
      encoding: "utf8",
      env: {
        ...process.env,
        SYNARA_APPLE_PUBLICATION_TEST_FAILPOINT: "after_stage_swift",
      },
    });
    assert.notEqual(recoveryResult.status, 0);
    assert.deepEqual(generation(pair), {
      swift: "old-swift",
      framework: "old-framework",
    });
    assert.deepEqual(publicationResidue(pair), []);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("a later publisher preserves a committed pair after SIGKILL", async () => {
  const directory = mkdtempSync(
    join(tmpdir(), "synara-apple-publication-kill-test."),
  );
  const pair = createPair(directory, "first", { oldPair: true });
  try {
    const held = await runHeld(pair, directory, "after_commit");
    process.kill(-held.child.pid, "SIGKILL");
    const killed = await held.result();
    assert.equal(killed.signal, "SIGKILL", killed.stderr);

    const recovery = createPair(directory, "recovery");
    const recoveryResult = spawnSync("bash", invocation(recovery), {
      encoding: "utf8",
      env: {
        ...process.env,
        SYNARA_APPLE_PUBLICATION_TEST_FAILPOINT: "after_stage_swift",
      },
    });
    assert.notEqual(recoveryResult.status, 0);
    assert.deepEqual(generation(pair), {
      swift: "first-swift",
      framework: "first-framework",
    });
    assert.deepEqual(publicationResidue(pair), []);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("only one publisher can claim recovery of a stale transaction", async () => {
  const directory = mkdtempSync(
    join(tmpdir(), "synara-apple-publication-recovery-race-test."),
  );
  const stalePair = createPair(directory, "stale", { oldPair: true });
  try {
    const killedPublisher = await runHeld(
      stalePair,
      directory,
      "after_install_swift",
    );
    process.kill(-killedPublisher.child.pid, "SIGKILL");
    const killed = await killedPublisher.result();
    assert.equal(killed.signal, "SIGKILL", killed.stderr);

    const recoveryWinner = createPair(directory, "recovery-winner");
    const heldRecovery = await runHeld(
      recoveryWinner,
      directory,
      "during_stale_recovery",
    );
    const recoveryContender = createPair(directory, "recovery-contender");
    const contenderResult = spawnSync("bash", invocation(recoveryContender), {
      encoding: "utf8",
    });
    assert.equal(contenderResult.status, 75, contenderResult.stderr);

    heldRecovery.release();
    const winnerResult = await heldRecovery.result();
    assert.equal(winnerResult.status, 0, winnerResult.stderr);
    assert.deepEqual(generation(stalePair), {
      swift: "recovery-winner-swift",
      framework: "recovery-winner-framework",
    });
    assert.deepEqual(publicationResidue(stalePair), []);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("a reused live PID does not impersonate a stale publication owner", async () => {
  const directory = mkdtempSync(
    join(tmpdir(), "synara-apple-publication-pid-reuse-test."),
  );
  const stalePair = createPair(directory, "stale", { oldPair: true });
  try {
    const killedPublisher = await runHeld(
      stalePair,
      directory,
      "after_install_swift",
    );
    process.kill(-killedPublisher.child.pid, "SIGKILL");
    const killed = await killedPublisher.result();
    assert.equal(killed.signal, "SIGKILL", killed.stderr);

    const lock = publicationLock(stalePair);
    writeFileSync(join(lock, "owner"), `${process.pid}\n`);
    writeFileSync(
      join(lock, "owner-start"),
      "a deliberately different process start\n",
    );
    const recovery = createPair(directory, "recovery");
    const result = spawnSync("bash", invocation(recovery), {
      encoding: "utf8",
      env: {
        ...process.env,
        SYNARA_APPLE_PUBLICATION_TEST_FAILPOINT: "after_stage_swift",
      },
    });
    assert.notEqual(result.status, 0);
    assert.doesNotMatch(result.stderr, /publication already active/);
    assert.deepEqual(generation(stalePair), {
      swift: "old-swift",
      framework: "old-framework",
    });
    assert.deepEqual(publicationResidue(stalePair), []);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("a recovery operation failure leaves the stale transaction fail-closed", async () => {
  const directory = mkdtempSync(
    join(tmpdir(), "synara-apple-publication-recovery-failure-test."),
  );
  const stalePair = createPair(directory, "stale", { oldPair: true });
  try {
    const killedPublisher = await runHeld(
      stalePair,
      directory,
      "after_install_swift",
    );
    process.kill(-killedPublisher.child.pid, "SIGKILL");
    const killed = await killedPublisher.result();
    assert.equal(killed.signal, "SIGKILL", killed.stderr);

    const failedRecovery = createPair(directory, "failed-recovery");
    const failureResult = spawnSync("bash", invocation(failedRecovery), {
      encoding: "utf8",
      env: {
        ...process.env,
        SYNARA_APPLE_PUBLICATION_TEST_RECOVERY_FAILPOINT:
          "after_recovery_claim",
      },
    });
    assert.equal(failureResult.status, 96, failureResult.stderr);
    assert.equal(
      readFileSync(stalePair.destinationSwift, "utf8"),
      "stale-swift",
    );
    assert.throws(() =>
      readFileSync(join(stalePair.destinationFramework, "generation")),
    );

    const blockedContender = createPair(directory, "blocked-contender");
    const blockedResult = spawnSync("bash", invocation(blockedContender), {
      encoding: "utf8",
    });
    assert.equal(blockedResult.status, 74, blockedResult.stderr);
    assert.match(
      blockedResult.stderr,
      /recovery owner .* is no longer running/,
    );
    assert.equal(
      readFileSync(stalePair.destinationSwift, "utf8"),
      "stale-swift",
    );
    assert.throws(() =>
      readFileSync(join(stalePair.destinationFramework, "generation")),
    );
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
