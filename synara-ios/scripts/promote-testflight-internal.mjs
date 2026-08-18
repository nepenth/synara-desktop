#!/usr/bin/env node

import { createPrivateKey, sign } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const API_BASE_URL = "https://api.appstoreconnect.apple.com";
const RETRYABLE_STATUS_CODES = new Set([408, 429, 500, 502, 503, 504]);
const TERMINAL_BUILD_STATES = new Set(["FAILED", "INVALID"]);
const TERMINAL_INTERNAL_STATES = new Set([
  "EXPIRED",
  "MISSING_EXPORT_COMPLIANCE",
  "PROCESSING_EXCEPTION",
]);

const sleep = (milliseconds) =>
  new Promise((resolve) => setTimeout(resolve, milliseconds));

const encodeBase64Url = (value) => Buffer.from(value).toString("base64url");

const requireValue = (name, value) => {
  if (!value) throw new Error(`${name} is required.`);
  return value;
};

const parsePositiveNumber = (name, value, fallback) => {
  const number = Number(value ?? fallback);
  if (!Number.isFinite(number) || number <= 0) {
    throw new Error(`${name} must be a positive number.`);
  }
  return number;
};

export function createAppStoreConnectToken({
  issuerId,
  keyId,
  privateKey,
  nowSeconds = Math.floor(Date.now() / 1000),
}) {
  const header = encodeBase64Url(
    JSON.stringify({ alg: "ES256", kid: keyId, typ: "JWT" })
  );
  const payload = encodeBase64Url(
    JSON.stringify({
      iss: issuerId,
      iat: nowSeconds,
      exp: nowSeconds + 600,
      aud: "appstoreconnect-v1",
    })
  );
  const unsignedToken = `${header}.${payload}`;
  const signature = sign("sha256", Buffer.from(unsignedToken), {
    key: createPrivateKey(privateKey),
    dsaEncoding: "ieee-p1363",
  });
  return `${unsignedToken}.${signature.toString("base64url")}`;
}

const formatApiErrors = (document, status) => {
  const errors = Array.isArray(document?.errors) ? document.errors : [];
  if (errors.length === 0) return `App Store Connect returned HTTP ${status}.`;
  return errors
    .map((error) =>
      [error.code, error.title, error.detail].filter(Boolean).join(": ")
    )
    .join("; ");
};

export class AppStoreConnectError extends Error {
  constructor(message, { status, cause } = {}) {
    super(message, { cause });
    this.name = "AppStoreConnectError";
    this.status = status;
  }
}

export class AppStoreConnectClient {
  constructor({
    issuerId,
    keyId,
    privateKey,
    fetchImplementation = globalThis.fetch,
    sleepImplementation = sleep,
    now = () => Date.now(),
    requestTimeoutMilliseconds = 30_000,
  }) {
    this.issuerId = issuerId;
    this.keyId = keyId;
    this.privateKey = privateKey;
    this.fetchImplementation = fetchImplementation;
    this.sleepImplementation = sleepImplementation;
    this.now = now;
    this.requestTimeoutMilliseconds = requestTimeoutMilliseconds;
  }

  async request(resourcePath, { method = "GET", query, body } = {}) {
    const url = new URL(resourcePath, API_BASE_URL);
    for (const [key, value] of Object.entries(query ?? {})) {
      if (value !== undefined) url.searchParams.set(key, String(value));
    }

    for (let attempt = 1; attempt <= 4; attempt += 1) {
      const token = createAppStoreConnectToken({
        issuerId: this.issuerId,
        keyId: this.keyId,
        privateKey: this.privateKey,
        nowSeconds: Math.floor(this.now() / 1000),
      });
      let response;
      let responseText;
      try {
        response = await this.fetchImplementation(url, {
          method,
          headers: {
            Authorization: `Bearer ${token}`,
            Accept: "application/json",
            ...(body ? { "Content-Type": "application/json" } : {}),
          },
          signal: AbortSignal.timeout(this.requestTimeoutMilliseconds),
          ...(body ? { body: JSON.stringify(body) } : {}),
        });
        responseText = await response.text();
      } catch (error) {
        if (attempt === 4) {
          throw new AppStoreConnectError(
            "App Store Connect request failed after 4 attempts.",
            { cause: error }
          );
        }
        await this.sleepImplementation(attempt * 2000);
        continue;
      }
      let document = {};
      let parseError;
      if (responseText) {
        try {
          document = JSON.parse(responseText);
        } catch (error) {
          parseError = error;
        }
      }
      if (response.ok) {
        if (parseError) {
          throw new AppStoreConnectError(
            `App Store Connect returned non-JSON HTTP ${response.status}.`,
            { status: response.status, cause: parseError }
          );
        }
        return document;
      }

      if (!RETRYABLE_STATUS_CODES.has(response.status) || attempt === 4) {
        throw new AppStoreConnectError(
          parseError
            ? `App Store Connect returned non-JSON HTTP ${response.status}.`
            : formatApiErrors(document, response.status),
          { status: response.status, cause: parseError }
        );
      }
      const retryAfterHeader = response.headers.get("retry-after");
      const retryAfterSeconds =
        retryAfterHeader === null ? Number.NaN : Number(retryAfterHeader);
      const retryDelay = Number.isFinite(retryAfterSeconds)
        ? retryAfterSeconds * 1000
        : attempt * 2000;
      await this.sleepImplementation(retryDelay);
    }
    throw new Error("App Store Connect request retry loop ended unexpectedly.");
  }
}

