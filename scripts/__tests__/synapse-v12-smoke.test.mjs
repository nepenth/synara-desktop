import assert from "node:assert/strict";
import test from "node:test";
import { disposableBaseUrl, runV12Smoke } from "../synapse-v12-smoke.mjs";

test("v12 smoke refuses non-disposable server URLs", () => {
  for (const value of [
    "https://127.0.0.1:8008",
    "http://matrix.example.org:8008",
    "http://127.0.0.1:8008/admin",
    "http://user:secret@127.0.0.1:8008/",
  ]) {
    assert.throws(() => disposableBaseUrl(value));
  }
  assert.equal(disposableBaseUrl("http://127.0.0.1:8008").origin, "http://127.0.0.1:8008");
});

test("v12 smoke checks default and explicit rooms, two users, relations, redaction, and encryption", async () => {
  const calls = [];
  const roomVersions = new Map();
  let redacted = false;
  const json = (value) => new Response(JSON.stringify(value), {
    status: 200,
    headers: { "content-type": "application/json" },
  });
  const fakeFetch = async (url, options) => {
    const path = decodeURIComponent(url.pathname);
    const body = options.body ? JSON.parse(options.body) : undefined;
    calls.push({ path, method: options.method, token: options.headers.authorization, body });
    if (path.endsWith("/createRoom")) {
      const roomId = "!room" + (roomVersions.size + 1) + ":localhost";
      roomVersions.set(roomId, {
        version: body.room_version ?? "12",
        encrypted: Boolean(body.initial_state),
      });
      return json({ room_id: roomId });
    }
    const state = path.match(/\/rooms\/([^/]+)\/state\/(.+)$/);
    if (state) {
      const room = roomVersions.get(state[1]);
      if (state[2] === "m.room.create") return json({ room_version: room.version });
      if (state[2] === "m.room.encryption") {
        return json({ algorithm: room.encrypted ? "m.megolm.v1.aes-sha2" : undefined });
      }
      return json({});
    }
    if (path.includes("/send/m.room.message/")) return json({ event_id: "$message:localhost" });
    if (path.includes("/send/m.reaction/")) return json({ event_id: "$reaction:localhost" });
    if (path.includes("/relations/")) return json({ chunk: [{ event_id: "$reaction:localhost" }] });
    if (path.includes("/redact/")) {
      redacted = true;
      return json({ event_id: "$redaction:localhost" });
    }
    if (path.includes("/event/")) {
      return json({ content: redacted ? {} : { body: "Synara disposable v12 smoke" } });
    }
    return json({});
  };

  const result = await runV12Smoke({
    baseUrl: "http://127.0.0.1:8008",
    aliceToken: "alice-disposable-token",
    bobToken: "bob-disposable-token",
    bobUserId: "@bob:localhost",
    fetchImpl: fakeFetch,
  });
  assert.equal(result.defaultRoomVersion, "12");
  assert.equal(result.roomId, "!room2:localhost");
  assert.equal(result.encryptedRoomId, "!room3:localhost");
  assert.ok(calls.some((call) => call.path.includes("/join/") && call.token === "Bearer bob-disposable-token"));
  assert.ok(calls.some((call) => call.path.includes("/relations/") && call.token === "Bearer alice-disposable-token"));
  assert.ok(calls.some((call) => call.path.includes("/redact/") && call.token === "Bearer alice-disposable-token"));
  assert.equal(redacted, true);
});
