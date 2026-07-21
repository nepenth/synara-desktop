import assert from "node:assert/strict";
import test from "node:test";

import {
  SafeIntegrationError,
  clientOptions,
  computeSharedSecretRegistrationMac,
  parseGeneratedRegistrationSecret,
  parseReceiptModes,
  pollUntil,
  registerDisposableAccount,
  validateLocalHomeserverUrl,
} from "../../synara/scripts/run-synapse-two-client-integration.mjs";

test("enables SDK timeline support for context and latest-window coverage", () => {
  assert.equal(clientOptions("http://127.0.0.1:8008").timelineSupport, true);
});

test("accepts only credential-free HTTP loopback homeserver origins", () => {
  assert.equal(
    validateLocalHomeserverUrl("http://127.0.0.1:8008"),
    "http://127.0.0.1:8008"
  );
  assert.equal(
    validateLocalHomeserverUrl("http://localhost:9008/"),
    "http://localhost:9008"
  );
  assert.equal(
    validateLocalHomeserverUrl("http://[::1]:8008"),
    "http://[::1]:8008"
  );
});

test("rejects production-capable or ambiguous homeserver URLs", () => {
  for (const value of [
    "https://127.0.0.1:8008",
    "http://matrix.example.com",
    "http://0.0.0.0:8008",
    "http://user:password@localhost:8008",
    "http://localhost:8008/_matrix/client/versions",
    "http://localhost:8008?access_token=secret",
    "not-a-url",
  ]) {
    assert.throws(
      () => validateLocalHomeserverUrl(value),
      SafeIntegrationError,
      value
    );
  }
});

test("expands the configured receipt mode deterministically", () => {
  assert.deepEqual(parseReceiptModes("public"), ["public"]);
  assert.deepEqual(parseReceiptModes("private"), ["private"]);
  assert.deepEqual(parseReceiptModes("both"), ["public", "private"]);
  assert.throws(() => parseReceiptModes("disabled"), SafeIntegrationError);
});

test("accepts only one generated registration secret", () => {
  const secret = "a".repeat(64);
  assert.equal(
    parseGeneratedRegistrationSecret(
      `server_name: "localhost"\nregistration_shared_secret: "${secret}"\n`
    ),
    secret
  );
  assert.throws(
    () =>
      parseGeneratedRegistrationSecret(
        "registration_shared_secret: insecure\n"
      ),
    /must contain one generated registration secret/
  );
  assert.throws(
    () =>
      parseGeneratedRegistrationSecret(
        `registration_shared_secret: "${secret}"\nregistration_shared_secret: "${secret}"\n`
      ),
    /must contain one generated registration secret/
  );
});

test("builds Synapse shared-secret registration HMAC exactly", () => {
  assert.equal(
    computeSharedSecretRegistrationMac(
      "a".repeat(64),
      "nonce-nonce-nonce",
      "user",
      "password"
    ),
    "f11c3a5dd9d90867fc084fb39f4d4d23efe30a70"
  );
});

test("registers disposable accounts through the loopback admin endpoint", async () => {
  const requests = [];
  const fetchFn = async (url, options) => {
    requests.push({ url, options });
    if (requests.length === 1) {
      return {
        ok: true,
        status: 200,
        json: async () => ({ nonce: "nonce-nonce-nonce" }),
      };
    }
    return {
      ok: true,
      status: 200,
      json: async () => ({
        user_id: "@reader:localhost",
        device_id: "DEVICE",
        access_token: "token",
      }),
    };
  };

  const response = await registerDisposableAccount(
    "http://127.0.0.1:8008",
    "reader",
    "password",
    { fetchFn, sharedSecret: "a".repeat(64) }
  );
  assert.equal(response.user_id, "@reader:localhost");
  assert.equal(requests.length, 2);
  assert.equal(
    requests[0].url,
    "http://127.0.0.1:8008/_synapse/admin/v1/register"
  );
  assert.equal(requests[0].options.redirect, "error");
  assert.equal(requests[1].options.redirect, "error");
  assert.notEqual(requests[0].options.signal, requests[1].options.signal);
  assert.equal(requests[1].options.method, "POST");
  assert.deepEqual(JSON.parse(requests[1].options.body), {
    nonce: "nonce-nonce-nonce",
    username: "reader",
    password: "password",
    admin: false,
    mac: "2c1b2aded9a5a87896c6eda668f19a0cc291db2f",
  });
});

test("reports an HTTP failure without parsing an untrusted error body", async () => {
  await assert.rejects(
    registerDisposableAccount("http://127.0.0.1:8008", "reader", "password", {
      sharedSecret: "a".repeat(64),
      fetchFn: async () => ({
        ok: false,
        status: 502,
        json: async () => {
          throw new Error("not JSON");
        },
      }),
    }),
    /Registration nonce request failed \(HTTP 502\)/
  );
});

test("bounded polling returns a value and fails closed on timeout", async () => {
  let attempts = 0;
  assert.equal(
    await pollUntil(
      "a fixture",
      () => {
        attempts += 1;
        return attempts === 2 ? "ready" : undefined;
      },
      { timeoutMs: 100, intervalMs: 1 }
    ),
    "ready"
  );

  await assert.rejects(
    pollUntil("a bounded fixture", () => false, {
      timeoutMs: 5,
      intervalMs: 1,
    }),
    /Timed out waiting for a bounded fixture/
  );
});
