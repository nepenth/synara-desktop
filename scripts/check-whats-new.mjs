// Validates the per-release "What's new" notes in release-notes/v<version>.json
// and requires one for the current package.json version. The desktop app
// bundles the file for its own version (synara/src/app/features/whats-new/),
// the iOS app reads the same files, and the release workflow writes the
// headline and desktop items into the updater's latest.json notes.
// See docs/whats-new-notes.md for how to write them.
import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

export const NOTES_DIR = "release-notes";
export const PLATFORMS = ["desktop", "ios"];
export const MAX_HEADLINE = 60;
export const MAX_ITEM = 140;
export const MAX_ITEMS_PER_PLATFORM = 4;

const TOP_LEVEL_KEYS = ["version", "headline", "items"];
const ITEM_KEYS = ["text", "platforms"];
const FILE_PATTERN = /^v(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)\.json$/;

/** Markdown or HTML that would show up literally in the app and the updater. */
const MARKUP = [
  [/`/, "a backtick"],
  [/\*\*|__|~~/, "emphasis markers"],
  [/\]\(/, "a Markdown link"],
  [/<\/?[A-Za-z]/, "an HTML tag"],
  [/^\s*(?:[#>*+-]|\d+[.)])\s/, "a Markdown heading, quote or list marker"],
];

const characters = (text) => [...text].length;

const plainTextProblems = (label, text, max) => {
  if (typeof text !== "string") return [`${label} must be a string`];
  const problems = [];
  const length = characters(text);
  if (length < 1 || length > max) problems.push(`${label} must be 1–${max} characters (is ${length})`);
  if (text.trim() !== text) problems.push(`${label} has leading or trailing spaces`);
  if (/[\r\n\t]/.test(text)) problems.push(`${label} must be one line`);
  for (const [pattern, what] of MARKUP) {
    if (pattern.test(text)) problems.push(`${label} must be plain text, not ${what}`);
  }
  return problems;
};

const unknownKeys = (label, value, allowed) =>
  Object.keys(value)
    .filter((key) => !allowed.includes(key))
    .map((key) => `${label} has unknown key "${key}"`);

/**
 * Problems with one notes file's parsed contents; empty when it is valid.
 * `fileName` is the base name, `v<version>.json`.
 */
export const validateWhatsNewNotes = (notes, fileName) => {
  const match = FILE_PATTERN.exec(fileName);
  if (!match) return [`file name must be v<version>.json, like v3.2.0.json`];
  if (!notes || typeof notes !== "object" || Array.isArray(notes)) return ["must be a JSON object"];

  const problems = [...unknownKeys("the file", notes, TOP_LEVEL_KEYS)];
  for (const key of TOP_LEVEL_KEYS) {
    if (!(key in notes)) problems.push(`missing "${key}"`);
  }
  if ("version" in notes && notes.version !== match[1]) {
    problems.push(`"version" is ${JSON.stringify(notes.version)} but the file name says ${match[1]}`);
  }
  if ("headline" in notes) problems.push(...plainTextProblems("headline", notes.headline, MAX_HEADLINE));
  if (!("items" in notes)) return problems;
  if (!Array.isArray(notes.items)) return [...problems, '"items" must be an array'];

  const perPlatform = Object.fromEntries(PLATFORMS.map((platform) => [platform, 0]));
  notes.items.forEach((item, index) => {
    const label = `item ${index + 1}`;
    if (!item || typeof item !== "object" || Array.isArray(item)) {
      problems.push(`${label} must be an object with "text" and "platforms"`);
      return;
    }
    problems.push(...unknownKeys(label, item, ITEM_KEYS));
    problems.push(...plainTextProblems(`${label} text`, item.text, MAX_ITEM));
    const { platforms } = item;
    if (!Array.isArray(platforms) || platforms.length === 0) {
      problems.push(`${label} "platforms" must be a non-empty list of ${PLATFORMS.join(", ")}`);
      return;
    }
    if (new Set(platforms).size !== platforms.length) problems.push(`${label} lists a platform twice`);
    for (const platform of platforms) {
      if (!PLATFORMS.includes(platform)) {
        problems.push(`${label} has unknown platform ${JSON.stringify(platform)}`);
      } else {
        perPlatform[platform] += 1;
      }
    }
  });
  for (const platform of PLATFORMS) {
    const count = perPlatform[platform];
    if (count < 1) problems.push(`needs at least 1 item for ${platform}`);
    if (count > MAX_ITEMS_PER_PLATFORM) {
      problems.push(`has ${count} items for ${platform}; at most ${MAX_ITEMS_PER_PLATFORM}`);
    }
  }
  return problems;
};

/**
 * Checks every notes file under `root/release-notes` and that `version` has
 * one. Returns the problems as "path: problem" lines.
 */
export const checkWhatsNew = ({ root, version }) => {
  const directory = path.join(root, NOTES_DIR);
  let names = [];
  try {
    names = readdirSync(directory).filter((name) => name.endsWith(".json")).sort();
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
  }
  const problems = [];
  for (const name of names) {
    const relativePath = `${NOTES_DIR}/${name}`;
    let notes;
    try {
      notes = JSON.parse(readFileSync(path.join(directory, name), "utf8"));
    } catch (error) {
      problems.push(`${relativePath}: is not valid JSON (${error.message})`);
      continue;
    }
    for (const problem of validateWhatsNewNotes(notes, name)) problems.push(`${relativePath}: ${problem}`);
  }
  if (version && !names.includes(`v${version}.json`)) {
    problems.push(
      `${NOTES_DIR}/v${version}.json: missing. Every release needs its "What's new" notes; see docs/whats-new-notes.md.`
    );
  }
  return { problems, checked: names.length };
};

const main = () => {
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const version = JSON.parse(readFileSync(path.join(root, "package.json"), "utf8")).version;
  const { problems, checked } = checkWhatsNew({ root, version });
  if (problems.length > 0) {
    for (const problem of problems) console.error(`[whats-new] ${problem}`);
    process.exit(1);
  }
  console.log(`What's new notes are ready: ${checked} file(s), including ${NOTES_DIR}/v${version}.json`);
};

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
