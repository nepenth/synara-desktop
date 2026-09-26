import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const version = JSON.parse(readFileSync(path.join(root, "package.json"), "utf8")).version;
const relativePath = `synara-ios/release-notes/v${version}-en-US.txt`;

let notes;
try {
  notes = readFileSync(path.join(root, relativePath), "utf8").trim();
} catch (error) {
  if (error?.code !== "ENOENT") throw error;
  throw new Error(`Missing TestFlight release notes: ${relativePath}`);
}

if (notes.length === 0 || notes.length > 4_000) {
  throw new Error(`${relativePath} must contain 1–4000 characters of release notes.`);
}

console.log(`TestFlight release notes are ready: ${relativePath}`);
