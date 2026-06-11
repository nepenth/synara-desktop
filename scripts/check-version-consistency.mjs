import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const readJson = (relativePath) => JSON.parse(readFileSync(path.join(root, relativePath), "utf8"));
const readText = (relativePath) => readFileSync(path.join(root, relativePath), "utf8");

const assertEqual = (label, actual, expected) => {
  if (actual !== expected) {
    throw new Error(`${label} is ${actual}, expected ${expected}`);
  }
};

const matchRequired = (label, text, pattern) => {
  const match = text.match(pattern);
  if (!match) throw new Error(`Unable to read ${label}`);
  return match[1];
};

const desktopPackage = readJson("package.json");
const desktopPackageLock = readJson("package-lock.json");
const tauriConfig = readJson("src-tauri/tauri.conf.json");
const cargoToml = readText("src-tauri/Cargo.toml");
const cargoLock = readText("src-tauri/Cargo.lock");
const runtimePackage = readJson("synara/package.json");
const runtimePackageLock = readJson("synara/package-lock.json");
const iosProject = readText("synara-ios/Synara.xcodeproj/project.pbxproj");
const iosXcodeGenProject = readText("synara-ios/project.yml");

const expectedVersion = tauriConfig.version;
const iosBuildVersion = matchRequired(
  "synara-ios/project.yml CURRENT_PROJECT_VERSION",
  iosXcodeGenProject,
  /CURRENT_PROJECT_VERSION:\s*"([^"]+)"/
);
const cargoVersion = matchRequired(
  "src-tauri/Cargo.toml package version",
  cargoToml,
  /^\[package\][\s\S]*?^version = "([^"]+)"/m
);
const cargoLockVersion = matchRequired(
  "src-tauri/Cargo.lock synara package version",
  cargoLock,
  /\[\[package\]\]\s*\nname = "synara"\s*\nversion = "([^"]+)"/
);
const archStartDir = path.join(root, "packaging/arch");
const archPkgver = execFileSync(
  "/bin/bash",
  [
    "-lc",
    'startdir="$SYNARA_ARCH_STARTDIR"; source "$SYNARA_ARCH_STARTDIR/PKGBUILD"; printf "%s" "$pkgver"',
  ],
  {
    cwd: root,
    env: {
      ...process.env,
      SYNARA_ARCH_STARTDIR: archStartDir,
    },
    encoding: "utf8",
  }
);
const archPkgrel = Number.parseInt(
  execFileSync(
    "/bin/bash",
    [
      "-lc",
      'startdir="$SYNARA_ARCH_STARTDIR"; source "$SYNARA_ARCH_STARTDIR/PKGBUILD"; printf "%s" "$pkgrel"',
    ],
    {
      cwd: root,
      env: {
        ...process.env,
        SYNARA_ARCH_STARTDIR: archStartDir,
      },
      encoding: "utf8",
    }
  ),
  10
);

assertEqual("package.json version", desktopPackage.version, expectedVersion);
assertEqual("package-lock.json version", desktopPackageLock.version, expectedVersion);
assertEqual(
  "package-lock.json root package version",
  desktopPackageLock.packages?.[""]?.version,
  expectedVersion
);
assertEqual("src-tauri/Cargo.toml version", cargoVersion, expectedVersion);
assertEqual("src-tauri/Cargo.lock synara version", cargoLockVersion, expectedVersion);
assertEqual("synara/package.json version", runtimePackage.version, expectedVersion);
assertEqual("synara/package-lock.json version", runtimePackageLock.version, expectedVersion);
assertEqual(
  "synara/package-lock.json root package version",
  runtimePackageLock.packages?.[""]?.version,
  expectedVersion
);
assertEqual("packaging/arch/PKGBUILD pkgver", archPkgver, expectedVersion.replaceAll("-", "_"));
if (!Number.isInteger(archPkgrel) || archPkgrel < 1) {
  throw new Error(`packaging/arch/PKGBUILD pkgrel is ${archPkgrel}, expected a positive integer`);
}
assertEqual(
  "synara-ios/project.yml MARKETING_VERSION",
  matchRequired("synara-ios/project.yml MARKETING_VERSION", iosXcodeGenProject, /MARKETING_VERSION:\s*"([^"]+)"/),
  expectedVersion
);
for (const marketingVersion of iosProject.matchAll(/MARKETING_VERSION = ([^;]+);/g)) {
  assertEqual("synara-ios/Synara.xcodeproj MARKETING_VERSION", marketingVersion[1], expectedVersion);
}
for (const buildVersion of iosProject.matchAll(/CURRENT_PROJECT_VERSION = ([^;]+);/g)) {
  assertEqual("synara-ios/Synara.xcodeproj CURRENT_PROJECT_VERSION", buildVersion[1], iosBuildVersion);
}

const iosMarketingVersion = matchRequired(
  "synara-ios/project.yml MARKETING_VERSION",
  iosXcodeGenProject,
  /MARKETING_VERSION:\s*"([^"]+)"/
);

const parseMajorMinor = (version) => {
  const match = `${version}`.match(/^(\d+)\.(\d+)/);
  if (!match) throw new Error(`Unable to parse major.minor from version ${version}`);
  return `${match[1]}.${match[2]}`;
};

const cargoTauriVersion = matchRequired(
  "src-tauri/Cargo.lock tauri crate version",
  cargoLock,
  /name = "tauri"\nversion = "([^"]+)"/
);
const npmTauriApiVersion = desktopPackage.dependencies?.["@tauri-apps/api"];
const npmTauriCliVersion = desktopPackage.devDependencies?.["@tauri-apps/cli"];

if (!npmTauriApiVersion) {
  throw new Error("package.json is missing @tauri-apps/api dependency");
}
if (!npmTauriCliVersion) {
  throw new Error("package.json is missing @tauri-apps/cli devDependency");
}

const cargoTauriMajorMinor = parseMajorMinor(cargoTauriVersion);
const npmTauriApiMajorMinor = parseMajorMinor(npmTauriApiVersion);
const npmTauriCliMajorMinor = parseMajorMinor(npmTauriCliVersion);

assertEqual(
  "Tauri npm api major.minor vs Cargo.lock tauri",
  npmTauriApiMajorMinor,
  cargoTauriMajorMinor
);
assertEqual(
  "Tauri npm cli major.minor vs Cargo.lock tauri",
  npmTauriCliMajorMinor,
  cargoTauriMajorMinor
);

console.log(
  `Tauri toolchain aligned at ${cargoTauriMajorMinor} (api ${npmTauriApiVersion}, cli ${npmTauriCliVersion}, cargo ${cargoTauriVersion}).`
);
console.log(`Version metadata is consistent at ${expectedVersion} (Arch pkgrel ${archPkgrel}).`);
if (iosMarketingVersion === expectedVersion) {
  console.log(
    `iOS App Store build number is ${iosBuildVersion} (CURRENT_PROJECT_VERSION); marketing version matches desktop at ${expectedVersion}.`
  );
}
