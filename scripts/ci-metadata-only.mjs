import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

const metadataPatterns = [
  /^package\.json$/,
  /^package-lock\.json$/,
  /^synara\/package\.json$/,
  /^synara\/package-lock\.json$/,
  /^src-tauri\/tauri\.conf\.json$/,
  /^synara-ios\/project\.yml$/,
  /^synara-ios\/Synara\.xcodeproj\/project\.pbxproj$/,
  /^packaging\/arch\/PKGBUILD$/,
  /^CHANGELOG\.md$/,
  /^docs\/releases\//,
  /^synara-ios\/release-notes\//,
  /^devAssets\/index\.html$/,
  /^synara\/src\/app\/features\/settings\/about\//,
  /^synara\/src\/app\/pages\/auth\/AuthFooter\.tsx$/,
  /^synara\/src\/app\/pages\/client\/WelcomePage\.tsx$/,
];

export const isMetadataOnlyChange = (files) => {
  const names = [...new Set((files || []).map((file) => file.trim()).filter(Boolean))];
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
    fromArgs.length > 0
      ? fromArgs
      : readFileSync(0, "utf8").split(/\r?\n/);
  process.exit(isMetadataOnlyChange(files) ? 0 : 1);
};

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  main();
}
