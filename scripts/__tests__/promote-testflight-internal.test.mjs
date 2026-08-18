import assert from "node:assert/strict";
import { generateKeyPairSync, verify } from "node:crypto";
import test from "node:test";

import {
  AppStoreConnectClient,
  AppStoreConnectError,
  createAppStoreConnectToken,
  upsertBetaBuildLocalization,
  waitForInternalTestFlightAvailability,
} from "../../synara-ios/scripts/promote-testflight-internal.mjs";

const appDocument = {
  data: [{ type: "apps", id: "app-1", attributes: { name: "Synara" } }],
};

const internalGroup = {
  type: "betaGroups",
  id: "group-1",
  attributes: {
    name: "Internal",
    isInternalGroup: true,
    hasAccessToAllBuilds: false,
  },
};

const uploadDocument = (state, errors = []) => ({
  data: [
    {
      type: "buildUploads",
      id: "build-1",
      attributes: {
        cfBundleShortVersionString: "1.2.56",
        cfBundleVersion: "1.2.56",
        platform: "IOS",
        state: { state, errors, warnings: [], infos: [] },
      },
    },
  ],
});

const preReleaseDocument = {
  data: [
    {
      type: "preReleaseVersions",
      id: "prerelease-1",
      attributes: { version: "1.2.56", platform: "IOS" },
      relationships: {
        builds: { data: [{ type: "builds", id: "build-1" }] },
      },
    },
  ],
  included: [
    {
      type: "builds",
      id: "build-1",
      attributes: { version: "1.2.56" },
    },
  ],
};

const buildDocument = ({
  processingState,
  uploadState,
  internalState,
  assigned = false,
  includeBuildUpload = true,
}) => ({
  data: {
    type: "builds",
    id: "build-1",
    attributes: {
      version: "1.2.56",
      processingState,
      buildAudienceType: "INTERNAL_ONLY",
      expired: false,
      usesNonExemptEncryption: false,
    },
    relationships: {
      preReleaseVersion: {
        data: { type: "preReleaseVersions", id: "prerelease-1" },
      },
      buildBetaDetail: {
        data: { type: "buildBetaDetails", id: "build-1" },
      },
      ...(includeBuildUpload
        ? { buildUpload: { data: { type: "buildUploads", id: "build-1" } } }
        : {}),
      betaGroups: {
        data: assigned ? [{ type: "betaGroups", id: "group-1" }] : [],
      },
    },
  },
  included: [
    {
      type: "preReleaseVersions",
      id: "prerelease-1",
      attributes: { version: "1.2.56", platform: "IOS" },
    },
    {
      type: "buildBetaDetails",
      id: "build-1",
      attributes: { internalBuildState: internalState },
    },
    ...(includeBuildUpload
      ? [
          {
            type: "buildUploads",
            id: "build-1",
            attributes: { state: { state: uploadState, errors: [] } },
          },
        ]
      : []),
    ...(assigned ? [internalGroup] : []),
  ],
});

function createFixtureClient({
  group = internalGroup,
  uploadState = "PROCESSING",
  uploadErrors = [],
  uploadResources,
  buildStates = [],
  postErrors = [],
}) {
  let buildRequest = 0;
  const posts = [];
  return {
    posts,
    async request(resourcePath, options = {}) {
      if (resourcePath === "/v1/apps") return appDocument;
      if (resourcePath === "/v1/apps/app-1/betaGroups") {
        return { data: [group] };
      }
      if (resourcePath === "/v1/apps/app-1/buildUploads") {
        return uploadResources
          ? { data: uploadResources }
          : uploadDocument(uploadState, uploadErrors);
      }
      if (resourcePath === "/v1/preReleaseVersions") {
        return buildStates.length > 0 ? preReleaseDocument : { data: [] };
      }
      if (resourcePath === "/v1/builds/build-1") {
        const state =
          buildStates[Math.min(buildRequest, buildStates.length - 1)];
        buildRequest += 1;
        return buildDocument(state);
      }
      if (
        resourcePath === "/v1/betaGroups/group-1/relationships/builds" &&
        options.method === "POST"
      ) {
        posts.push(options.body);
        const error = postErrors.shift();
        if (error) throw error;
        return {};
      }
      throw new Error(`Unexpected fixture request: ${resourcePath}`);
    },
  };
}

