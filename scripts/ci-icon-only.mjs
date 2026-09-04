import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

import { isMetadataOnlyChange } from "./ci-metadata-only.mjs";

const productionIconPatterns = [
  /^assets\/branding\/app-icon-manifest\.json$/,
  /^assets\/branding\/synara-app-icon-(?:master|small|desktop)\.png$/,
  /^assets\/branding\/synara-symbolic\.svg$/,
  /^src-tauri\/icons\/(?!tray-template\.png$).+\.(?:png|icns|ico)$/,
  /^synara-ios\/Synara\/Resources\/Assets\.xcassets\/AppIcon\.appiconset\/(?:Contents\.json|AppIcon-[0-9]+\.png)$/,
];

const iconInfrastructurePatterns = [
  /^scripts\/(?:check-app-icons\.mjs|ci-icon-only\.mjs|generate-app-icons\.swift|render-app-icon-review\.swift)$/,
  /^scripts\/__tests__\/(?:check-app-icons|ci-icon-only)\.test\.mjs$/,
  /^docs\/design\/app-icon-refresh\//,
];

const matches = (patterns, name) =>
  patterns.some((pattern) => pattern.test(name));

export const isIconOnlyChange = (files) => {
  const names = [
    ...new Set((files || []).map((file) => file.trim()).filter(Boolean)),
  ];
  if (names.length === 0) return false;
  const hasProductionIcon = names.some((name) =>
    matches(productionIconPatterns, name)
  );
  if (!hasProductionIcon) return false;
  return names.every(
    (name) =>
      matches(productionIconPatterns, name) ||
      matches(iconInfrastructurePatterns, name) ||
      isMetadataOnlyChange([name])
  );
};

const main = () => {
  const fromArgs = process.argv.slice(2);
  const files =
    fromArgs.length > 0 ? fromArgs : readFileSync(0, "utf8").split(/\r?\n/);
  process.exit(isIconOnlyChange(files) ? 0 : 1);
};

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  main();
}
