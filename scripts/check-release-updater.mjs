import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const releaseWorkflowPath = ".github/workflows/release.yml";

const readJson = (relativePath) =>
  JSON.parse(readFileSync(path.join(root, relativePath), "utf8"));
const readText = (relativePath) =>
  readFileSync(path.join(root, relativePath), "utf8");

const hasPlaceholder = (value) =>
  /CHANGE_ME|TODO|example\.(com|org|net)|localhost|127\.0\.0\.1|<[^>]+>|__[^_]+__/i.test(
    value,
  );

const isHttpsEndpoint = (endpoint) => {
  try {
    return new URL(endpoint).protocol === "https:";
  } catch {
    return false;
  }
};

const hasWorkflowPattern = (workflow, pattern) => pattern.test(workflow);

const workflowStepContaining = (workflow, command) => {
  const lines = workflow.split(/\r?\n/);
  const commandIndex = lines.findIndex((line) => line.includes(command));
  if (commandIndex === -1) return "";

  let start = commandIndex;
  let stepIndent;
  while (start >= 0) {
    const match = lines[start].match(/^(\s*)-\s+(?:name|run|uses):/);
    if (match) {
      stepIndent = match[1].length;
      break;
    }
    start -= 1;
  }
  if (stepIndent === undefined) return "";

  let end = start + 1;
  while (end < lines.length) {
    const match = lines[end].match(/^(\s*)-\s+(?:name|run|uses):/);
    if (match && match[1].length === stepIndent) break;
    end += 1;
  }
  return lines.slice(start, end).join("\n");
};

const workflowConfiguresUpdaterChannel = (workflow) =>
  hasWorkflowPattern(workflow, /configure-release-updater\.mjs/) &&
  hasWorkflowPattern(workflow, /SYNARA_UPDATER_PUBKEY/) &&
  hasWorkflowPattern(workflow, /SYNARA_UPDATER_ENDPOINT/);

const workflowConfiguresTauriNotarization = (workflow) => {
  const buildStep = workflowStepContaining(
    workflow,
    "npm run tauri build -- --target universal-apple-darwin",
  );
  return (
    hasWorkflowPattern(
      buildStep,
      /APPLE_ID:\s*\$\{\{\s*secrets\.APPLE_ID\s*\}\}/,
    ) &&
    hasWorkflowPattern(
      buildStep,
      /APPLE_PASSWORD:\s*\$\{\{\s*secrets\.APPLE_APP_SPECIFIC_PASSWORD\s*\}\}/,
    ) &&
    hasWorkflowPattern(
      buildStep,
      /APPLE_TEAM_ID:\s*\$\{\{\s*secrets\.APPLE_TEAM_ID\s*\}\}/,
    )
  );
};

