#!/usr/bin/env node
/**
 * Host-neutral P4-1 contract check. It proves the committed inputs describe a
 * real, reproducible Apple build without pretending a non-Apple host produced
 * an XCFramework or generated Swift output.
 */
import { accessSync, readFileSync } from "node:fs";
import { constants } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const required = [
  "crates/synara-core/src/synara_core.udl",
  "crates/synara-core/src/ffi.rs",
  "crates/synara-core/build.rs",
  "crates/synara-core-bindgen/Cargo.toml",
  "crates/synara-core-bindgen/src/main.rs",
  "synara-ios/SynaraCore/Package.swift",
  "synara-ios/SynaraCore/Sources/SynaraCore/SynaraCore.swift",
  "synara-ios/SynaraCore/Sources/synara_coreFFI/include/.gitkeep",
  "synara-ios/SynaraCore/.gitignore",
  "scripts/generate-synara-core-swift.sh",
];
for (const path of required) {
  try {
    accessSync(resolve(root, path), constants.R_OK);
  } catch {
    throw new Error(`missing P4-1 scaffold input: ${path}`);
  }
}

const cargo = readFileSync(resolve(root, "crates/synara-core/Cargo.toml"), "utf8");
const udl = readFileSync(resolve(root, "crates/synara-core/src/synara_core.udl"), "utf8");
const lib = readFileSync(resolve(root, "crates/synara-core/src/lib.rs"), "utf8");
const ffi = readFileSync(resolve(root, "crates/synara-core/src/ffi.rs"), "utf8");
const packageManifest = readFileSync(resolve(root, "synara-ios/SynaraCore/Package.swift"), "utf8");
const ignored = readFileSync(resolve(root, "synara-ios/SynaraCore/.gitignore"), "utf8");
const generator = readFileSync(resolve(root, "scripts/generate-synara-core-swift.sh"), "utf8");

const assertions = [
  [cargo, 'crate-type = ["lib", "staticlib", "cdylib"]', "Apple library crate types"],
  [cargo, 'uniffi = { version = "=0.28.3" }', "pinned UniFFI runtime"],
  [readFileSync(resolve(root, "crates/synara-core-bindgen/Cargo.toml"), "utf8"), 'uniffi = { version = "=0.28.3", features = ["cli"] }', "pinned project-owned UniFFI generator"],
  [cargo, 'features = ["build"]', "UniFFI build scaffolding"],
  [udl, "namespace synara_core", "project-owned UniFFI namespace"],
  [udl, "binding_scaffold_version", "P4-1 binding bootstrap"],
  [udl, "[Async, Throws=LoginFlowsError]", "async typed login-flow operation"],
  [udl, "sequence<LoginFlowDto> login_flows(string homeserver_url)", "typed login-flow return"],
  [udl, "dictionary LoginFlowDto", "typed login-flow DTO"],
  [udl, "boolean? get_login_token", "optional token-capability metadata"],
  [udl, "interface LoginFlowsError", "typed privacy-safe login-flow error"],
  [lib, 'uniffi::include_scaffolding!("synara_core")', "Rust FFI scaffolding inclusion"],
  [ffi, "discover_login_flows", "shared-core login-flow discovery call"],
  [ffi, "HttpLoginFlowTransport::new()", "bounded Core login-flow transport"],
  [readFileSync(resolve(root, "crates/synara-core/build.rs"), "utf8"), "unexpected UniFFI 0.28.3 metadata-doc shape", "fail-closed generated-scaffolding lint patch"],
  [packageManifest, 'name: "SynaraCore"', "Swift package target"],
  [packageManifest, 'name: "synara_coreFFI"', "generated C FFI binary target"],
  [packageManifest, '.binaryTarget(', "generated XCFramework binary target"],
  [packageManifest, 'path: "Artifacts/SynaraCore.xcframework"', "generated XCFramework path"],
  [packageManifest, 'dependencies: ["synara_coreFFI"]', "Swift-to-generated-FFI dependency"],
  [packageManifest, 'path: "Sources/SynaraCore"', "generated-source package target"],
  [ignored, "/Sources/SynaraCore/Generated/*.swift", "generated Swift exclusion"],
  [ignored, "/Sources/synara_coreFFI/include/*", "generated C FFI exclusion"],
  [ignored, "/Artifacts/", "generated XCFramework exclusion"],
  [ignored, "/.build/", "Swift package build-product exclusion"],
  [generator, '[[ "$(uname -s)" != "Darwin" ]]', "clear non-Apple failure"],
  [generator, "aarch64-apple-ios-sim", "Apple Silicon simulator target"],
  [generator, "x86_64-apple-ios", "Intel simulator target"],
  [generator, "xcrun lipo -create", "combined generic simulator library"],
  [generator, "aarch64-apple-darwin", "Apple macOS target"],
  [generator, "cargo build --locked --release --package synara-core", "locked Rust build"],
  [generator, "cargo run --locked --package synara-core-bindgen", "project-owned locked bindgen invocation"],
  [generator, "-- generate", "project-owned bindgen command"],
  [generator, "--no-format", "source-preserving bindgen invocation"],
  [generator, 'synara_coreFFI.modulemap', "generated FFI module map publication"],
  [generator, 'module.modulemap', "XCFramework module map structure"],
  [generator, '-headers "$headers_tmp"', "XCFramework generated C header inclusion"],
  [generator, "xcodebuild -create-xcframework", "XCFramework assembly"],
];
for (const [text, needle, label] of assertions) {
  if (!text.includes(needle)) throw new Error(`missing ${label}: ${needle}`);
}

console.log("SynaraCore UniFFI Swift scaffold contract passed.");
