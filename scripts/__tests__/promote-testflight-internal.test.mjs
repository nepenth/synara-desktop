import assert from "node:assert/strict";
import { generateKeyPairSync, verify } from "node:crypto";
import test from "node:test";

import {
  AppStoreConnectClient,
  AppStoreConnectError,
  createAppStoreConnectToken,
  submitToExternalTestFlightGroups,
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

test("accepts a build that is open to external testing when internal-only is off", async () => {
  const client = createFixtureClient({
    uploadState: "COMPLETE",
    buildStates: [
      {
        processingState: "VALID",
        uploadState: "COMPLETE",
        internalState: "IN_BETA_TESTING",
        assigned: true,
      },
    ],
  });
  const original = client.request.bind(client);
  client.request = async (resourcePath, options) => {
    const document = await original(resourcePath, options);
    if (resourcePath === "/v1/builds/build-1") {
      document.data.attributes.buildAudienceType = "APP_STORE_ELIGIBLE";
    }
    return document;
  };
  await assert.rejects(
    waitForInternalTestFlightAvailability(waitOptions(client)),
    /not restricted to internal testing/
  );
  const snapshot = await waitForInternalTestFlightAvailability(
    waitOptions(client, { internalOnly: false })
  );
  assert.equal(snapshot.internalBuildState, "IN_BETA_TESTING");
});

test("rejects an internal-only build when internal-only is off", async () => {
  const client = createFixtureClient({
    uploadState: "COMPLETE",
    buildStates: [
      {
        processingState: "VALID",
        uploadState: "COMPLETE",
        internalState: "IN_BETA_TESTING",
        assigned: true,
      },
    ],
  });
  await assert.rejects(
    waitForInternalTestFlightAvailability(
      waitOptions(client, { internalOnly: false })
    ),
    /can never reach external testers/
  );
});

const externalGroup = {
  type: "betaGroups",
  id: "group-2",
  attributes: { name: "Friends", isInternalGroup: false },
};

function createExternalFixtureClient({
  group = externalGroup,
  audience = "APP_STORE_ELIGIBLE",
  externalStates,
  autoNotifyEnabled = false,
  submissionError,
}) {
  let buildRequest = 0;
  let assigned = false;
  const calls = [];
  return {
    calls,
    async request(resourcePath, options = {}) {
      if (resourcePath === "/v1/apps") return appDocument;
      if (resourcePath === "/v1/apps/app-1/betaGroups") {
        return { data: [internalGroup, group] };
      }
      if (resourcePath === "/v1/preReleaseVersions") return preReleaseDocument;
      if (resourcePath === "/v1/builds/build-1") {
        const externalState =
          externalStates[Math.min(buildRequest, externalStates.length - 1)];
        buildRequest += 1;
        const document = buildDocument({
          processingState: "VALID",
          uploadState: "COMPLETE",
          internalState: "IN_BETA_TESTING",
        });
        document.data.attributes.buildAudienceType = audience;
        document.data.relationships.betaGroups.data = assigned
          ? [{ type: "betaGroups", id: "group-2" }]
          : [];
        if (assigned) document.included.push(externalGroup);
        Object.assign(
          document.included.find(
            (resource) => resource.type === "buildBetaDetails"
          ).attributes,
          { externalBuildState: externalState, autoNotifyEnabled }
        );
        return document;
      }
      calls.push(`${options.method} ${resourcePath}`);
      if (resourcePath === "/v1/buildBetaDetails/build-1") return {};
      if (resourcePath === "/v1/betaAppReviewSubmissions") {
        if (submissionError) throw submissionError;
        return {};
      }
      if (resourcePath === "/v1/betaGroups/group-2/relationships/builds") {
        assigned = true;
        return {};
      }
      throw new Error(`Unexpected fixture request: ${resourcePath}`);
    },
  };
}

const externalOptions = (client, overrides = {}) => {
  const { internalGroupIds, ...options } = waitOptions(client, overrides);
  return {
    ...options,
    externalGroupIds: ["group-2"],
    timeoutMilliseconds: 5000,
  };
};

test("submits for beta review, then assigns the external group, without waiting for the review", async () => {
  const client = createExternalFixtureClient({
    externalStates: [
      "PROCESSING",
      "READY_FOR_BETA_SUBMISSION",
      "WAITING_FOR_BETA_REVIEW",
    ],
  });
  const snapshot = await submitToExternalTestFlightGroups(
    externalOptions(client)
  );
  assert.deepEqual(client.calls, [
    "PATCH /v1/buildBetaDetails/build-1",
    "POST /v1/betaAppReviewSubmissions",
    "POST /v1/betaGroups/group-2/relationships/builds",
  ]);
  assert.equal(snapshot.externalBuildState, "WAITING_FOR_BETA_REVIEW");
  assert.deepEqual(snapshot.assignedGroups, [
    { id: "group-2", name: "Friends" },
  ]);
});

test("only assigns the group when Apple already has the build in review", async () => {
  const client = createExternalFixtureClient({
    externalStates: ["BETA_APPROVED"],
    autoNotifyEnabled: true,
  });
  await submitToExternalTestFlightGroups(externalOptions(client));
  assert.deepEqual(client.calls, [
    "POST /v1/betaGroups/group-2/relationships/builds",
  ]);
});

test("refuses to send an internal-only build to external testers", async () => {
  const client = createExternalFixtureClient({
    audience: "INTERNAL_ONLY",
    externalStates: ["NOT_APPLICABLE"],
  });
  await assert.rejects(
    submitToExternalTestFlightGroups(externalOptions(client)),
    /internal testing only/
  );
  assert.deepEqual(client.calls, []);
});

test("rejects a configured internal group as an external target", async () => {
  const client = createExternalFixtureClient({
    externalStates: ["READY_FOR_BETA_SUBMISSION"],
  });
  await assert.rejects(
    submitToExternalTestFlightGroups({
      ...externalOptions(client),
      externalGroupIds: ["group-1"],
    }),
    /is not external/
  );
  assert.deepEqual(client.calls, []);
});

test("explains a refused beta review submission and fails on a rejected build", async () => {
  const refused = createExternalFixtureClient({
    externalStates: ["READY_FOR_BETA_SUBMISSION"],
    submissionError: new AppStoreConnectError("Missing beta app description.", {
      status: 422,
    }),
  });
  await assert.rejects(
    submitToExternalTestFlightGroups(externalOptions(refused)),
    /Apple refused the beta review submission: Missing beta app description\. Check the app's TestFlight Test Information/
  );

  const rejected = createExternalFixtureClient({
    externalStates: ["BETA_REJECTED"],
  });
  await assert.rejects(
    submitToExternalTestFlightGroups(externalOptions(rejected)),
    /BETA_REJECTED/
  );
});

test("times out when Apple never moves the build into review", async () => {
  const client = createExternalFixtureClient({
    externalStates: ["READY_FOR_BETA_SUBMISSION"],
  });
  await assert.rejects(
    submitToExternalTestFlightGroups(externalOptions(client)),
    /Timed out waiting.*READY_FOR_BETA_SUBMISSION/
  );
  assert.equal(
    client.calls.filter((call) => call.includes("betaAppReviewSubmissions"))
      .length,
    1
  );
});
