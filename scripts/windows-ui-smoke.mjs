// Windows UI smoke test: install a built NSIS setup silently, launch Synara,
// and prove the renderer drew the sign-in screen, not just that a process or
// window exists. WebView2 exposes the page over the Chrome DevTools Protocol
// when WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS asks for a debugging port, so the
// script reads the page text there and saves a screenshot of it.
//
// Usage (Windows only):
//   node scripts/windows-ui-smoke.mjs --setup <path to *-setup.exe> --out <dir>
//
// Exit 0 when the sign-in screen rendered. Every other outcome exits 1, with
// whatever screenshot and page text it could capture left in <dir>.
import { execFileSync, spawn } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

export const DEBUG_PORT = 9333;
export const EXPECTED_TEXT = "Welcome back";
const LAUNCH_TIMEOUT_MS = 90_000;
const POLL_MS = 1_000;

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

const parseArgs = (argv) => {
  const args = {};
  for (let index = 0; index < argv.length; index += 2) {
    const key = argv[index];
    const value = argv[index + 1];
    if (!key?.startsWith("--") || value === undefined) {
      throw new Error(`Expected --name value pairs, got: ${argv.join(" ")}`);
    }
    args[key.slice(2)] = value;
  }
  if (!args.setup || !args.out) {
    throw new Error("--setup and --out are required");
  }
  return args;
};

// The page WebView2 reports for the app: the packaged origin, never DevTools
// or an extension page.
export const pickAppTarget = (targets) =>
  (targets || []).find(
    (target) =>
      target.type === "page" &&
      typeof target.webSocketDebuggerUrl === "string" &&
      /^https?:\/\/tauri\.localhost\//.test(target.url || ""),
  );

export const renderedSignIn = (text) =>
  typeof text === "string" && text.includes(EXPECTED_TEXT);

// The NSIS installer installs per user by default; look where it puts the app.
export const installedExeCandidates = (env) =>
  [
    env.LOCALAPPDATA && path.join(env.LOCALAPPDATA, "Synara", "Synara.exe"),
    env.LOCALAPPDATA &&
      path.join(env.LOCALAPPDATA, "Programs", "Synara", "Synara.exe"),
    env.ProgramFiles && path.join(env.ProgramFiles, "Synara", "Synara.exe"),
  ].filter(Boolean);

const cdp = (webSocketUrl) =>
  new Promise((resolve, reject) => {
    const socket = new WebSocket(webSocketUrl);
    let nextId = 0;
    const pending = new Map();
    socket.addEventListener("message", (event) => {
      const message = JSON.parse(event.data);
      const entry = pending.get(message.id);
      if (!entry) return;
      pending.delete(message.id);
      if (message.error) entry.reject(new Error(message.error.message));
      else entry.resolve(message.result);
    });
    socket.addEventListener("error", () =>
      reject(new Error("DevTools socket error")),
    );
    socket.addEventListener("open", () =>
      resolve({
        send: (method, params = {}) =>
          new Promise((res, rej) => {
            nextId += 1;
            pending.set(nextId, { resolve: res, reject: rej });
            socket.send(JSON.stringify({ id: nextId, method, params }));
          }),
        close: () => socket.close(),
      }),
    );
  });

const main = async () => {
  const args = parseArgs(process.argv.slice(2));
  mkdirSync(args.out, { recursive: true });

  console.log(`[windows-ui-smoke] installing ${args.setup}`);
  execFileSync(args.setup, ["/S"], { stdio: "inherit" });
  const exe = installedExeCandidates(process.env).find((candidate) =>
    existsSync(candidate),
  );
  if (!exe) {
    throw new Error(
      `Synara.exe not found after install; looked in ${installedExeCandidates(process.env).join(", ")}`,
    );
  }

  console.log(`[windows-ui-smoke] launching ${exe}`);
  const app = spawn(exe, [], {
    detached: false,
    stdio: "ignore",
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${DEBUG_PORT}`,
    },
  });
  let exited = false;
  app.on("exit", (code) => {
    exited = true;
    console.log(`[windows-ui-smoke] Synara exited with ${code}`);
  });

  let lastText = "";
  let session;
  try {
    const deadline = Date.now() + LAUNCH_TIMEOUT_MS;
    while (Date.now() < deadline && !exited) {
      await sleep(POLL_MS);
      let targets;
      try {
        const response = await fetch(
          `http://127.0.0.1:${DEBUG_PORT}/json/list`,
        );
        targets = await response.json();
      } catch {
        continue; // WebView2 not listening yet.
      }
      const target = pickAppTarget(targets);
      if (!target) continue;
      session ??= await cdp(target.webSocketDebuggerUrl);
      const result = await session.send("Runtime.evaluate", {
        expression: "document.body ? document.body.innerText : ''",
        returnByValue: true,
      });
      lastText = String(result?.result?.value ?? "");
      if (renderedSignIn(lastText)) break;
    }

    if (session) {
      const shot = await session.send("Page.captureScreenshot", {
        format: "png",
      });
      writeFileSync(
        path.join(args.out, "synara-windows.png"),
        Buffer.from(shot.data, "base64"),
      );
      console.log(
        `[windows-ui-smoke] screenshot saved to ${path.join(args.out, "synara-windows.png")}`,
      );
    }
    writeFileSync(path.join(args.out, "page-text.txt"), lastText);

    if (exited)
      throw new Error("Synara exited before the sign-in screen rendered");
    if (!session)
      throw new Error(
        `No Synara page appeared on the DevTools port within ${LAUNCH_TIMEOUT_MS / 1000}s`,
      );
    if (!renderedSignIn(lastText)) {
      throw new Error(
        `The page never showed "${EXPECTED_TEXT}"; its text is in page-text.txt`,
      );
    }
    console.log(
      `[windows-ui-smoke] the sign-in screen rendered ("${EXPECTED_TEXT}").`,
    );
  } finally {
    session?.close();
    if (!exited) app.kill();
  }
};

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  main().then(
    () => process.exit(0),
    (error) => {
      console.error(`[windows-ui-smoke] ${error.message}`);
      process.exit(1);
    },
  );
}
