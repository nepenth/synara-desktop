import { execFileSync, execSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const readText = (relativePath) => readFileSync(path.join(root, relativePath), "utf8");
const writeText = (relativePath, contents) =>
  writeFileSync(path.join(root, relativePath), contents, "utf8");

const usage = `Usage: npm run bump:version -- <version> [--ios-build <build>]

Bumps the shared Synara marketing version across desktop, runtime, Cargo, Arch
packaging metadata, and iOS MARKETING_VERSION.

Optional:
  --ios-build <build>  Set synara-ios CURRENT_PROJECT_VERSION (App Store build number)
`;

const args = process.argv.slice(2);
const version = args.find((arg) => !arg.startsWith("--"));
const iosBuildFlagIndex = args.indexOf("--ios-build");
const iosBuild = iosBuildFlagIndex >= 0 ? args[iosBuildFlagIndex + 1] : undefined;

if (!version || !/^\d+\.\d+\.\d+/.test(version)) {
  console.error(usage);
  process.exit(1);
}

if (iosBuildFlagIndex >= 0 && !iosBuild) {
  console.error("Missing value for --ios-build");
  process.exit(1);
}

const replaceField = (label, text, pattern, replacement) => {
  if (!pattern.test(text)) {
    throw new Error(`Unable to update ${label}`);
  }
  return text.replace(pattern, replacement);
};

const tauriConfigPath = "src-tauri/tauri.conf.json";
const tauriConfig = JSON.parse(readText(tauriConfigPath));
tauriConfig.version = version;
writeText(tauriConfigPath, `${JSON.stringify(tauriConfig, null, 2)}\n`);

const cargoTomlPath = "src-tauri/Cargo.toml";
writeText(
  cargoTomlPath,
  replaceField(
    "src-tauri/Cargo.toml version",
    readText(cargoTomlPath),
    /^version = "[^"]+"/m,
    `version = "${version}"`
  )
);

execSync(`npm version ${version} --no-git-tag-version`, { cwd: root, stdio: "inherit" });
execSync(`node scripts/update-version.js ${version}`, {
  cwd: path.join(root, "synara"),
  stdio: "inherit",
});

const iosProjectPath = "synara-ios/project.yml";
let iosProject = readText(iosProjectPath);
iosProject = replaceField(
  "synara-ios/project.yml MARKETING_VERSION",
  iosProject,
  /MARKETING_VERSION:\s*"[^"]+"/,
  `MARKETING_VERSION: "${version}"`
);
if (iosBuild) {
  iosProject = replaceField(
    "synara-ios/project.yml CURRENT_PROJECT_VERSION",
    iosProject,
    /CURRENT_PROJECT_VERSION:\s*"[^"]+"/,
    `CURRENT_PROJECT_VERSION: "${iosBuild}"`
  );
}
writeText(iosProjectPath, iosProject);

const iosPbxprojPath = "synara-ios/Synara.xcodeproj/project.pbxproj";
let iosPbxproj = readText(iosPbxprojPath);
iosPbxproj = iosPbxproj.replaceAll(/MARKETING_VERSION = [^;]+;/g, `MARKETING_VERSION = ${version};`);
if (iosBuild) {
  iosPbxproj = iosPbxproj.replaceAll(
    /CURRENT_PROJECT_VERSION = [^;]+;/g,
    `CURRENT_PROJECT_VERSION = ${iosBuild};`
  );
}
writeText(iosPbxprojPath, iosPbxproj);

execFileSync("node", ["scripts/check-version-consistency.mjs"], {
  cwd: root,
  stdio: "inherit",
});

console.log(`Bumped shared marketing version to ${version}.`);
if (iosBuild) {
  console.log(`Set iOS build number to ${iosBuild}.`);
}