import { rustDeclarationSurface } from "./rust-module-sources.mjs";

const operations = [
  [
    "secret_storage_bootstrap",
    "passphrase",
    "SecretStorageSetupDto",
    "SessionStatusError",
  ],
  [
    "secret_storage_unlock",
    "recovery_secret",
    "SecretStorageStatusDto",
    "SessionStatusError",
  ],
  [
    "secret_storage_reset",
    "passphrase",
    "SecretStorageSetupDto",
    "SessionStatusError",
  ],
  ["backup_setup", "passphrase", "BackupStatusDto", "LeftoverCommandError"],
  [
    "backup_repair",
    "recovery_secret",
    "BackupStatusDto",
    "LeftoverCommandError",
  ],
];

export function inspectTypedRecoveryBoundaries({ udl, ffi }) {
  ffi = rustDeclarationSurface(ffi);
  const errors = [];
  for (const [method, argument, result, error] of operations) {
    // `udl` is the FFI surface rendered from the pinned Swift golden; Swift
    // has no typed throws, so the error type is checked on the Rust export.
    const signature = `[Async, Throws] ${result} ${method}(string ${argument});`;
    if (!udl.includes(signature))
      errors.push(`${method} requires its dedicated typed UDL signature`);
    const start = ffi.indexOf(`    pub async fn ${method}(`);
    const end = ffi.indexOf("\n    }", start);
    const body = start >= 0 && end > start ? ffi.slice(start, end) : "";
    if (!body.replace(/\s+/g, " ").includes(`-> Result<${result}, ${error}>`)) {
      errors.push(`${method} requires its dedicated typed UDL signature`);
    }
    if (!body.includes(`Zeroizing::new(${argument})`)) {
      errors.push(`${method} must retain recovery input in a zeroizing buffer`);
    }
    if (
      !new RegExp(`\\.core\\s*\\.${method}\\(secret\\.as_str\\(\\)\\)`).test(
        body
      )
    ) {
      errors.push(`${method} must call the dedicated Core operation`);
    }
    if (
      /\.command\s*\(|serde_json::json!|(?:println!|tracing::|log::)/.test(body)
    ) {
      errors.push(
        `${method} must not put recovery input in generic envelopes or logs`
      );
    }
  }
  const setup = udl.match(
    /dictionary SecretStorageSetupDto\s*\{([^}]+)\}/
  )?.[1];
  if (
    setup?.trim() !== "SecretStorageStatusDto status;\n  string? recovery_key;"
  ) {
    errors.push(
      "SecretStorageSetupDto must expose only status and the explicit once-displayed recovery key"
    );
  }
  return { ok: errors.length === 0, errors };
}
