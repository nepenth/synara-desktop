import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

// Filenames cannot distinguish a version bump from dependency, build, security,
// or executable changes. Only inert release prose may skip validation.
const metadataPatterns = [
  /^CHANGELOG\.md$/,
  /^docs\/releases\/.*\.md$/,
  /^synara-ios\/release-notes\/.*\.txt$/,
];

export const isMetadataOnlyChange = (files) => {
  const names = [
    ...new Set((files || []).map((file) => file.trim()).filter(Boolean)),
  ];
  if (names.length === 0) {
    return true;
  }
  return names.every((name) =>
    metadataPatterns.some((pattern) => pattern.test(name))
  );
};

const main = () => {
  const fromArgs = process.argv.slice(2);
  const files =
    fromArgs.length > 0 ? fromArgs : readFileSync(0, "utf8").split(/\r?\n/);
  process.exit(isMetadataOnlyChange(files) ? 0 : 1);
};

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  main();
}
