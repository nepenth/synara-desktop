#!/usr/bin/env node

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const read = (path) => readFileSync(resolve(root, path), "utf8");
const workspace = read("Cargo.toml");
const coreManifest = read("crates/synara-core/Cargo.toml");
const nseManifest = read("crates/synara-nse-core/Cargo.toml");
const nseUdl = read("crates/synara-nse-core/src/synara_nse_core.udl");
const nseRust = read("crates/synara-nse-core/src/lib.rs");
const generator = read("scripts/generate-synara-nse-core-swift.sh");
const publicationHelper = read("scripts/lib/publish-generated-apple-pair.sh");
const generatorSyntax = spawnSync(
  "bash",
  ["-n", resolve(root, "scripts/generate-synara-nse-core-swift.sh")],
  { encoding: "utf8" },
);
if (generatorSyntax.status !== 0) {
  throw new Error(
    `SynaraNseCore generator shell syntax failed: ${generatorSyntax.stderr || generatorSyntax.stdout}`,
  );
}
const notificationService = read(
  "synara-ios/SynaraNotificationService/NotificationService.swift",
);
const project = read("synara-ios/project.yml");

const requireText = (source, needle, label) => {
  if (!source.includes(needle)) throw new Error(`missing ${label}: ${needle}`);
};
const forbidText = (source, needle, label) => {
  if (source.includes(needle)) throw new Error(`forbidden ${label}: ${needle}`);
};

requireText(workspace, "[profile.nse-release]", "NSE size profile");
requireText(workspace, 'lto = "fat"', "NSE cross-crate LTO");
requireText(coreManifest, 'default = ["full-uniffi"]', "full Core default feature");
requireText(coreManifest, "nse-preview = []", "NSE Core feature");
requireText(nseManifest, "default-features = false", "full binding exclusion");
requireText(nseManifest, 'features = ["nse-preview"]', "NSE-only feature");
requireText(nseUdl, "interface NsePreviewRequest {", "cancelable request boundary");
requireText(nseUdl, "NsePreviewDto resolve();", "one-shot resolver");
requireText(nseUdl, "void cancel();", "prompt cancellation operation");
requireText(nseUdl, "bytes? get(string key);", "read-only secret callback");
for (const forbidden of [" put(", " delete(", "close_read_only_store", "interface NseCore {"]) {
  forbidText(nseUdl, forbidden, "NSE UDL capability");
}
if ((nseUdl.match(/\[Async/g) ?? []).length !== 1) {
  throw new Error("NSE UDL must expose exactly one async operation");
}
forbidText(nseRust, "synara_core::SharedCore", "full application owner in NSE Rust boundary");
requireText(generator, '--profile "$rust_profile"', "NSE-specific Cargo profile");
requireText(generator, 'headers_tmp="$headers_root/synara_nse_coreFFI"', "namespaced C module");
requireText(generator, "simulator-arm64", "bounded local simulator generation mode");
requireText(generator, "SYNARA_NSE_CORE_APPLE_SPACE_BOUNDED", "space-bounded build mode");
requireText(generator, 'target_build_dir="$work_dir/cargo-target-$target"', "isolated target build");
requireText(generator, 'remove_bounded_target_dir "$target_build_dir"', "bounded target cleanup");
requireText(generator, '"$publication_helper"', "shared artifact publication");
requireText(publicationHelper, 'publication_state="publishing"', "transactional publication state");
requireText(publicationHelper, 'publication_state="committed"', "coherent pair commit point");
requireText(notificationService, "import SynaraNseCore", "NSE-only Swift module import");
requireText(notificationService, "request.cancel()", "NSE deadline cancellation");
forbidText(notificationService, "import SynaraCore", "full Core import in extension");
requireText(project, "package: SynaraNseCore", "extension NSE-only package dependency");

console.log("Synara NSE Core isolation scaffold checks passed.");