const findIncluded = (document, type, id) =>
  (document.included ?? []).find(
    (resource) =>
      resource.type === type && (id === undefined || resource.id === id)
  );

const simplifyMessages = (messages) =>
  (messages ?? []).map((message) => ({
    code: message.code,
    description: message.description,
  }));

const simplifyUpload = (upload) =>
  upload
    ? {
        id: upload.id,
        cfBundleShortVersionString:
          upload.attributes?.cfBundleShortVersionString,
        cfBundleVersion: upload.attributes?.cfBundleVersion,
        createdDate: upload.attributes?.createdDate,
        uploadedDate: upload.attributes?.uploadedDate,
        platform: upload.attributes?.platform,
        state: upload.attributes?.state?.state,
        errors: simplifyMessages(upload.attributes?.state?.errors),
        warnings: simplifyMessages(upload.attributes?.state?.warnings),
        infos: simplifyMessages(upload.attributes?.state?.infos),
      }
    : undefined;

const simplifyBuild = (build) =>
  build
    ? {
        id: build.id,
        version: build.attributes?.version,
        uploadedDate: build.attributes?.uploadedDate,
        processingState: build.attributes?.processingState,
        buildAudienceType: build.attributes?.buildAudienceType,
        expired: build.attributes?.expired,
        usesNonExemptEncryption: build.attributes?.usesNonExemptEncryption,
      }
    : undefined;

async function writeSnapshot(diagnosticsDirectory, snapshot) {
  if (!diagnosticsDirectory) return;
  await mkdir(diagnosticsDirectory, { recursive: true });
  await writeFile(
    path.join(diagnosticsDirectory, "app-store-connect-state.json"),
    `${JSON.stringify(snapshot, null, 2)}\n`,
    "utf8"
  );
}

async function resolveApp(client, bundleId) {
  const document = await client.request("/v1/apps", {
    query: { "filter[bundleId]": bundleId, limit: 2 },
  });
  if (document.data?.length !== 1) {
    throw new Error(
      `Expected one App Store Connect app for ${bundleId}, found ${
        document.data?.length ?? 0
      }.`
    );
  }
  return document.data[0];
}

async function resolveInternalGroups(client, appId, configuredGroupIds) {
  const document = await client.request(`/v1/apps/${appId}/betaGroups`, {
    query: { limit: 200 },
  });
  const groupsById = new Map(
    (document.data ?? []).map((group) => [group.id, group])
  );
  return configuredGroupIds.map((groupId) => {
    const group = groupsById.get(groupId);
    if (!group) {
      throw new Error(
        `Configured TestFlight beta group ${groupId} does not belong to app ${appId}.`
      );
    }
    if (group.attributes?.isInternalGroup !== true) {
      throw new Error(
        `Configured TestFlight beta group ${groupId} is not internal.`
      );
    }
    return group;
  });
}

export async function upsertBetaBuildLocalization(
  client,
  { buildId, locale = "en-US", whatsNew }
) {
  const normalized = whatsNew?.trim();
  if (!normalized) return undefined;
  if (normalized.length > 4000) {
    throw new Error("TestFlight What's New text must not exceed 4000 characters.");
  }

  const document = await client.request(
    `/v1/builds/${buildId}/betaBuildLocalizations`,
    { query: { limit: 200 } }
  );
  const existing = (document.data ?? []).find(
    (localization) => localization.attributes?.locale === locale
  );
  if (existing) {
    await client.request(`/v1/betaBuildLocalizations/${existing.id}`, {
      method: "PATCH",
      body: {
        data: {
          type: "betaBuildLocalizations",
          id: existing.id,
          attributes: { whatsNew: normalized },
        },
      },
    });
    return existing.id;
  }

  const created = await client.request("/v1/betaBuildLocalizations", {
    method: "POST",
    body: {
      data: {
        type: "betaBuildLocalizations",
        attributes: { locale, whatsNew: normalized },
        relationships: {
          build: { data: { type: "builds", id: buildId } },
        },
      },
    },
  });
  return created.data?.id;
}

