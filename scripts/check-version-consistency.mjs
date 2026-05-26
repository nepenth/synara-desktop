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

const expectedVersion = tauriConfig.version;
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
      SYNARA_ARCH_STARTDIR: path.join(root, "packaging/arch"),
    },
    encoding: "utf8",
  }
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

console.log(`Version metadata is consistent at ${expectedVersion}.`);
