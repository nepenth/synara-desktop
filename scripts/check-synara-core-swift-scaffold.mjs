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
  "crates/synara-core/src/session_projection_ffi.rs",
  "crates/synara-core/build.rs",
  "crates/synara-core-bindgen/Cargo.toml",
  "crates/synara-core-bindgen/src/main.rs",
  "synara-ios/SynaraCore/Package.swift",
  "synara-ios/SynaraCore/Sources/SynaraCore/SynaraCore.swift",
  "synara-ios/Synara/Services/MatrixSessionProjectionMirror.swift",
  "synara-ios/SynaraTests/SynaraCoreBindingsTests.swift",
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
const sessionProjectionFfi = readFileSync(
  resolve(root, "crates/synara-core/src/session_projection_ffi.rs"),
  "utf8"
);
const sessionProjectionAdapter = readFileSync(
  resolve(root, "synara-ios/Synara/Services/MatrixSessionProjectionMirror.swift"),
  "utf8"
);
const swiftBindingsTests = readFileSync(
  resolve(root, "synara-ios/SynaraTests/SynaraCoreBindingsTests.swift"),
  "utf8"
);
const matrixRustSDKService = readFileSync(
  resolve(root, "synara-ios/Synara/Services/MatrixRustSDKService.swift"),
  "utf8"
);
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
  [udl, "dictionary SessionProjection", "P4-3 safe session-projection record"],
  [udl, "interface SessionProjectionCore", "P4-3 project-owned session facade"],
  [udl, "SessionProjection? session_snapshot()", "P4-3 projection snapshot operation"],
  [udl, "interface SessionProjectionError", "P4-3 static privacy-safe error"],
  [lib, 'uniffi::include_scaffolding!("synara_core")', "Rust FFI scaffolding inclusion"],
  [lib, "SessionProjectionCore", "P4-3 facade export"],
  [sessionProjectionFfi, "Core::with_registry", "P4-3 Core open/close/snapshot delegation"],
  [sessionProjectionFfi, "CommandRegistry::new()", "P4-3 facade has no command registry"],
  [sessionProjectionFfi, "uniffi_projection_facade_executes_core_open_snapshot_and_close", "P4-3 Rust behavioral facade test"],
  [sessionProjectionFfi, "facade_rejects_hostile_values_with_static_privacy_safe_error", "P4-3 Rust hostile-input privacy test"],
  [sessionProjectionAdapter, "openAfterInstalledClient", "post-install projection hook"],
  [sessionProjectionAdapter, "closeBeforeSDKWipe", "pre-wipe projection close hook"],
  [swiftBindingsTests, "testSessionProjectionFacadeExecutesOpenSnapshotAndCloseOverGeneratedRustFFI", "Swift behavioral FFI test"],
  [swiftBindingsTests, "try await core.open", "Swift generated FFI open execution"],
  [swiftBindingsTests, "try await core.sessionSnapshot()", "Swift generated FFI snapshot execution"],
  [swiftBindingsTests, "try await core.close()", "Swift generated FFI close execution"],
  [
    matrixRustSDKService,
    "self.client = client\n            activeSession = session\n            await sessionProjectionMirror.openAfterInstalledClient",
    "login mirror only after MatrixRustSDK client install",
  ],
  [
    matrixRustSDKService,
    "client = newClient\n            activeSession = session\n            await sessionProjectionMirror.openAfterInstalledClient",
    "restore mirror only after MatrixRustSDK client install",
  ],
  [
    matrixRustSDKService,
    "await sessionProjectionMirror.closeBeforeSDKWipe()\n        retainClientHandle(client)",
    "projection close before SDK client release/wipe",
  ],
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

