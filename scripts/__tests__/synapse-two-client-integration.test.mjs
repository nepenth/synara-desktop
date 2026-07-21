import assert from "node:assert/strict";
import test from "node:test";

import {
  SafeIntegrationError,
  parseReceiptModes,
  pollUntil,
  validateLocalHomeserverUrl,
} from "../../synara/scripts/run-synapse-two-client-integration.mjs";

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
