import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { startServer } from "./server.mjs";

test("agent writes require a token and valid snapshot, then persist for the widget", async () => {
  const directory = await mkdtemp(join(tmpdir(), "synara-widget-"));
  const dataFile = join(directory, "status.json");
  const server = await startServer({ port: 0, token: "test-token", dataFile });
  const url = `http://127.0.0.1:${server.address().port}/api/status`;
  try {
    const unauthorized = await fetch(url, {
      method: "PUT",
      body: '{"items":[]}',
    });
    assert.equal(unauthorized.status, 401);
    const invalid = await fetch(url, {
      method: "PUT",
      headers: { Authorization: "Bearer test-token" },
      body: JSON.stringify({ items: [{ id: "a", title: "Missing status" }] }),
    });
    assert.equal(invalid.status, 400);
    const valid = {
      items: [
        {
          id: "a",
          title: "<script>not HTML</script>",
          status: "active",
          priority: 1,
        },
      ],
    };
    const accepted = await fetch(url, {
      method: "PUT",
      headers: { Authorization: "Bearer test-token" },
      body: JSON.stringify(valid),
    });
    assert.equal(accepted.status, 200);
    const snapshot = await (await fetch(url)).json();
    assert.equal(snapshot.items[0].title, valid.items[0].title);
    assert.equal(JSON.parse(await readFile(dataFile, "utf8")).items[0].id, "a");
  } finally {
    await new Promise((resolve) => server.close(resolve));
    await rm(directory, { recursive: true, force: true });
  }
});
