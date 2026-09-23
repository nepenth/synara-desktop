import { createServer } from "node:http";
import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { randomUUID, timingSafeEqual } from "node:crypto";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const seed = {
  version: 1,
  updatedAt: null,
  items: [
    {
      id: "sample-1",
      title: "Connect an agent to the status API",
      status: "active",
      priority: 1,
    },
    {
      id: "sample-2",
      title: "Review the next project milestone",
      status: "pending",
      priority: 2,
    },
  ],
};
const files = new Map([
  ["/", ["index.html", "text/html; charset=utf-8"]],
  ["/app.js", ["app.js", "text/javascript; charset=utf-8"]],
  ["/style.css", ["style.css", "text/css; charset=utf-8"]],
]);

const validString = (value, max) =>
  typeof value === "string" && value.trim().length > 0 && value.length <= max;

export function validateSnapshot(value) {
  if (
    !value ||
    typeof value !== "object" ||
    !Array.isArray(value.items) ||
    value.items.length > 100
  ) {
    throw new Error("Expected an items array with at most 100 entries.");
  }
  const seen = new Set();
  const items = value.items.map((item) => {
    const id = typeof item?.id === "string" ? item.id.trim() : "";
    if (
      !item ||
      typeof item !== "object" ||
      !validString(id, 80) ||
      !validString(item.title, 240) ||
      !["pending", "active", "blocked", "done"].includes(item.status) ||
      !Number.isInteger(item.priority) ||
      item.priority < 1 ||
      item.priority > 3 ||
      (item.detail !== undefined &&
        (typeof item.detail !== "string" || item.detail.length > 1000)) ||
      seen.has(id)
    ) {
      throw new Error(
        "Each item needs a unique id, title, status, priority (1–3), and optional detail."
      );
    }
    seen.add(id);
    return {
      id,
      title: item.title.trim(),
      status: item.status,
      priority: item.priority,
      ...(item.detail ? { detail: item.detail } : {}),
    };
  });
  return { version: 1, updatedAt: new Date().toISOString(), items };
}

export async function startServer({
  host = "127.0.0.1",
  port = 8765,
  token,
  dataFile = join(here, "data", "status.json"),
} = {}) {
  if (!validString(token, 4096))
    throw new Error("Set SYNARA_WIDGET_WRITE_TOKEN before starting.");
  let snapshot = seed;
  try {
    snapshot = JSON.parse(await readFile(dataFile, "utf8"));
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }

  const server = createServer(async (request, response) => {
    try {
      const path = new URL(request.url, "http://localhost").pathname;
      response.setHeader("Cache-Control", "no-store");
      response.setHeader("X-Content-Type-Options", "nosniff");
      if (path === "/api/status" && request.method === "GET") {
        response.setHeader("Content-Type", "application/json; charset=utf-8");
        response.end(JSON.stringify(snapshot));
        return;
      }
      if (path === "/api/status" && request.method === "PUT") {
        const supplied =
          request.headers.authorization?.replace(/^Bearer /, "") ?? "";
        const a = Buffer.from(supplied);
        const b = Buffer.from(token);
        if (a.length !== b.length || !timingSafeEqual(a, b)) {
          response.writeHead(401).end("Unauthorized");
          return;
        }
        let body = "";
        for await (const chunk of request) {
          body += chunk;
          if (body.length > 65536) throw new Error("Payload too large.");
        }
        const next = validateSnapshot(JSON.parse(body));
        await mkdir(dirname(dataFile), { recursive: true });
        const temporary = `${dataFile}.${randomUUID()}.tmp`;
        await writeFile(temporary, `${JSON.stringify(next, null, 2)}\n`, {
          mode: 0o600,
        });
        await rename(temporary, dataFile);
        snapshot = next;
        response.setHeader("Content-Type", "application/json; charset=utf-8");
        response.end(JSON.stringify(next));
        return;
      }
      if (request.method === "GET" && files.has(path)) {
        const [name, type] = files.get(path);
        response.setHeader("Content-Type", type);
        response.setHeader(
          "Content-Security-Policy",
          "default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'none'"
        );
        response.end(await readFile(join(here, name)));
        return;
      }
      response.writeHead(404).end("Not found");
    } catch (error) {
      const status =
        error.message === "Payload too large."
          ? 413
          : error instanceof SyntaxError ||
            error.message?.includes("Expected") ||
            error.message?.includes("Each item")
          ? 400
          : 500;
      response
        .writeHead(status)
        .end(status === 500 ? "Internal error" : error.message);
    }
  });
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(port, host, resolve);
  });
  return server;
}

if (
  process.argv[1] &&
  fileURLToPath(import.meta.url) === resolve(process.argv[1])
) {
  startServer({ token: process.env.SYNARA_WIDGET_WRITE_TOKEN })
    .then((server) =>
      console.log(
        `Status widget listening on http://127.0.0.1:${server.address().port}`
      )
    )
    .catch((error) => {
      console.error(error.message);
      process.exitCode = 1;
    });
}
