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

async function createHarness(exportStatus) {
  const root = await mkdtemp(
    path.join(os.tmpdir(), "synara-testflight-upload-")
  );
  const binDirectory = path.join(root, "bin");
  const diagnosticsDirectory = path.join(root, "diagnostics");
  const distributionLogs = path.join(root, "Synara_fixture.xcdistributionlogs");
  const archiveCheckMarker = path.join(root, "archive-check.txt");
  const outputFile = path.join(root, "github-output.txt");
  await Promise.all([
    mkdir(binDirectory, { recursive: true }),
    mkdir(distributionLogs, { recursive: true }),
    writeFile(outputFile, "", "utf8"),
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
  printf 'Created bundle at path "%s".\\n' "$FAKE_DISTRIBUTION_LOGS"
  printf 'fixture export\\n'
  exit "$FAKE_EXPORT_STATUS"
fi
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
printf 'checked\n' > "$SYNARA_ARCHIVE_CHECK_MARKER"
`,
    "utf8"
  );
  await chmod(fakeArchiveChecker, 0o755);

  const result = spawnSync(uploadScript, {
    cwd: repositoryRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      PATH: `${binDirectory}:${process.env.PATH}`,
      TMPDIR: root,
      GITHUB_OUTPUT: outputFile,
      FAKE_DISTRIBUTION_LOGS: distributionLogs,
      FAKE_EXPORT_STATUS: String(exportStatus),
      SYNARA_IOS_TEAM_ID: "TEAM",
      SYNARA_IOS_PROVISIONING_PROFILE: "app-profile",
      SYNARA_IOS_NOTIFICATION_SERVICE_PROVISIONING_PROFILE:
        "notification-profile",
      SYNARA_PUSH_GATEWAY_URL:
        "https://push.example.test/_matrix/push/v1/notify",
      SYNARA_IOS_ARCHIVE_ROOT: root,
      SYNARA_IOS_DIAGNOSTICS_DIR: diagnosticsDirectory,
      SYNARA_IOS_NOTIFICATION_ARCHIVE_CHECKER: fakeArchiveChecker,
      SYNARA_EXPECTED_ARCHIVE_PATH: path.join(
        root,
        "Synara-1.2.56-1.2.57.xcarchive"
      ),
      SYNARA_ARCHIVE_CHECK_MARKER: archiveCheckMarker,
    },
  });
  return {
    archiveCheckMarker,
    diagnosticsDirectory,
    distributionLogs,
    outputFile,
    result,
  };
}

test("preserves version outputs, command logs, and Xcode distribution diagnostics", async () => {
  const harness = await createHarness(0);
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
  const harness = await createHarness(17);
  assert.equal(harness.result.status, 17);
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