async function findExactBuildUpload(
  client,
  { appId, marketingVersion, buildNumber }
) {
  const document = await client.request(`/v1/apps/${appId}/buildUploads`, {
    query: {
      "filter[cfBundleShortVersionString]": marketingVersion,
      "filter[cfBundleVersion]": buildNumber,
      "filter[platform]": "IOS",
      include: "build",
      limit: 50,
    },
  });
  const uploads = document.data ?? [];
  const nonFailedUploads = uploads.filter(
    (upload) => upload.attributes?.state?.state !== "FAILED"
  );
  if (nonFailedUploads.length > 1) {
    throw new Error(
      `Found multiple active uploads for Synara ${marketingVersion} (${buildNumber}).`
    );
  }
  const latestFailedUpload = uploads
    .filter((upload) => upload.attributes?.state?.state === "FAILED")
    .toSorted((left, right) =>
      String(
        right.attributes?.uploadedDate ?? right.attributes?.createdDate ?? ""
      ).localeCompare(
        String(
          left.attributes?.uploadedDate ?? left.attributes?.createdDate ?? ""
        )
      )
    )[0];
  return {
    activeUpload: nonFailedUploads[0],
    latestFailedUpload,
  };
}

async function findExactBuild(
  client,
  { appId, marketingVersion, buildNumber }
) {
  const preReleaseDocument = await client.request("/v1/preReleaseVersions", {
    query: {
      "filter[app]": appId,
      "filter[version]": marketingVersion,
      "filter[platform]": "IOS",
      include: "builds",
      limit: 10,
      "limit[builds]": 50,
    },
  });
  if ((preReleaseDocument.data?.length ?? 0) > 1) {
    throw new Error(
      `Found multiple iOS prerelease records for ${marketingVersion}.`
    );
  }
  const preRelease = preReleaseDocument.data?.[0];
  if (!preRelease) return undefined;

  const relatedBuildIds = new Set(
    (preRelease.relationships?.builds?.data ?? []).map(
      (resource) => resource.id
    )
  );
  const candidates = (preReleaseDocument.included ?? []).filter(
    (resource) =>
      resource.type === "builds" &&
      relatedBuildIds.has(resource.id) &&
      resource.attributes?.version === buildNumber
  );
  if (candidates.length > 1) {
    throw new Error(
      `Found multiple builds for Synara ${marketingVersion} (${buildNumber}).`
    );
  }
  if (candidates.length === 0) return undefined;

  const document = await client.request(`/v1/builds/${candidates[0].id}`, {
    query: {
      include: "preReleaseVersion,betaGroups,buildBetaDetail,buildUpload",
    },
  });
  const build = document.data;
  const linkedPreRelease = findIncluded(
    document,
    "preReleaseVersions",
    build.relationships?.preReleaseVersion?.data?.id
  );
  if (
    linkedPreRelease?.attributes?.version !== marketingVersion ||
    linkedPreRelease?.attributes?.platform !== "IOS" ||
    build.attributes?.version !== buildNumber
  ) {
    throw new Error(
      "App Store Connect returned a build that did not match the release."
    );
  }
  return {
    build,
    preRelease: linkedPreRelease,
    betaDetail: findIncluded(
      document,
      "buildBetaDetails",
      build.relationships?.buildBetaDetail?.data?.id
    ),
    buildUpload: findIncluded(
      document,
      "buildUploads",
      build.relationships?.buildUpload?.data?.id
    ),
    betaGroups: (build.relationships?.betaGroups?.data ?? []).map(
      (resource) =>
        findIncluded(document, "betaGroups", resource.id) ?? resource
    ),
  };
}

const describeState = ({ upload, exactBuild }) =>
  [
    `upload=${upload?.attributes?.state?.state ?? "NOT_FOUND"}`,
    `build=${exactBuild?.build?.attributes?.processingState ?? "NOT_FOUND"}`,
    `internal=${
      exactBuild?.betaDetail?.attributes?.internalBuildState ?? "NOT_FOUND"
    }`,
    `groups=${exactBuild?.betaGroups?.length ?? 0}`,
  ].join(" ");

