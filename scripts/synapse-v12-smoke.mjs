#!/usr/bin/env node
// Run only against a disposable Synapse 1.162+ server bound to loopback.
import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import { pathToFileURL } from "node:url";

export function disposableBaseUrl(value) {
  const url = new URL(value);
  assert.equal(url.protocol, "http:", "The v12 smoke server must use local HTTP");
  assert.ok(
    ["127.0.0.1", "localhost", "[::1]"].includes(url.hostname),
    "The v12 smoke server must be bound to loopback",
  );
  assert.equal(url.pathname, "/", "The v12 smoke URL must have no path");
  assert.equal(url.search, "", "The v12 smoke URL must have no query");
  assert.equal(url.hash, "", "The v12 smoke URL must have no fragment");
  assert.equal(url.username, "", "Do not put credentials in the server URL");
  assert.equal(url.password, "", "Do not put credentials in the server URL");
  return url;
}

function segment(value) {
  return encodeURIComponent(value);
}

export async function runV12Smoke({ baseUrl, aliceToken, bobToken, bobUserId, fetchImpl = fetch }) {
  const base = disposableBaseUrl(baseUrl);
  assert.ok(aliceToken && bobToken && bobUserId, "Two disposable access tokens and Bob's user ID are required");
  assert.match(bobUserId, /^@[^:]+:[^:]+$/, "Bob's Matrix user ID is malformed");

  async function request(token, method, path, body) {
    const url = new URL("/_matrix/client/v3/" + path, base);
    const response = await fetchImpl(url, {
      method,
      headers: {
        authorization: "Bearer " + token,
        ...(body === undefined ? {} : { "content-type": "application/json" }),
      },
      ...(body === undefined ? {} : { body: JSON.stringify(body) }),
      signal: AbortSignal.timeout(15_000),
    });
    const payload = await response.json().catch(() => ({}));
    if (!response.ok) {
      throw new Error("Synapse v12 smoke request failed: HTTP " + response.status + " " + (payload.errcode ?? ""));
    }
    return payload;
  }

  const roomPath = (roomId, suffix) => "rooms/" + segment(roomId) + "/" + suffix;
  const createRoom = async (body) => {
    const result = await request(aliceToken, "POST", "createRoom", body);
    assert.match(result.room_id ?? "", /^!.+:.+$/, "Synapse returned no room ID");
    return result.room_id;
  };
  const state = (token, roomId, type) => request(token, "GET", roomPath(roomId, "state/" + type));

  const defaultRoomId = await createRoom({ preset: "private_chat", name: "Synara v12 default smoke" });
  const defaultCreate = await state(aliceToken, defaultRoomId, "m.room.create");
  assert.ok(defaultCreate.room_version, "Default room has no version");

  const roomId = await createRoom({
    room_version: "12",
    preset: "private_chat",
    name: "Synara v12 event smoke",
  });
  const create = await state(aliceToken, roomId, "m.room.create");
  assert.equal(create.room_version, "12", "Explicit version-12 creation was not honored");

  await request(aliceToken, "POST", roomPath(roomId, "invite"), { user_id: bobUserId });
  await request(bobToken, "POST", "join/" + segment(roomId), {});
  await state(bobToken, roomId, "m.room.join_rules");
  await state(bobToken, roomId, "m.room.power_levels");

  const event = await request(
    aliceToken,
    "PUT",
    roomPath(roomId, "send/m.room.message/" + randomUUID()),
    { msgtype: "m.text", body: "Synara disposable v12 smoke" },
  );
  assert.ok(event.event_id, "Message send returned no event ID");
  const eventPath = roomPath(roomId, "event/" + segment(event.event_id));
  const readback = await request(bobToken, "GET", eventPath);
  assert.equal(readback.content?.body, "Synara disposable v12 smoke");

  const reaction = await request(
    bobToken,
    "PUT",
    roomPath(roomId, "send/m.reaction/" + randomUUID()),
    { "m.relates_to": { rel_type: "m.annotation", event_id: event.event_id, key: "✅" } },
  );
  const relations = await request(
    aliceToken,
    "GET",
    roomPath(roomId, "relations/" + segment(event.event_id) + "/m.annotation/m.reaction"),
  );
  assert.ok(relations.chunk?.some((item) => item.event_id === reaction.event_id), "Reaction missing from /relations");

  await request(
    aliceToken,
    "PUT",
    roomPath(roomId, "redact/" + segment(event.event_id) + "/" + randomUUID()),
    { reason: "Disposable v12 smoke" },
  );
  let redacted = false;
  for (let attempt = 0; attempt < 10; attempt += 1) {
    const visible = await request(bobToken, "GET", eventPath);
    if (visible.content?.body === undefined) {
      redacted = true;
      break;
    }
    await new Promise((resolve) => setTimeout(resolve, 300));
  }
  assert.ok(redacted, "Bob still sees the redacted message body");

  const encryptedRoomId = await createRoom({
    room_version: "12",
    preset: "private_chat",
    name: "Synara v12 encrypted smoke",
    initial_state: [{ type: "m.room.encryption", content: { algorithm: "m.megolm.v1.aes-sha2" } }],
  });
  assert.equal((await state(aliceToken, encryptedRoomId, "m.room.create")).room_version, "12");
  assert.equal((await state(aliceToken, encryptedRoomId, "m.room.encryption")).algorithm, "m.megolm.v1.aes-sha2");
  await request(aliceToken, "POST", roomPath(encryptedRoomId, "invite"), { user_id: bobUserId });
  await request(bobToken, "POST", "join/" + segment(encryptedRoomId), {});

  return { defaultRoomVersion: defaultCreate.room_version, roomId, encryptedRoomId };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  runV12Smoke({
    baseUrl: process.env.SYNARA_V12_BASE_URL ?? "http://127.0.0.1:8008",
    aliceToken: process.env.SYNARA_V12_ALICE_TOKEN,
    bobToken: process.env.SYNARA_V12_BOB_TOKEN,
    bobUserId: process.env.SYNARA_V12_BOB_USER_ID,
  }).then(
    ({ defaultRoomVersion, roomId, encryptedRoomId }) => {
      process.stdout.write("v12 event smoke passed; default room version: " + defaultRoomVersion + "\n");
      process.stdout.write("Event room: " + roomId + "; encrypted room: " + encryptedRoomId + "\n");
      process.stdout.write("Finish the encrypted send/readback in both Synara clients.\n");
    },
    (error) => {
      process.stderr.write(error.message + "\n");
      process.exitCode = 1;
    },
  );
}
