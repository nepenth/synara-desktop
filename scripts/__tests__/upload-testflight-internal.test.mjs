import assert from "node:assert/strict";
import { chmod, mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../.."
);
const uploadScript = path.join(
  repositoryRoot,
  "synara-ios/scripts/upload-testflight-internal.sh"
);

async function createHarness({
  exportBehaviors = ["ok:0"],
  exportRetries,
  retrySeconds = 0,
} = {}) {
  const root = await mkdtemp(
    path.join(os.tmpdir(), "synara-testflight-upload-")
  );
  const binDirectory = path.join(root, "bin");
  const diagnosticsDirectory = path.join(root, "diagnostics");
  const distributionLogs = path.join(root, "Synara_fixture.xcdistributionlogs");
  const archiveCheckMarker = path.join(root, "archive-check.txt");
  const archiveCountFile = path.join(root, "archive-count.txt");
  const exportCountFile = path.join(root, "export-count.txt");
  const outputFile = path.join(root, "github-output.txt");
  await Promise.all([
    mkdir(binDirectory, { recursive: true }),
    mkdir(distributionLogs, { recursive: true }),
    writeFile(outputFile, "", "utf8"),
    writeFile(archiveCountFile, "0", "utf8"),
    writeFile(exportCountFile, "0", "utf8"),
  ]);
  await writeFile(
    path.join(distributionLogs, "DistributionSummary.plist"),
    "fixture diagnostics",
    "utf8"
  );

  const fakeXcodebuild = path.join(binDirectory, "xcodebuild");
  await writeFile(
    fakeXcodebuild,
    `#!/usr/bin/env bash
set -euo pipefail
if [[ " $* " == *" -showBuildSettings "* ]]; then
  printf '    MARKETING_VERSION = 1.2.56\\n'
  printf '    CURRENT_PROJECT_VERSION = 1.2.57\\n'
  exit 0
fi
if [[ " $* " == *" -exportArchive "* ]]; then
  count="$(cat "$FAKE_EXPORT_COUNT_FILE")"
  count=$((count + 1))
  printf '%s\\n' "$count" > "$FAKE_EXPORT_COUNT_FILE"
  IFS=',' read -r -a behaviors <<< "$FAKE_EXPORT_BEHAVIORS"
  index=$((count - 1))
  behavior="\${behaviors[$index]:-signing:17}"
  kind="\${behavior%%:*}"
  status="\${behavior##*:}"
  printf 'Created bundle at path "%s".\\n' "$FAKE_DISTRIBUTION_LOGS"
  case "$kind" in
    ok)
      printf 'fixture export\\n'
      ;;
    transient)
      printf 'Account credentials have expired.\\n'
      printf 'reauthenticationNotSupported\\n'
      ;;
    duplicate)
      printf 'Redundant Binary Upload. There already exists a binary upload with this build.\\n'
      printf 'ITMS-4230\\n'
      ;;
    *)
      printf 'fixture export\\n'
      ;;
  esac
  exit "$status"
fi
count="$(cat "$FAKE_ARCHIVE_COUNT_FILE")"
printf '%s\\n' "$((count + 1))" > "$FAKE_ARCHIVE_COUNT_FILE"
printf 'fixture archive\\n'
`,
    "utf8"
  );
  await chmod(fakeXcodebuild, 0o755);

  const fakeArchiveChecker = path.join(binDirectory, "check-archive");
  await writeFile(
    fakeArchiveChecker,
    `#!/usr/bin/env bash
set -euo pipefail
[[ "$1" == "$SYNARA_EXPECTED_ARCHIVE_PATH" ]]
[[ "$2" == "$SYNARA_IOS_DIAGNOSTICS_DIR" ]]
printf 'checked\\n' > "$SYNARA_ARCHIVE_CHECK_MARKER"
`,
    "utf8"
  );
  await chmod(fakeArchiveChecker, 0o755);

  const env = {
    ...process.env,
    PATH: `${binDirectory}:${process.env.PATH}`,
    TMPDIR: root,
    GITHUB_OUTPUT: outputFile,
    FAKE_DISTRIBUTION_LOGS: distributionLogs,
    FAKE_EXPORT_BEHAVIORS: exportBehaviors.join(","),
    FAKE_ARCHIVE_COUNT_FILE: archiveCountFile,
    FAKE_EXPORT_COUNT_FILE: exportCountFile,
    SYNARA_IOS_TEAM_ID: "TEAM",
    SYNARA_IOS_PROVISIONING_PROFILE: "app-profile",
    SYNARA_IOS_NOTIFICATION_SERVICE_PROVISIONING_PROFILE:
      "notification-profile",
    SYNARA_PUSH_GATEWAY_URL: "https://push.example.test/_matrix/push/v1/notify",
    SYNARA_IOS_ARCHIVE_ROOT: root,
    SYNARA_IOS_DIAGNOSTICS_DIR: diagnosticsDirectory,
    SYNARA_IOS_NOTIFICATION_ARCHIVE_CHECKER: fakeArchiveChecker,
    SYNARA_EXPECTED_ARCHIVE_PATH: path.join(
      root,
      "Synara-1.2.56-1.2.57.xcarchive"
    ),
    SYNARA_ARCHIVE_CHECK_MARKER: archiveCheckMarker,
    SYNARA_TESTFLIGHT_EXPORT_RETRY_SECONDS: String(retrySeconds),
  };
  if (exportRetries !== undefined) {
    env.SYNARA_TESTFLIGHT_EXPORT_RETRIES = String(exportRetries);
  }

  const result = spawnSync(uploadScript, {
    cwd: repositoryRoot,
    encoding: "utf8",
    env,
  });
  return {
    archiveCheckMarker,
    archiveCountFile,
    diagnosticsDirectory,
    distributionLogs,
    exportCountFile,
    outputFile,
    result,
  };
}

