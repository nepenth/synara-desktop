import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";

import {
  EXPECTED_TEXT,
  installedExeCandidates,
  parseArgs,
} from "../windows-ui-smoke.mjs";

test("looks for the per-user NSIS install first", () => {
  const candidates = installedExeCandidates({
    LOCALAPPDATA: "C:\\Users\\runner\\AppData\\Local",
    ProgramFiles: "C:\\Program Files",
  });
  assert.equal(
    candidates[0],
    path.join("C:\\Users\\runner\\AppData\\Local", "Synara", "Synara.exe"),
  );
  assert.equal(candidates.length, 3);
  assert.deepEqual(installedExeCandidates({}), []);
});

test("requires both the setup and the output directory", () => {
  assert.deepEqual(parseArgs(["--setup", "a.exe", "--out", "dir"]), {
    setup: "a.exe",
    out: "dir",
  });
  assert.throws(() => parseArgs(["--setup", "a.exe"]), /required/);
  assert.throws(() => parseArgs(["--setup"]), /pairs/);
});

test("the expected heading is the sign-in page's", () => {
  const login = readFileSync(
    path.join(process.cwd(), "synara/src/app/pages/auth/login/Login.tsx"),
    "utf8",
  );
  assert.match(login, new RegExp(`>\\s*${EXPECTED_TEXT}\\s*<`));
});
