import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const readJson = (relativePath) =>
  JSON.parse(readFileSync(path.join(root, relativePath), "utf8"));
const readText = (relativePath) =>
  readFileSync(path.join(root, relativePath), "utf8");

const hasPlaceholder = (value) =>
  /CHANGE_ME|TODO|example\.(com|org|net)|localhost|127\.0\.0\.1|<[^>]+>|__[^_]+__/i.test(
    value
  );

const isHttpsEndpoint = (endpoint) => {
  try {
    return new URL(endpoint).protocol === "https:";
  } catch {
    return false;
  }
};

const hasWorkflowPattern = (workflow, pattern) => pattern.test(workflow);

export function inspectReleaseUpdaterReadiness({
  tauriConfig,
  cargoToml,
  rustLib,
  capabilities,
  desktopPackage,
  releaseWorkflow,
  requireEnabled = false,
}) {
  const errors = [];
  const warnings = [];
  const report = (message) => {
    if (requireEnabled) {
      errors.push(message);
    } else {
      warnings.push(message);
    }
  };

  const updaterArtifacts = tauriConfig?.bundle?.createUpdaterArtifacts;
  const updaterConfig = tauriConfig?.plugins?.updater;
  const endpoints = updaterConfig?.endpoints;
  const permissions = capabilities?.permissions ?? [];
  const packageDependencies = {
    ...(desktopPackage?.dependencies ?? {}),
    ...(desktopPackage?.devDependencies ?? {}),
  };

  if (updaterArtifacts !== true) {
    report(
      "src-tauri/tauri.conf.json must set bundle.createUpdaterArtifacts to true."
    );
  }

  if (!updaterConfig || typeof updaterConfig !== "object") {
    report("src-tauri/tauri.conf.json must define plugins.updater.");
  }

  if (
    typeof updaterConfig?.pubkey !== "string" ||
    updaterConfig.pubkey.trim().length < 40
  ) {
    report(
      "plugins.updater.pubkey must contain the production Tauri updater public key."
    );
  } else if (hasPlaceholder(updaterConfig.pubkey)) {
    report("plugins.updater.pubkey still looks like placeholder material.");
  }

  if (!Array.isArray(endpoints) || endpoints.length === 0) {
    report(
      "plugins.updater.endpoints must contain at least one production HTTPS endpoint."
    );
  } else {
    for (const endpoint of endpoints) {
      if (typeof endpoint !== "string" || endpoint.trim() === "") {
        report(
          "plugins.updater.endpoints must contain only non-empty strings."
        );
        continue;
      }
      if (!isHttpsEndpoint(endpoint)) {
        report(`Updater endpoint is not a valid HTTPS URL: ${endpoint}`);
      }
      if (hasPlaceholder(endpoint)) {
        report(`Updater endpoint still looks like a placeholder: ${endpoint}`);
      }
    }
  }

  if (!/^\s*tauri-plugin-updater\s*=.+$/m.test(cargoToml)) {
    report(
      "src-tauri/Cargo.toml must depend on tauri-plugin-updater for desktop targets."
    );
  }

  if (
    !/tauri_plugin_updater::Builder|tauri_plugin_updater::init|plugin\s*\(\s*tauri_plugin_updater/m.test(
      rustLib
    )
  ) {
    report("src-tauri/src/lib.rs must register the Tauri updater plugin.");
  }

  if (
    !permissions.some(
      (permission) =>
        permission === "updater:default" || permission === "updater:allow-check"
    )
  ) {
    report(
      "src-tauri/capabilities/main.json must grant updater:allow-check or updater:default when the frontend owns update checks."
    );
  }

  if (!packageDependencies["@tauri-apps/plugin-updater"]) {
    report(
      "package.json must depend on @tauri-apps/plugin-updater for frontend update checks."
    );
  }

  if (
    hasWorkflowPattern(
      releaseWorkflow,
      /createUpdaterArtifacts["']?\s*:\s*false/
    )
  ) {
    report(
      ".github/workflows/release-desktop.yml still overrides bundle.createUpdaterArtifacts to false."
    );
  }

  if (!hasWorkflowPattern(releaseWorkflow, /TAURI_SIGNING_PRIVATE_KEY/)) {
    report(
      ".github/workflows/release-desktop.yml must expose TAURI_SIGNING_PRIVATE_KEY to release builds."
    );
  }

  if (
    !hasWorkflowPattern(releaseWorkflow, /TAURI_SIGNING_PRIVATE_KEY_PASSWORD/)
  ) {
    report(
      ".github/workflows/release-desktop.yml must expose TAURI_SIGNING_PRIVATE_KEY_PASSWORD to release builds."
    );
  }

  if (!hasWorkflowPattern(releaseWorkflow, /\.sig|latest\.json|platforms:/)) {
    report(
      ".github/workflows/release-desktop.yml must upload updater signatures or signed update metadata."
    );
  }

  return {
    ok: errors.length === 0,
    updaterArtifactsEnabled: updaterArtifacts === true,
    errors,
    warnings,
  };
}

function main() {
  const requireEnabled = process.argv.includes("--require-enabled");
  const result = inspectReleaseUpdaterReadiness({
    tauriConfig: readJson("src-tauri/tauri.conf.json"),
    cargoToml: readText("src-tauri/Cargo.toml"),
    rustLib: readText("src-tauri/src/lib.rs"),
    capabilities: readJson("src-tauri/capabilities/main.json"),
    desktopPackage: readJson("package.json"),
    releaseWorkflow: readText(".github/workflows/release-desktop.yml"),
    requireEnabled,
  });

  for (const warning of result.warnings) {
    console.warn(`[release-updater] warning: ${warning}`);
  }
  for (const error of result.errors) {
    console.error(`[release-updater] error: ${error}`);
  }

  if (!result.ok) {
    console.error("[release-updater] release updater gate failed.");
    process.exit(1);
  }

  if (result.updaterArtifactsEnabled) {
    console.log(
      "[release-updater] updater release prerequisites are configured."
    );
  } else {
    console.log(
      "[release-updater] updater is disabled; run with --require-enabled to enforce release readiness."
    );
  }
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  main();
}