const waitOptions = (client, overrides = {}) => {
  let clock = 0;
  return {
    client,
    bundleId: "com.whylandcreative.synara",
    marketingVersion: "1.2.56",
    buildNumber: "1.2.56",
    internalGroupIds: ["group-1"],
    timeoutMilliseconds: 1000,
    pollIntervalMilliseconds: 1000,
    now: () => clock,
    sleepImplementation: async (milliseconds) => {
      clock += milliseconds;
    },
    logger: { log() {} },
    ...overrides,
  };
};

test("creates a standards-compliant short-lived ES256 App Store Connect token", () => {
  const { privateKey, publicKey } = generateKeyPairSync("ec", {
    namedCurve: "P-256",
  });
  const token = createAppStoreConnectToken({
    issuerId: "issuer-id",
    keyId: "key-id",
    privateKey: privateKey.export({ type: "pkcs8", format: "pem" }),
    nowSeconds: 1000,
  });
  const [headerPart, payloadPart, signaturePart] = token.split(".");
  assert.deepEqual(JSON.parse(Buffer.from(headerPart, "base64url")), {
    alg: "ES256",
    kid: "key-id",
    typ: "JWT",
  });
  assert.deepEqual(JSON.parse(Buffer.from(payloadPart, "base64url")), {
    iss: "issuer-id",
    iat: 1000,
    exp: 1600,
    aud: "appstoreconnect-v1",
  });
  assert.equal(
    verify(
      "sha256",
      Buffer.from(`${headerPart}.${payloadPart}`),
      { key: publicKey, dsaEncoding: "ieee-p1363" },
      Buffer.from(signaturePart, "base64url")
    ),
    true
  );
});

test("creates and updates localized TestFlight What's New text", async () => {
  const calls = [];
  const createClient = (existing = []) => ({
    async request(resourcePath, options = {}) {
      calls.push({ resourcePath, options });
      if (resourcePath.endsWith("/betaBuildLocalizations") && !options.method) {
        return { data: existing };
      }
      return { data: { id: existing[0]?.id ?? "localization-new" } };
    },
  });

  await upsertBetaBuildLocalization(createClient(), {
    buildId: "build-1",
    locale: "en-US",
    whatsNew: "  Test secure session restore.  ",
  });
  assert.equal(calls[1].resourcePath, "/v1/betaBuildLocalizations");
  assert.equal(calls[1].options.method, "POST");
  assert.equal(
    calls[1].options.body.data.attributes.whatsNew,
    "Test secure session restore."
  );

  calls.length = 0;
  await upsertBetaBuildLocalization(
    createClient([
      {
        type: "betaBuildLocalizations",
        id: "localization-1",
        attributes: { locale: "en-US" },
      },
    ]),
    { buildId: "build-1", locale: "en-US", whatsNew: "Updated notes" }
  );
  assert.equal(
    calls[1].resourcePath,
    "/v1/betaBuildLocalizations/localization-1"
  );
  assert.equal(calls[1].options.method, "PATCH");
});

