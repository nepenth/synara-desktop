import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";

import {
  installedExeCandidates,
  pickAppTarget,
  renderedSignIn,
} from "../windows-ui-smoke.mjs";

test("picks the app page, not DevTools or other targets", () => {
  const app = {
    type: "page",
    url: "http://tauri.localhost/login",
    webSocketDebuggerUrl: "ws://127.0.0.1:9333/devtools/page/A",
  };
  assert.equal(
    pickAppTarget([
      { type: "page", url: "devtools://devtools/bundled/inspector.html", webSocketDebuggerUrl: "ws://x" },
      { type: "service_worker", url: "http://tauri.localhost/sw.js", webSocketDebuggerUrl: "ws://y" },
      app,
    ]),
    app
  );
  assert.equal(pickAppTarget([{ type: "page", url: "about:blank", webSocketDebuggerUrl: "ws://z" }]), undefined);
  assert.equal(pickAppTarget(undefined), undefined);
});

test("only the sign-in heading counts as a rendered UI", () => {
  assert.equal(renderedSignIn("Synara\nWelcome back\nSign in to your account on matrix.org."), true);
  assert.equal(renderedSignIn(""), false);
  assert.equal(renderedSignIn(undefined), false);
});

test("looks for the per-user NSIS install first", () => {
  const candidates = installedExeCandidates({
    LOCALAPPDATA: "C:\\Users\\runner\\AppData\\Local",
    ProgramFiles: "C:\\Program Files",
  });
  assert.equal(candidates[0], path.join("C:\\Users\\runner\\AppData\\Local", "Synara", "Synara.exe"));
  assert.equal(candidates.length, 3);
  assert.deepEqual(installedExeCandidates({}), []);
});