// The app's MatrixRustSDK service constructs this actor from another file in
// the same app target, so it must be module-internal. Parse enough Swift
// lexically to remove comments and string literals before inspecting the one
// declaration. The scan binds access modifiers across arbitrary whitespace:
// a modifier split onto a preceding line must not evade the privacy contract.
const stripSwiftCommentsAndStrings = (source) => {
  let output = "";
  let index = 0;
  const blank = (text) => text.replace(/[^\n]/g, " ");
  while (index < source.length) {
    if (source.startsWith("//", index)) {
      const end = source.indexOf("\n", index);
      const stop = end === -1 ? source.length : end;
      output += blank(source.slice(index, stop));
      index = stop;
    } else if (source.startsWith("/*", index)) {
      let depth = 1;
      let cursor = index + 2;
      while (cursor < source.length && depth > 0) {
        if (source.startsWith("/*", cursor)) {
          depth += 1;
          cursor += 2;
        } else if (source.startsWith("*/", cursor)) {
          depth -= 1;
          cursor += 2;
        } else {
          cursor += 1;
        }
      }
      output += blank(source.slice(index, cursor));
      index = cursor;
    } else if (source.startsWith('"""', index)) {
      let cursor = index + 3;
      while (cursor < source.length && !source.startsWith('"""', cursor)) cursor += 1;
      cursor = Math.min(source.length, cursor + 3);
      output += blank(source.slice(index, cursor));
      index = cursor;
    } else if (source[index] === '"') {
      let cursor = index + 1;
      while (cursor < source.length) {
        if (source[cursor] === "\\") cursor += 2;
        else if (source[cursor++] === '"') break;
      }
      output += blank(source.slice(index, cursor));
      index = cursor;
    } else {
      output += source[index++];
    }
  }
  return output;
};

const swiftTokens = (source) => [
  ...source.matchAll(/[A-Za-z_][A-Za-z0-9_]*|[{}:]/g),
].map((match) => ({ value: match[0], index: match.index }));

const accessModifiers = new Set([
  "public",
  "open",
  "private",
  "fileprivate",
  "package",
  "internal",
  "final",
  "nonisolated",
  "isolated",
  "static",
  "class",
  "distributed",
]);

const isModuleInternalProjectionActor = (source) => {
  const stripped = stripSwiftCommentsAndStrings(source);
  // Conditional branches make a lightweight source scanner ambiguous. This
  // one-file internal adapter has no need for them, so reject rather than
  // guessing which declaration Xcode activates.
  if (/^\s*#\s*(?:if|elseif|else|endif)\b/m.test(stripped) || stripped.includes("@")) {
    return false;
  }

  const tokens = swiftTokens(stripped);
  const namePositions = tokens
    .map((token, index) => (token.value === "MatrixSessionProjectionMirror" ? index : -1))
    .filter((index) => index >= 0);
  if (namePositions.length !== 1) return false;

  const name = namePositions[0];
  if (tokens[name - 1]?.value !== "actor") return false;
  if (!new Set(["{", ":"]).has(tokens[name + 1]?.value)) return false;

  // The adapter must be a file-scope declaration. An unmodified nested actor
  // inherits an enclosing type/extension's access, so it is not enough to
  // inspect only the actor's direct prefix. Track braces from the token stream
  // and reject any declaration inside a type or extension body.
  let braceDepth = 0;
  for (const token of tokens.slice(0, name)) {
    if (token.value === "{") braceDepth += 1;
    if (token.value === "}") {
      braceDepth -= 1;
      if (braceDepth < 0) return false;
    }
  }
  if (braceDepth !== 0) return false;

  // Bind every consecutive declaration modifier before `actor` irrespective
  // of newlines/comments. Only no modifier or the explicit `internal` marker
  // is accepted. A preceding import/other declaration is not a modifier and
  // therefore correctly terminates this declaration-prefix scan.
  const modifiers = [];
  for (let index = name - 2; index >= 0 && accessModifiers.has(tokens[index].value); index -= 1) {
    modifiers.unshift(tokens[index].value);
  }
  return modifiers.length === 0 || (modifiers.length === 1 && modifiers[0] === "internal");
};

