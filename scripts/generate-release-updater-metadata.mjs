import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const LINUX_PLATFORM = "linux-x86_64";
const MACOS_PLATFORMS = ["darwin-x86_64", "darwin-aarch64"];
const ALL_PLATFORMS = [LINUX_PLATFORM, ...MACOS_PLATFORMS];
const DEFAULT_REQUIRED_PLATFORMS = MACOS_PLATFORMS;

const parseArgs = (argv) => {
  const args = {};
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (!value.startsWith("--")) {
      throw new Error(`Unexpected argument: ${value}`);
    }
    const key = value.slice(2);
    const next = argv[index + 1];
    if (!next || next.startsWith("--")) {
      throw new Error(`Missing value for --${key}`);
    }
    args[key] = next;
    index += 1;
  }
  return args;
};

const walk = (directory) => {
  const entries = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const fullPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      entries.push(...walk(fullPath));
    } else {
      entries.push(fullPath);
    }
  }
  return entries;
};

const releaseAssetUrl = ({ repo, tag, assetName }) =>
  `https://github.com/${repo}/releases/download/${encodeURIComponent(tag)}/${encodeURIComponent(assetName)}`;

const classifyArtifact = (artifactPath) => {
  const normalized = artifactPath.split(path.sep).join("/");
  if (normalized.includes("/appimage/") || normalized.includes("linux")) {
    return [LINUX_PLATFORM];
  }
  if (normalized.includes("/macos/") || normalized.includes("darwin")) {
    return MACOS_PLATFORMS;
  }
  return [];
};

const readSignature = (artifactPath) => {
  const signaturePath = `${artifactPath}.sig`;
  if (!existsSync(signaturePath)) {
    throw new Error(`Missing updater signature sidecar: ${signaturePath}`);
  }
  const signature = readFileSync(signaturePath, "utf8").trim();
  if (!signature) {
    throw new Error(`Updater signature sidecar is empty: ${signaturePath}`);
  }
  return signature;
};

export function generateReleaseUpdaterMetadata({
  artifactsDir,
  repo,
  tag,
  version,
  pubDate = new Date().toISOString(),
  requiredPlatforms = DEFAULT_REQUIRED_PLATFORMS,
}) {
  if (!repo || !/^[^/]+\/[^/]+$/.test(repo)) {
    throw new Error("repo must be in owner/name format");
  }
  if (!tag) {
    throw new Error("tag is required");
  }
  if (!version) {
    throw new Error("version is required");
  }
  if (!existsSync(artifactsDir)) {
    throw new Error(`Artifacts directory does not exist: ${artifactsDir}`);
  }
  for (const platform of requiredPlatforms) {
    if (!ALL_PLATFORMS.includes(platform)) {
      throw new Error(`Unsupported required updater platform: ${platform}`);
    }
  }

  const updaterArchives = walk(artifactsDir).filter(
    (filePath) => filePath.endsWith(".tar.gz") && !filePath.endsWith(".sig")
  );
  const platforms = {};

  for (const artifactPath of updaterArchives) {
    const platformNames = classifyArtifact(artifactPath);
    if (platformNames.length === 0) {
      continue;
    }
    const signature = readSignature(artifactPath);
    const url = releaseAssetUrl({
      repo,
      tag,
      assetName: path.basename(artifactPath),
    });

    for (const platformName of platformNames) {
      if (platforms[platformName]) {
        throw new Error(`Duplicate updater artifact for ${platformName}`);
      }
      platforms[platformName] = {
        signature,
        url,
      };
    }
  }

  for (const requiredPlatform of requiredPlatforms) {
    if (!platforms[requiredPlatform]) {
      throw new Error(`Missing updater metadata for ${requiredPlatform}`);
    }
  }

  return {
    version,
    pub_date: pubDate,
    platforms,
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const output = args.output ?? "latest.json";
  const metadata = generateReleaseUpdaterMetadata({
    artifactsDir: path.resolve(args.artifacts),
    repo: args.repo,
    tag: args.tag,
    version: args.version,
    pubDate: args["pub-date"],
    requiredPlatforms: args["required-platforms"]
      ? args["required-platforms"].split(",").map((value) => value.trim()).filter(Boolean)
      : undefined,
  });
  writeFileSync(output, `${JSON.stringify(metadata, null, 2)}\n`);
  console.log(`[updater-metadata] wrote ${output}`);
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  try {
    main();
  } catch (error) {
    console.error(`[updater-metadata] ${error.message}`);
    process.exit(1);
  }
}