test("preserves version outputs, command logs, and Xcode distribution diagnostics", async () => {
  const harness = await createHarness();
  assert.equal(harness.result.status, 0, harness.result.stderr);
  assert.match(
    await readFile(harness.outputFile, "utf8"),
    /marketing_version=1\.2\.56/
  );
  assert.match(
    await readFile(harness.outputFile, "utf8"),
    /build_number=1\.2\.57/
  );
  assert.equal(await readFile(harness.archiveCheckMarker, "utf8"), "checked\n");
  assert.equal(await readFile(harness.archiveCountFile, "utf8"), "1\n");
  assert.equal(await readFile(harness.exportCountFile, "utf8"), "1\n");
  assert.match(
    await readFile(
      path.join(harness.diagnosticsDirectory, "xcodebuild-archive.log"),
      "utf8"
    ),
    /fixture archive/
  );
  assert.match(
    await readFile(
      path.join(harness.diagnosticsDirectory, "xcodebuild-export.log"),
      "utf8"
    ),
    /fixture export/
  );
  assert.equal(
    await readFile(
      path.join(
        harness.diagnosticsDirectory,
        path.basename(harness.distributionLogs),
        "DistributionSummary.plist"
      ),
      "utf8"
    ),
    "fixture diagnostics"
  );
});

test("returns the original Xcode export status after capturing diagnostics", async () => {
  const harness = await createHarness({ exportBehaviors: ["signing:17"] });
  assert.equal(harness.result.status, 17);
  assert.equal(await readFile(harness.exportCountFile, "utf8"), "1\n");
  assert.equal(
    await readFile(
      path.join(
        harness.diagnosticsDirectory,
        path.basename(harness.distributionLogs),
        "DistributionSummary.plist"
      ),
      "utf8"
    ),
    "fixture diagnostics"
  );
});

test("retries only export after a transient App Store Connect auth failure", async () => {
  const harness = await createHarness({
    exportBehaviors: ["transient:70", "ok:0"],
  });
  assert.equal(harness.result.status, 0, harness.result.stderr);
  assert.equal(await readFile(harness.archiveCountFile, "utf8"), "1\n");
  assert.equal(await readFile(harness.exportCountFile, "utf8"), "2\n");
  assert.match(harness.result.stdout, /Retrying App Store Connect export/);
  assert.match(
    await readFile(
      path.join(
        harness.diagnosticsDirectory,
        "xcodebuild-export-attempt-2.log"
      ),
      "utf8"
    ),
    /fixture export/
  );
  assert.match(
    await readFile(
      path.join(harness.diagnosticsDirectory, "xcodebuild-export.log"),
      "utf8"
    ),
    /fixture export/
  );
});

test("does not retry signing or archive-unrelated export failures", async () => {
  const harness = await createHarness({ exportBehaviors: ["signing:17"] });
  assert.equal(harness.result.status, 17);
  assert.equal(await readFile(harness.archiveCountFile, "utf8"), "1\n");
  assert.equal(await readFile(harness.exportCountFile, "utf8"), "1\n");
  assert.doesNotMatch(
    harness.result.stdout,
    /Retrying App Store Connect export/
  );
});

test("treats a redundant App Store Connect build as export success", async () => {
  const harness = await createHarness({
    exportBehaviors: ["duplicate:65"],
  });
  assert.equal(harness.result.status, 0, harness.result.stderr);
  assert.equal(await readFile(harness.exportCountFile, "utf8"), "1\n");
  assert.match(
    harness.result.stdout,
    /already has this build; treating export as success/
  );
});

test("treats a later redundant upload as success after a transient first attempt", async () => {
  const harness = await createHarness({
    exportBehaviors: ["transient:70", "duplicate:65"],
  });
  assert.equal(harness.result.status, 0, harness.result.stderr);
  assert.equal(await readFile(harness.archiveCountFile, "utf8"), "1\n");
  assert.equal(await readFile(harness.exportCountFile, "utf8"), "2\n");
});

test("exhausts transient export retries and keeps the last Xcode status", async () => {
  const harness = await createHarness({
    exportBehaviors: ["transient:70", "transient:70", "transient:70"],
    exportRetries: 2,
  });
  assert.equal(harness.result.status, 70);
  assert.equal(await readFile(harness.archiveCountFile, "utf8"), "1\n");
  assert.equal(await readFile(harness.exportCountFile, "utf8"), "3\n");
  assert.match(
    await readFile(
      path.join(harness.diagnosticsDirectory, "xcodebuild-export.log"),
      "utf8"
    ),
    /Account credentials have expired/
  );
});