export async function waitForInternalTestFlightAvailability({
  client,
  bundleId,
  marketingVersion,
  buildNumber,
  internalGroupIds,
  whatsNew,
  whatsNewLocale = "en-US",
  diagnosticsDirectory,
  timeoutMilliseconds = 2_400_000,
  pollIntervalMilliseconds = 15_000,
  now = () => Date.now(),
  sleepImplementation = sleep,
  logger = console,
}) {
  const app = await resolveApp(client, bundleId);
  const groups = await resolveInternalGroups(client, app.id, internalGroupIds);
  const deadline = now() + timeoutMilliseconds;
  let lastDescription;
  let promotionAttempted = false;
  let localizationPublished = false;
  let failedUploadObservations = 0;

  while (now() <= deadline) {
    const { activeUpload, latestFailedUpload } = await findExactBuildUpload(
      client,
      {
        appId: app.id,
        marketingVersion,
        buildNumber,
      }
    );
    const exactBuild = await findExactBuild(client, {
      appId: app.id,
      marketingVersion,
      buildNumber,
    });
    const effectiveUpload = exactBuild?.buildUpload ?? activeUpload;
    const description = describeState({ upload: effectiveUpload, exactBuild });
    if (description !== lastDescription) {
      logger.log(
        `TestFlight ${marketingVersion} (${buildNumber}): ${description}`
      );
      lastDescription = description;
    }

    const snapshot = {
      queriedAt: new Date(now()).toISOString(),
      expected: { bundleId, marketingVersion, buildNumber },
      app: { id: app.id, name: app.attributes?.name },
      targetGroups: groups.map((group) => ({
        id: group.id,
        name: group.attributes?.name,
        isInternalGroup: group.attributes?.isInternalGroup,
        hasAccessToAllBuilds: group.attributes?.hasAccessToAllBuilds,
      })),
      upload: simplifyUpload(effectiveUpload),
      latestFailedUpload: simplifyUpload(latestFailedUpload),
      build: simplifyBuild(exactBuild?.build),
      internalBuildState:
        exactBuild?.betaDetail?.attributes?.internalBuildState,
      assignedGroups: (exactBuild?.betaGroups ?? []).map((group) => ({
        id: group.id,
        name: group.attributes?.name,
      })),
    };
    await writeSnapshot(diagnosticsDirectory, snapshot);

    const uploadState = effectiveUpload?.attributes?.state;
    if (
      (uploadState?.errors?.length ?? 0) > 0 ||
      uploadState?.state === "FAILED"
    ) {
      throw new Error(
        `Apple failed the build upload: ${JSON.stringify(
          simplifyMessages(uploadState?.errors)
        )}`
      );
    }

    if (!exactBuild && !activeUpload && latestFailedUpload) {
      failedUploadObservations += 1;
      if (failedUploadObservations >= 5) {
        throw new Error(
          `Apple failed the build upload: ${JSON.stringify(
            simplifyMessages(latestFailedUpload.attributes?.state?.errors)
          )}`
        );
      }
    } else {
      failedUploadObservations = 0;
    }

    const processingState = exactBuild?.build?.attributes?.processingState;
    if (TERMINAL_BUILD_STATES.has(processingState)) {
      throw new Error(`Apple marked the build ${processingState}.`);
    }
    if (processingState === "VALID") {
      if (exactBuild.build.attributes?.expired === true) {
        throw new Error("Apple marked the uploaded build expired.");
      }
      if (exactBuild.build.attributes?.buildAudienceType !== "INTERNAL_ONLY") {
        throw new Error(
          "The uploaded build is not restricted to internal testing."
        );
      }
      if (exactBuild.build.attributes?.usesNonExemptEncryption !== false) {
        throw new Error("The uploaded build has unresolved export compliance.");
      }

      if (whatsNew && !localizationPublished) {
        await upsertBetaBuildLocalization(client, {
          buildId: exactBuild.build.id,
          locale: whatsNewLocale,
          whatsNew,
        });
        localizationPublished = true;
        logger.log(`Published ${whatsNewLocale} TestFlight release notes.`);
      }

      const assignedGroupIds = new Set(
        exactBuild.betaGroups.map((group) => group.id)
      );
      for (const group of groups) {
        if (group.attributes?.hasAccessToAllBuilds === true) {
          assignedGroupIds.add(group.id);
        }
      }
      const missingGroups = groups.filter(
        (group) => !assignedGroupIds.has(group.id)
      );
      if (missingGroups.length > 0 && !promotionAttempted) {
        for (const group of missingGroups) {
          try {
            await client.request(
              `/v1/betaGroups/${group.id}/relationships/builds`,
              {
                method: "POST",
                body: { data: [{ type: "builds", id: exactBuild.build.id }] },
              }
            );
            logger.log(
              `Assigned build ${exactBuild.build.id} to ${group.attributes?.name}.`
            );
          } catch (error) {
            if (
              !(error instanceof AppStoreConnectError && error.status === 409)
            ) {
              throw error;
            }
            logger.log(
              `Assignment for build ${exactBuild.build.id} raced with another update; rechecking ${group.attributes?.name}.`
            );
          }
        }
        promotionAttempted = true;
      }

      const internalState =
        exactBuild.betaDetail?.attributes?.internalBuildState;
      if (TERMINAL_INTERNAL_STATES.has(internalState)) {
        throw new Error(
          `Apple marked the internal beta build ${internalState}.`
        );
      }
      const allGroupsAssigned = groups.every((group) =>
        assignedGroupIds.has(group.id)
      );
      if (
        (effectiveUpload?.attributes?.state?.state === "COMPLETE" ||
          processingState === "VALID") &&
        allGroupsAssigned &&
        internalState === "IN_BETA_TESTING"
      ) {
        logger.log(
          `Synara ${marketingVersion} (${buildNumber}) is available to internal TestFlight testers.`
        );
        return snapshot;
      }
    }

    await sleepImplementation(pollIntervalMilliseconds);
  }

  throw new Error(
    `Timed out waiting for Synara ${marketingVersion} (${buildNumber}) to become available in internal TestFlight.`
  );
}

