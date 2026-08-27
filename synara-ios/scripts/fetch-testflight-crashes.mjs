#!/usr/bin/env node

import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

import { AppStoreConnectClient } from "./promote-testflight-internal.mjs";

const argumentValue = (name) => {
  const index = process.argv.indexOf(name);
  return index === -1 ? undefined : process.argv[index + 1];
};

const requireValue = (name, value) => {
  if (!value) throw new Error(`${name} is required.`);
  return value;
};

const appId = requireValue(
  "--app-id or SYNARA_ASC_APP_ID",
  argumentValue("--app-id") ?? process.env.SYNARA_ASC_APP_ID
);
const buildVersion =
  argumentValue("--build") ?? process.env.SYNARA_IOS_MARKETING_VERSION;
const outputDirectory = path.resolve(
  requireValue(
    "--output or SYNARA_IOS_CRASH_DIAGNOSTICS_DIR",
    argumentValue("--output") ?? process.env.SYNARA_IOS_CRASH_DIAGNOSTICS_DIR
  )
);
const privateKey = await readFile(
  requireValue("SYNARA_ASC_KEY_PATH", process.env.SYNARA_ASC_KEY_PATH),
  "utf8"
);
const client = new AppStoreConnectClient({
  issuerId: requireValue("SYNARA_ASC_ISSUER_ID", process.env.SYNARA_ASC_ISSUER_ID),
  keyId: requireValue("SYNARA_ASC_KEY_ID", process.env.SYNARA_ASC_KEY_ID),
  privateKey,
});

const submissions = await client.request(
  `/v1/apps/${encodeURIComponent(appId)}/betaFeedbackCrashSubmissions`,
  {
    query: {
      limit: 20,
      sort: "-createdDate",
      include: "build",
    },
  }
);
const buildVersionsById = new Map(
  (submissions.included ?? [])
    .filter((resource) => resource.type === "builds")
    .map((resource) => [resource.id, resource.attributes?.version])
);
const selected = (submissions.data ?? []).filter((submission) => {
  if (!buildVersion) return true;
  const buildId = submission.relationships?.build?.data?.id;
  return buildVersionsById.get(buildId) === buildVersion;
});

await mkdir(outputDirectory, { recursive: true });
const index = [];
for (const submission of selected) {
  const crashLog = await client.request(
    `/v1/betaFeedbackCrashSubmissions/${encodeURIComponent(submission.id)}/crashLog`
  );
  const logText = crashLog.data?.attributes?.logText;
  if (typeof logText !== "string" || logText.length === 0) {
    throw new Error(`Crash ${submission.id} did not contain logText.`);
  }
  await writeFile(
    path.join(outputDirectory, `${submission.id}.crash`),
    logText.endsWith("\n") ? logText : `${logText}\n`,
    "utf8"
  );
  const buildId = submission.relationships?.build?.data?.id;
  index.push({
    id: submission.id,
    buildVersion: buildVersionsById.get(buildId) ?? null,
    createdDate: submission.attributes?.createdDate ?? null,
    deviceModel: submission.attributes?.deviceModel ?? null,
    osVersion: submission.attributes?.osVersion ?? null,
    comment: submission.attributes?.comment ?? null,
  });
}
await writeFile(
  path.join(outputDirectory, "index.json"),
  `${JSON.stringify(index, null, 2)}\n`,
  "utf8"
);

console.log(
  JSON.stringify(
    {
      outputDirectory,
      buildVersion: buildVersion ?? "all recent builds",
      crashCount: index.length,
    },
    null,
    2
  )
);