test("retries retryable non-JSON responses before parsing a successful response", async () => {
  const { privateKey } = generateKeyPairSync("ec", { namedCurve: "P-256" });
  const responses = [
    { ok: false, status: 503, body: "<html>Unavailable</html>" },
    { ok: true, status: 200, body: '{"data":[]}' },
  ];
  const delays = [];
  const client = new AppStoreConnectClient({
    issuerId: "issuer-id",
    keyId: "key-id",
    privateKey: privateKey.export({ type: "pkcs8", format: "pem" }),
    fetchImplementation: async () => {
      const response = responses.shift();
      return {
        ok: response.ok,
        status: response.status,
        headers: { get: () => null },
        text: async () => response.body,
      };
    },
    sleepImplementation: async (milliseconds) => delays.push(milliseconds),
  });
  assert.deepEqual(await client.request("/v1/apps"), { data: [] });
  assert.deepEqual(delays, [2000]);
});

test("bounds each App Store Connect fetch with an abort signal", async () => {
  const { privateKey } = generateKeyPairSync("ec", { namedCurve: "P-256" });
  let observedSignal;
  const client = new AppStoreConnectClient({
    issuerId: "issuer-id",
    keyId: "key-id",
    privateKey: privateKey.export({ type: "pkcs8", format: "pem" }),
    requestTimeoutMilliseconds: 1234,
    fetchImplementation: async (_url, options) => {
      observedSignal = options.signal;
      return {
        ok: true,
        status: 200,
        headers: { get: () => null },
        text: async () => '{"data":[]}',
      };
    },
  });
  await client.request("/v1/apps");
  assert.equal(observedSignal instanceof AbortSignal, true);
});

test("retries transient network failures", async () => {
  const { privateKey } = generateKeyPairSync("ec", { namedCurve: "P-256" });
  let attempts = 0;
  const client = new AppStoreConnectClient({
    issuerId: "issuer-id",
    keyId: "key-id",
    privateKey: privateKey.export({ type: "pkcs8", format: "pem" }),
    fetchImplementation: async () => {
      attempts += 1;
      if (attempts === 1) throw new Error("connection reset");
      return {
        ok: true,
        status: 200,
        headers: { get: () => null },
        text: async () => '{"data":[]}',
      };
    },
    sleepImplementation: async () => {},
  });
  assert.deepEqual(await client.request("/v1/apps"), { data: [] });
  assert.equal(attempts, 2);
});

test("waits through processing, promotes only a valid build, and verifies beta availability", async () => {
  const client = createFixtureClient({
    uploadState: "COMPLETE",
    buildStates: [
      {
        processingState: "VALID",
        uploadState: "COMPLETE",
        internalState: "READY_FOR_BETA_TESTING",
        assigned: false,
      },
      {
        processingState: "VALID",
        uploadState: "COMPLETE",
        internalState: "IN_BETA_TESTING",
        assigned: true,
      },
    ],
  });
  const result = await waitForInternalTestFlightAvailability(
    waitOptions(client)
  );
  assert.equal(result.build.processingState, "VALID");
  assert.equal(result.internalBuildState, "IN_BETA_TESTING");
  assert.deepEqual(client.posts, [
    { data: [{ type: "builds", id: "build-1" }] },
  ]);
});

test("accepts the upload-list state when the build response omits its upload include", async () => {
  const client = createFixtureClient({
    uploadState: "COMPLETE",
    buildStates: [
      {
        processingState: "VALID",
        uploadState: "COMPLETE",
        internalState: "IN_BETA_TESTING",
        assigned: true,
        includeBuildUpload: false,
      },
    ],
  });
  const result = await waitForInternalTestFlightAvailability(
    waitOptions(client)
  );
  assert.equal(result.upload.state, "COMPLETE");
  assert.equal(result.internalBuildState, "IN_BETA_TESTING");
});

test("ignores an older failed upload when one exact upload is active", async () => {
  const completeUpload = uploadDocument("COMPLETE").data[0];
  const failedUpload = {
    ...uploadDocument("FAILED", [
      { code: "UPLOAD-FAILED", description: "Prior attempt" },
    ]).data[0],
    id: "failed-upload",
  };
  const client = createFixtureClient({
    uploadResources: [failedUpload, completeUpload],
    buildStates: [
      {
        processingState: "VALID",
        uploadState: "COMPLETE",
        internalState: "IN_BETA_TESTING",
        assigned: true,
      },
    ],
  });
  const result = await waitForInternalTestFlightAvailability(
    waitOptions(client)
  );
  assert.equal(result.upload.id, "build-1");
  assert.equal(result.upload.state, "COMPLETE");
});

