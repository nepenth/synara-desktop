import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const hasPlaceholder = (value) =>
  /CHANGE_ME|TODO|example\.(com|org|net)|localhost|127\.0\.0\.1|<[^>]+>|__[^_]+__/i.test(
    value
  );

const isHttpsUrl = (value) => {
  try {
    return new URL(value).protocol === "https:";
  } catch {
    return false;
  }
};

const defaultEndpointForRepo = (repository) =>
  repository
    ? `https://github.com/${repository}/releases/latest/download/latest.json`
    : "";

const normalizeEndpoints = ({ endpoint, repository }) => {
  const source = endpoint?.trim() || defaultEndpointForRepo(repository);
  return source
    .split(",")
    .map((value) => value.trim())
    .filter(Boolean);
};

export function buildReleaseUpdaterConfig({
  baseConfig,
  pubkey,
  endpoint,
  repository,
}) {
  const cleanPubkey = pubkey?.trim() ?? "";
  if (cleanPubkey.length < 40 || hasPlaceholder(cleanPubkey)) {
    throw new Error(
      "SYNARA_UPDATER_PUBKEY must contain the production Tauri updater public key."
    );
  }

  const endpoints = normalizeEndpoints({ endpoint, repository });
  if (endpoints.length === 0) {
    throw new Error(
      "SYNARA_UPDATER_ENDPOINT or GITHUB_REPOSITORY must provide a production updater endpoint."
    );
  }
  for (const candidate of endpoints) {
    if (!isHttpsUrl(candidate) || hasPlaceholder(candidate)) {
      throw new Error(`Invalid production updater endpoint: ${candidate}`);
    }
  }

  return {
    ...baseConfig,
    bundle: {
      ...(baseConfig.bundle ?? {}),
      createUpdaterArtifacts: true,
    },
    plugins: {
      ...(baseConfig.plugins ?? {}),
      updater: {
        pubkey: cleanPubkey,
        endpoints,
      },
    },
  };
}

function main() {
  const configPath = path.join(root, "src-tauri/tauri.conf.json");
  const baseConfig = JSON.parse(readFileSync(configPath, "utf8"));
  const nextConfig = buildReleaseUpdaterConfig({
    baseConfig,
    pubkey: process.env.SYNARA_UPDATER_PUBKEY,
    endpoint: process.env.SYNARA_UPDATER_ENDPOINT,
    repository: process.env.GITHUB_REPOSITORY,
  });
  writeFileSync(configPath, `${JSON.stringify(nextConfig, null, 2)}\n`);
  console.log("[release-updater] configured release updater channel.");
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  try {
    main();
  } catch (error) {
    console.error(`[release-updater] ${error.message}`);
    process.exit(1);
  }
}