for (const [fixture, expected] of [
  ["actor MatrixSessionProjectionMirror {}", true],
  ["internal actor MatrixSessionProjectionMirror {}", true],
  ["internal\nactor MatrixSessionProjectionMirror {}", true],
  ["public actor MatrixSessionProjectionMirror {}", false],
  ["public\nactor MatrixSessionProjectionMirror {}", false],
  ["private\n/* comment */\nactor MatrixSessionProjectionMirror {}", false],
  ["// actor MatrixSessionProjectionMirror {}\npublic actor Other {}", false],
  ['let bait = "actor MatrixSessionProjectionMirror {}"', false],
  ["fileprivate actor MatrixSessionProjectionMirror {}", false],
  ["package actor MatrixSessionProjectionMirror {}", false],
  ["package\nactor MatrixSessionProjectionMirror {}", false],
  ["distributed actor MatrixSessionProjectionMirror {}", false],
  ["public distributed actor MatrixSessionProjectionMirror {}", false],
  ["private\ndistributed actor MatrixSessionProjectionMirror {}", false],
  ["final internal actor MatrixSessionProjectionMirror {}", false],
  ["final class MatrixSessionProjectionMirror {}", false],
  ["public extension PublicHost { actor MatrixSessionProjectionMirror {} }", false],
  ["struct Host {\ninternal actor MatrixSessionProjectionMirror {}\n}", false],
  ["#if false\nactor MatrixSessionProjectionMirror {}\n#endif", false],
  ["#if false\nactor MatrixSessionProjectionMirror {}\n#endif\npublic actor MatrixSessionProjectionMirror {}", false],
]) {
  if (isModuleInternalProjectionActor(fixture) !== expected) {
    throw new Error("P4-3 projection actor access parser fixture failed");
  }
}
if (!isModuleInternalProjectionActor(sessionProjectionAdapter)) {
  throw new Error("P4-3 projection adapter must be exactly one module-internal actor declaration");
}

// P4-3 is a deliberately closed session projection, not a second Matrix
// session implementation. Parse the one public record rather than scanning
// comments/tests, then fail if its field census changes or a secret field is
// ever introduced into the UniFFI surface.
const projectionRecord = udl.match(/dictionary SessionProjection \{([\s\S]*?)\};/);
if (!projectionRecord) throw new Error("missing P4-3 SessionProjection record");
const projectionFields = [...projectionRecord[1].matchAll(/^\s*(?:u64|string|boolean|SessionProjectionLifecycle)\s+(\w+);/gm)].map(
  ([, field]) => field
);
const approvedProjectionFields = [
  "generation",
  "user_id",
  "device_id",
  "homeserver_url",
  "lifecycle",
  "crypto_ready",
];
if (
  projectionFields.length !== approvedProjectionFields.length ||
  projectionFields.some((field, index) => field !== approvedProjectionFields[index])
) {
  throw new Error(
    `P4-3 SessionProjection must expose exactly ${approvedProjectionFields.join(", ")}; found ${projectionFields.join(", ")}`
  );
}
for (const forbidden of [
  "access_token",
  "refresh_token",
  "password",
  "recovery_key",
  "private_key",
  "session_key",
  "display_name",
  "avatar_url",
  "client",
  "store",
]) {
  if (projectionFields.includes(forbidden)) {
    throw new Error(`P4-3 secret/private field must not cross UniFFI: ${forbidden}`);
  }
}

const projectionObject = udl.match(/interface SessionProjectionCore \{([\s\S]*?)\};/);
if (!projectionObject) throw new Error("missing P4-3 SessionProjectionCore object");
const projectionOperations = [
  ...projectionObject[1].matchAll(/(?:constructor|void|SessionProjection\?)\s+(\w+)\s*\(/g),
].map(([, operation]) => operation);
if (projectionOperations.join(",") !== "open,session_snapshot,close") {
  throw new Error(`P4-3 facade must expose only open/session_snapshot/close; found ${projectionOperations.join(", ")}`);
}

// P4-3's private Core dependency must continue satisfying every required
// Platform method without turning media configuration into a projection or a
// UniFFI surface. Check the exact inert, closed, string-free implementation.
const projectionPlatformImplStart = sessionProjectionFfi.indexOf(
  "impl Platform for ProjectionOnlyPlatform {"
);
const projectionPlatformImplEnd = sessionProjectionFfi.indexOf(
  "\n}\n\nfn inert_platform_error",
  projectionPlatformImplStart
);
const inertProjectionMediaConfig = `fn media_config(&self) -> MediaConfigFuture<'_> {
        Box::pin(async { Err(PlatformMediaConfigError::NoSession) })
    }`;
const inertProjectionMediaConfigStart = sessionProjectionFfi.indexOf(inertProjectionMediaConfig);
if (
  projectionPlatformImplStart < 0 ||
  projectionPlatformImplEnd < 0 ||
  inertProjectionMediaConfigStart < projectionPlatformImplStart ||
  inertProjectionMediaConfigStart > projectionPlatformImplEnd
) {
  throw new Error(
    "P4-3 ProjectionOnlyPlatform must implement Platform::media_config with the exact closed, string-free NoSession future"
  );
}

console.log("SynaraCore UniFFI Swift scaffold contract passed.");