const workflowPublishesGeneratedUpdaterMetadata = (workflow) =>
  hasWorkflowPattern(workflow, /updater-metadata:/) &&
  hasWorkflowPattern(workflow, /needs:\s*\[\s*macos\s*\]/) &&
  hasWorkflowPattern(workflow, /actions\/download-artifact/) &&
  hasWorkflowPattern(workflow, /generate-release-updater-metadata\.mjs/) &&
  hasWorkflowPattern(workflow, /--repo\s+["']?\$GITHUB_REPOSITORY["']?/) &&
  hasWorkflowPattern(workflow, /--tag\s+["']?\$GITHUB_REF_NAME["']?/) &&
  hasWorkflowPattern(workflow, /name:\s*gh-release-updater/) &&
  hasWorkflowPattern(workflow, /path:\s*latest\.json/) &&
  hasWorkflowPattern(workflow, /softprops\/action-gh-release/) &&
  hasWorkflowPattern(
    workflow,
    /files:\s*\|[\s\S]*release-artifacts\/gh-release-updater\/latest\.json/,
  );

const workflowVerifiesMacosDistributableContents = (workflow) =>
  hasWorkflowPattern(workflow, /Verify macOS distributable contents/) &&
  hasWorkflowPattern(workflow, /hdiutil\s+attach/) &&
  hasWorkflowPattern(workflow, /tar\s+-xzf/) &&
  hasWorkflowPattern(
    workflow,
    /codesign\s+--verify[\s\S]*mount_dir\/Synara\.app/,
  ) &&
  hasWorkflowPattern(
    workflow,
    /codesign\s+--verify[\s\S]*extract_dir\/Synara\.app/,
  );

const hasPackagedLocalhostRemoteCapability = (capabilities) =>
  (capabilities?.remote?.urls ?? []).some(
    (url) => url === "http://localhost:*" || url === "http://localhost:*/*",
  );

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
      "src-tauri/tauri.conf.json must set bundle.createUpdaterArtifacts to true.",
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
      "plugins.updater.pubkey must contain the production Tauri updater public key.",
    );
  } else if (hasPlaceholder(updaterConfig.pubkey)) {
    report("plugins.updater.pubkey still looks like placeholder material.");
  }

  if (!Array.isArray(endpoints) || endpoints.length === 0) {
    report(
      "plugins.updater.endpoints must contain at least one production HTTPS endpoint.",
    );
  } else {
    for (const endpoint of endpoints) {
      if (typeof endpoint !== "string" || endpoint.trim() === "") {
        report(
          "plugins.updater.endpoints must contain only non-empty strings.",
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
      "src-tauri/Cargo.toml must depend on tauri-plugin-updater for desktop targets.",
    );
  }

  if (
    !/tauri_plugin_updater::Builder|tauri_plugin_updater::init|plugin\s*\(\s*tauri_plugin_updater/m.test(
      rustLib,
    )
  ) {
    report("src-tauri/src/lib.rs must register the Tauri updater plugin.");
  }

  if (
    !permissions.some(
      (permission) =>
        permission === "updater:default" ||
        permission === "updater:allow-check",
    )
  ) {
    report(
      "src-tauri/capabilities/main.json must grant updater:allow-check or updater:default when the frontend owns update checks.",
    );
  }

  if (
    !permissions.some(
      (permission) =>
        permission === "updater:default" ||
        permission === "updater:allow-download-and-install",
    )
  ) {
    report(
      "src-tauri/capabilities/main.json must grant updater:allow-download-and-install or updater:default for macOS install prompts.",
    );
  }

  if (
    !permissions.some(
      (permission) =>
        permission === "process:default" ||
        permission === "process:allow-restart",
    )
  ) {
    report(
      "src-tauri/capabilities/main.json must grant process:allow-restart or process:default for post-update relaunch.",
    );
  }

  if (!packageDependencies["@tauri-apps/plugin-updater"]) {
    report(
      "package.json must depend on @tauri-apps/plugin-updater for frontend update checks.",
    );
  }

  if (!packageDependencies["@tauri-apps/plugin-process"]) {
    report(
      "package.json must depend on @tauri-apps/plugin-process for post-update relaunch.",
    );
  }

  if (!/^\s*tauri-plugin-process\s*=.+$/m.test(cargoToml)) {
    report(
      "src-tauri/Cargo.toml must depend on tauri-plugin-process for post-update relaunch.",
    );
  }

  if (
    !/tauri_plugin_process::init|plugin\s*\(\s*tauri_plugin_process/m.test(
      rustLib,
    )
  ) {
    report("src-tauri/src/lib.rs must register the Tauri process plugin.");
  }

  if (!hasPackagedLocalhostRemoteCapability(capabilities)) {
    errors.push(
      "src-tauri/capabilities/main.json must allow the packaged localhost webview origin with remote.urls containing http://localhost:*/*.",
    );
  }

  if (
    hasWorkflowPattern(
      releaseWorkflow,
      /createUpdaterArtifacts["']?\s*:\s*false/,
    )
  ) {
    errors.push(
      `${releaseWorkflowPath} still overrides bundle.createUpdaterArtifacts to false.`,
    );
  }

  if (
    updaterArtifacts !== true &&
    !workflowConfiguresUpdaterChannel(releaseWorkflow)
  ) {
    errors.push(
      `${releaseWorkflowPath} must configure the release updater channel before strict validation.`,
    );
  }

  if (!hasWorkflowPattern(releaseWorkflow, /TAURI_SIGNING_PRIVATE_KEY/)) {
    errors.push(
      `${releaseWorkflowPath} must expose TAURI_SIGNING_PRIVATE_KEY to release builds.`,
    );
  }

  if (!workflowConfiguresTauriNotarization(releaseWorkflow)) {
    errors.push(
      `${releaseWorkflowPath} must expose APPLE_ID, APPLE_APP_SPECIFIC_PASSWORD as APPLE_PASSWORD, and APPLE_TEAM_ID on the Tauri build step so it notarizes the app bundle.`,
    );
  }

  if (
    !hasWorkflowPattern(releaseWorkflow, /TAURI_SIGNING_PRIVATE_KEY_PASSWORD/)
  ) {
    errors.push(
      `${releaseWorkflowPath} must expose TAURI_SIGNING_PRIVATE_KEY_PASSWORD to release builds.`,
    );
  }

  if (!hasWorkflowPattern(releaseWorkflow, /\.sig/)) {
    errors.push(`${releaseWorkflowPath} must upload updater signature sidecars.`);
  }

  if (!workflowPublishesGeneratedUpdaterMetadata(releaseWorkflow)) {
    errors.push(
      `${releaseWorkflowPath} must generate and upload signed updater metadata from macOS updater artifacts.`,
    );
  }

  if (!workflowVerifiesMacosDistributableContents(releaseWorkflow)) {
    errors.push(
      `${releaseWorkflowPath} must verify the mounted macOS DMG app and extracted updater archive before publishing.`,
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
    releaseWorkflow: readText(releaseWorkflowPath),
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
      "[release-updater] updater release prerequisites are configured.",
    );
  } else {
    console.log(
      "[release-updater] release workflow contract is valid; runtime updater configuration remains disabled until release materialization.",
    );
  }
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  main();
}