test("does not let a list-only failed upload override a valid build", async () => {
  const failedUpload = uploadDocument("FAILED", [
    { code: "UPLOAD-FAILED", description: "Historical attempt" },
  ]).data[0];
  const client = createFixtureClient({
    uploadResources: [failedUpload],
    buildStates: [
      {
        processingState: "VALID",
        uploadState: "COMPLETE",
        internalState: "IN_BETA_TESTING",
        assigned: true,
        includeBuildUpload: false,
      },
    ],
  });
  const result = await waitForInternalTestFlightAvailability(
    waitOptions(client)
  );
  assert.equal(result.upload, undefined);
  assert.equal(result.latestFailedUpload.state, "FAILED");
  assert.equal(result.build.processingState, "VALID");
});

test("does not manually assign automatic all-builds internal groups", async () => {
  const client = createFixtureClient({
    group: {
      ...internalGroup,
      attributes: {
        ...internalGroup.attributes,
        hasAccessToAllBuilds: true,
      },
    },
    uploadState: "COMPLETE",
    buildStates: [
      {
        processingState: "VALID",
        uploadState: "COMPLETE",
        internalState: "IN_BETA_TESTING",
        assigned: false,
      },
    ],
  });
  await waitForInternalTestFlightAvailability(waitOptions(client));
  assert.deepEqual(client.posts, []);
});

test("rechecks a manual group after a concurrent assignment conflict", async () => {
  const logs = [];
  const client = createFixtureClient({
    uploadState: "COMPLETE",
    postErrors: [
      new AppStoreConnectError("relationship changed", { status: 409 }),
    ],
    buildStates: [
      {
        processingState: "VALID",
        uploadState: "COMPLETE",
        internalState: "READY_FOR_BETA_TESTING",
        assigned: false,
      },
      {
        processingState: "VALID",
        uploadState: "COMPLETE",
        internalState: "IN_BETA_TESTING",
        assigned: true,
      },
    ],
  });
  await waitForInternalTestFlightAvailability(
    waitOptions(client, { logger: { log: (message) => logs.push(message) } })
  );
  assert.equal(client.posts.length, 1);
  assert.equal(
    logs.some((message) => message.startsWith("Assigned build")),
    false
  );
  assert.equal(
    logs.some((message) => message.includes("raced")),
    true
  );
});

test("surfaces Apple upload errors even before a build resource appears", async () => {
  const client = createFixtureClient({
    uploadState: "FAILED",
    uploadErrors: [{ code: "ITMS-90000", description: "Invalid bundle" }],
  });
  await assert.rejects(
    waitForInternalTestFlightAvailability(
      waitOptions(client, { pollIntervalMilliseconds: 100 })
    ),
    /ITMS-90000.*Invalid bundle/
  );
});

test("times out instead of treating a processing upload as released", async () => {
  const client = createFixtureClient({
    buildStates: [
      {
        processingState: "PROCESSING",
        uploadState: "PROCESSING",
        internalState: "PROCESSING",
      },
    ],
  });
  await assert.rejects(
    waitForInternalTestFlightAvailability(waitOptions(client)),
    /Timed out waiting/
  );
  assert.deepEqual(client.posts, []);
});

test("rejects a configured external beta group before attempting promotion", async () => {
  const client = createFixtureClient({
    group: {
      ...internalGroup,
      attributes: { ...internalGroup.attributes, isInternalGroup: false },
    },
  });
  await assert.rejects(
    waitForInternalTestFlightAvailability(waitOptions(client)),
    /is not internal/
  );
  assert.deepEqual(client.posts, []);
});
