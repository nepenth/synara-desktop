import test from "node:test";
import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { checkWhatsNew, validateWhatsNewNotes } from "../check-whats-new.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");

const valid = () => ({
  version: "3.2.0",
  headline: "Easier settings",
  items: [
    { text: "Settings are grouped by what you want to do.", platforms: ["desktop"] },
    { text: "Rows respond to taps again.", platforms: ["ios"] },
  ],
});

const withRoot = (files, fn) => {
  const dir = mkdtempSync(path.join(os.tmpdir(), "synara-whats-new-"));
  try {
    mkdirSync(path.join(dir, "release-notes"));
    for (const [name, contents] of Object.entries(files)) {
      writeFileSync(
        path.join(dir, "release-notes", name),
        typeof contents === "string" ? contents : JSON.stringify(contents)
      );
    }
    return fn(dir);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
};

test("the repository's notes are valid and the current version has a file", () => {
  const version = JSON.parse(readFileSync(path.join(root, "package.json"), "utf8")).version;
  assert.deepEqual(checkWhatsNew({ root, version }).problems, []);
});

test("a valid file has no problems", () => {
  assert.deepEqual(validateWhatsNewNotes(valid(), "v3.2.0.json"), []);
  // An item can apply to both platforms.
  const shared = valid();
  shared.items = [{ text: "Shows what changed.", platforms: ["desktop", "ios"] }];
  assert.deepEqual(validateWhatsNewNotes(shared, "v3.2.0.json"), []);
});

test("the version must match the file name", () => {
  assert.match(validateWhatsNewNotes(valid(), "v3.2.1.json").join("\n"), /file name says 3\.2\.1/);
  assert.match(validateWhatsNewNotes(valid(), "notes.json").join("\n"), /v<version>\.json/);
});

test("headline and items have length limits", () => {
  const long = valid();
  long.headline = "x".repeat(61);
  assert.match(validateWhatsNewNotes(long, "v3.2.0.json").join("\n"), /headline must be 1–60/);
  long.headline = "";
  assert.match(validateWhatsNewNotes(long, "v3.2.0.json").join("\n"), /headline must be 1–60/);
  const item = valid();
  item.items[0].text = "y".repeat(141);
  assert.match(validateWhatsNewNotes(item, "v3.2.0.json").join("\n"), /item 1 text must be 1–140/);
  item.items[0].text = "y".repeat(140);
  assert.deepEqual(validateWhatsNewNotes(item, "v3.2.0.json"), []);
});

test("items are one line of plain text", () => {
  for (const text of [
    "Use **bold** text",
    "See [the docs](https://example.com)",
    "Run `npm`",
    "- a list item",
    "# A heading",
    "Line one\nline two",
    "An <b>HTML</b> tag",
  ]) {
    const notes = valid();
    notes.items[0].text = text;
    assert.notDeepEqual(validateWhatsNewNotes(notes, "v3.2.0.json"), [], text);
  }
  // Quotes, semicolons and a hyphen inside a sentence are fine.
  const plain = valid();
  plain.items[0].text = 'Search understands "enable gif"; re-open works too.';
  assert.deepEqual(validateWhatsNewNotes(plain, "v3.2.0.json"), []);
});

test("platforms are a non-empty subset of desktop and ios", () => {
  const empty = valid();
  empty.items[0].platforms = [];
  assert.match(validateWhatsNewNotes(empty, "v3.2.0.json").join("\n"), /non-empty/);
  const unknown = valid();
  unknown.items[0].platforms = ["desktop", "android"];
  assert.match(validateWhatsNewNotes(unknown, "v3.2.0.json").join("\n"), /unknown platform "android"/);
  const twice = valid();
  twice.items[0].platforms = ["desktop", "desktop"];
  assert.match(validateWhatsNewNotes(twice, "v3.2.0.json").join("\n"), /twice/);
});

test("each platform has 1–4 items", () => {
  const none = valid();
  none.items = none.items.filter((item) => !item.platforms.includes("ios"));
  assert.match(validateWhatsNewNotes(none, "v3.2.0.json").join("\n"), /at least 1 item for ios/);
  const many = valid();
  for (let index = 0; index < 4; index += 1) {
    many.items.push({ text: `Change ${index}`, platforms: ["desktop"] });
  }
  assert.match(validateWhatsNewNotes(many, "v3.2.0.json").join("\n"), /5 items for desktop; at most 4/);
});

test("unknown keys are rejected", () => {
  const top = { ...valid(), date: "2026-11-18" };
  assert.match(validateWhatsNewNotes(top, "v3.2.0.json").join("\n"), /unknown key "date"/);
  const item = valid();
  item.items[0].link = "https://example.com";
  assert.match(validateWhatsNewNotes(item, "v3.2.0.json").join("\n"), /item 1 has unknown key "link"/);
  const missing = valid();
  delete missing.headline;
  assert.match(validateWhatsNewNotes(missing, "v3.2.0.json").join("\n"), /missing "headline"/);
});

test("the checker requires a file for the current version and reports bad JSON", () => {
  withRoot({ "v3.2.0.json": valid() }, (dir) => {
    assert.deepEqual(checkWhatsNew({ root: dir, version: "3.2.0" }).problems, []);
    const [missing] = checkWhatsNew({ root: dir, version: "3.3.0" }).problems;
    assert.match(missing, /release-notes\/v3\.3\.0\.json: missing/);
  });
  withRoot({ "v3.2.0.json": "{ not json" }, (dir) => {
    const [problem] = checkWhatsNew({ root: dir, version: "3.2.0" }).problems;
    assert.match(problem, /release-notes\/v3\.2\.0\.json: is not valid JSON/);
  });
  withRoot({}, (dir) => {
    assert.equal(checkWhatsNew({ root: dir, version: "3.2.0" }).problems.length, 1);
  });
});