export async function main(environment = process.env) {
  const privateKeyPath = requireValue(
    "SYNARA_ASC_KEY_PATH",
    environment.SYNARA_ASC_KEY_PATH
  );
  const groupIds = [
    ...new Set(
      requireValue(
        "SYNARA_TESTFLIGHT_INTERNAL_GROUP_IDS",
        environment.SYNARA_TESTFLIGHT_INTERNAL_GROUP_IDS
      )
        .split(",")
        .map((value) => value.trim())
        .filter(Boolean)
    ),
  ];
  if (groupIds.length === 0) {
    throw new Error("SYNARA_TESTFLIGHT_INTERNAL_GROUP_IDS must not be empty.");
  }

  const client = new AppStoreConnectClient({
    issuerId: requireValue(
      "SYNARA_ASC_ISSUER_ID",
      environment.SYNARA_ASC_ISSUER_ID
    ),
    keyId: requireValue("SYNARA_ASC_KEY_ID", environment.SYNARA_ASC_KEY_ID),
    privateKey: await readFile(privateKeyPath, "utf8"),
  });
  const whatsNew = environment.SYNARA_TESTFLIGHT_WHATS_NEW_PATH
    ? await readFile(environment.SYNARA_TESTFLIGHT_WHATS_NEW_PATH, "utf8")
    : undefined;
  return waitForInternalTestFlightAvailability({
    client,
    bundleId: environment.SYNARA_IOS_BUNDLE_ID ?? "com.whylandcreative.synara",
    marketingVersion: requireValue(
      "SYNARA_IOS_MARKETING_VERSION",
      environment.SYNARA_IOS_MARKETING_VERSION
    ),
    buildNumber: requireValue(
      "SYNARA_IOS_BUILD_NUMBER",
      environment.SYNARA_IOS_BUILD_NUMBER
    ),
    internalGroupIds: groupIds,
    whatsNew,
    whatsNewLocale: environment.SYNARA_TESTFLIGHT_WHATS_NEW_LOCALE ?? "en-US",
    diagnosticsDirectory: environment.SYNARA_IOS_DIAGNOSTICS_DIR,
    timeoutMilliseconds:
      parsePositiveNumber(
        "SYNARA_TESTFLIGHT_PROCESSING_TIMEOUT_SECONDS",
        environment.SYNARA_TESTFLIGHT_PROCESSING_TIMEOUT_SECONDS,
        2400
      ) * 1000,
    pollIntervalMilliseconds:
      parsePositiveNumber(
        "SYNARA_TESTFLIGHT_POLL_INTERVAL_SECONDS",
        environment.SYNARA_TESTFLIGHT_POLL_INTERVAL_SECONDS,
        15
      ) * 1000,
  });
}

const invokedPath = process.argv[1] ? path.resolve(process.argv[1]) : undefined;
if (invokedPath === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  });
}
