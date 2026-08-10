import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const PRODUCTION_VERSION = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const APP_TAG = /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;

export class ReleaseVersionPolicyError extends Error {
  constructor(message) {
    super(message);
    this.name = "ReleaseVersionPolicyError";
  }
}

/**
 * Production releases intentionally accept only a canonical final SemVer.
 * Build metadata has equal SemVer precedence and prereleases use a separate
 * lane, so neither can reserve or publish through this production workflow.
 */
export function parseProductionVersion(version) {
  if (typeof version !== "string") {
    throw new ReleaseVersionPolicyError("Production version must be a string.");
  }
  const match = version.match(PRODUCTION_VERSION);
  if (!match) {
    throw new ReleaseVersionPolicyError(
      `Production version ${JSON.stringify(
        version
      )} must be canonical X.Y.Z (no prerelease or build metadata).`
    );
  }
  return {
    raw: version,
    major: BigInt(match[1]),
    minor: BigInt(match[2]),
    patch: BigInt(match[3]),
  };
}

export function compareProductionVersions(left, right) {
  const a = typeof left === "string" ? parseProductionVersion(left) : left;
  const b = typeof right === "string" ? parseProductionVersion(right) : right;
  for (const field of ["major", "minor", "patch"]) {
    if (a[field] < b[field]) return -1;
    if (a[field] > b[field]) return 1;
  }
  return 0;
}

export function parseApplicationRelease(record) {
  if (
    !record ||
    typeof record !== "object" ||
    typeof record.tag_name !== "string"
  ) {
    return undefined;
  }
  const match = record.tag_name.match(APP_TAG);
  if (!match) return undefined;
  const version = `${match[1]}.${match[2]}.${match[3]}`;
  return {
    tag: record.tag_name,
    version: parseProductionVersion(version),
    draft: record.draft === true,
    prerelease: record.prerelease === true,
  };
}

const positiveRunAttempt = (value) => {
  const parsed = Number.parseInt(String(value), 10);
  if (!Number.isSafeInteger(parsed) || parsed < 1) {
    throw new ReleaseVersionPolicyError(
      "GITHUB_RUN_ATTEMPT must be a positive integer."
    );
  }
  return parsed;
};

/**
 * Decide whether this workflow invocation may create a release, or is a retry
 * of the exact Actions run. The caller separately proves the immutable remote
 * tag still peels to this run's commit.
 */
export function evaluateReleaseVersion({
  version,
  tag,
  releases,
  runAttempt = 1,
}) {
  const candidate = parseProductionVersion(version);
  if (tag !== `v${candidate.raw}`) {
    throw new ReleaseVersionPolicyError(
      `Release tag ${JSON.stringify(tag)} must exactly equal v${candidate.raw}.`
    );
  }
  if (!Array.isArray(releases)) {
    throw new ReleaseVersionPolicyError(
      "GitHub release ledger must be an array."
    );
  }

  const attempt = positiveRunAttempt(runAttempt);
  const applications = releases.map(parseApplicationRelease).filter(Boolean);
  const sameVersion = applications.filter(
    (release) => compareProductionVersions(release.version, candidate) === 0
  );
  const published = applications.filter((release) => !release.draft);
  const maxPublished = published.reduce(
    (maximum, release) =>
      !maximum ||
      compareProductionVersions(release.version, maximum.version) > 0
        ? release
        : maximum,
    undefined
  );

  if (sameVersion.length > 0) {
    if (attempt === 1) {
      const state = sameVersion.some((release) => !release.draft)
        ? "already published"
        : "reserved by a draft";
      throw new ReleaseVersionPolicyError(
        `Release version ${candidate.raw} is ${state}; a fresh production release may not reuse it.`
      );
    }
    return {
      mode: sameVersion.some((release) => !release.draft)
        ? "resume-published"
        : "resume-draft",
      version: candidate.raw,
      maxPublishedVersion: maxPublished?.version.raw,
    };
  }

  if (
    maxPublished &&
    compareProductionVersions(candidate, maxPublished.version) <= 0
  ) {
    throw new ReleaseVersionPolicyError(
      `Release version ${candidate.raw} must be greater than already published ${maxPublished.version.raw}.`
    );
  }

  return {
    mode: "create",
    version: candidate.raw,
    maxPublishedVersion: maxPublished?.version.raw,
  };
}

export function assertImmutableTagPush(event, tag) {
  if (!event || typeof event !== "object") {
    throw new ReleaseVersionPolicyError(
      "Release workflow requires its GitHub push event payload."
    );
  }
  if (event.ref_type !== "tag") {
    throw new ReleaseVersionPolicyError(
      "Production releases may run only for a tag push."
    );
  }
  if (
    event.created !== true ||
    event.deleted === true ||
    event.forced === true
  ) {
    throw new ReleaseVersionPolicyError(
      "Release tags must be freshly created; deleted, recreated, or force-updated tags are rejected."
    );
  }
  if (typeof event.ref === "string" && event.ref !== tag) {
    throw new ReleaseVersionPolicyError(
      `Push event tag ${JSON.stringify(
        event.ref
      )} does not match ${JSON.stringify(tag)}.`
    );
  }
}

const parseNextLink = (link) => {
  if (!link) return undefined;
  const match = link.match(/<([^>]+)>;\s*rel="next"/);
  return match?.[1];
};

/** Fetch every GitHub Release page: drafts and prereleases are ledger entries too. */
export async function fetchGitHubReleaseLedger({
  repository,
  token,
  fetchImpl = fetch,
}) {
  if (
    typeof repository !== "string" ||
    !/^[^/\s]+\/[^/\s]+$/.test(repository)
  ) {
    throw new ReleaseVersionPolicyError(
      "GITHUB_REPOSITORY must be owner/name."
    );
  }
  if (typeof token !== "string" || token.length === 0) {
    throw new ReleaseVersionPolicyError(
      "Release ledger lookup requires GH_TOKEN."
    );
  }

  const releases = [];
  const seenUrls = new Set();
  let next = `https://api.github.com/repos/${repository}/releases?per_page=100&page=1`;
  while (next) {
    if (seenUrls.has(next)) {
      throw new ReleaseVersionPolicyError(
        "GitHub release pagination loop detected."
      );
    }
    seenUrls.add(next);
    const response = await fetchImpl(next, {
      headers: {
        Accept: "application/vnd.github+json",
        Authorization: `Bearer ${token}`,
        "X-GitHub-Api-Version": "2022-11-28",
      },
    });
    if (!response.ok) {
      throw new ReleaseVersionPolicyError(
        `Unable to read GitHub release ledger (${response.status} ${response.statusText}).`
      );
    }
    const page = await response.json();
    if (!Array.isArray(page)) {
      throw new ReleaseVersionPolicyError(
        "GitHub release ledger page was not an array."
      );
    }
    releases.push(...page);
    next = parseNextLink(response.headers.get("link"));
  }
  return releases;
}

const readPushEvent = (eventPath) => {
  if (!eventPath) return undefined;
  try {
    return JSON.parse(readFileSync(eventPath, "utf8"));
  } catch (error) {
    throw new ReleaseVersionPolicyError(
      `Unable to read GitHub push event payload: ${
        error instanceof Error ? error.message : String(error)
      }`
    );
  }
};

const git = (args) =>
  execFileSync("git", args, {
    cwd: root,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();

/** Resolve lightweight and annotated tags to the commit they actually name. */
export function remotePeeledTagCommit(tag, runSha, execFile = git) {
  const ref = `refs/tags/${tag}`;
  let output;
  try {
    output = execFile(["ls-remote", "origin", ref, `${ref}^{}`]);
  } catch (error) {
    throw new ReleaseVersionPolicyError(
      `Unable to resolve remote release tag ${tag}: ${
        error instanceof Error ? error.message : String(error)
      }`
    );
  }
  const refs = new Map(
    output
      .split(/\r?\n/)
      .filter(Boolean)
      .map((line) => line.split(/\s+/))
      .filter(
        ([sha, name]) => /^[0-9a-f]{40}$/i.test(sha) && typeof name === "string"
      )
      .map(([sha, name]) => [name, sha])
  );
  const remote = refs.get(`${ref}^{}`) ?? refs.get(ref);
  if (!remote) {
    throw new ReleaseVersionPolicyError(
      `Remote release tag ${tag} does not exist.`
    );
  }

  let expected;
  try {
    expected = execFile(["rev-parse", `${runSha}^{commit}`]);
  } catch (error) {
    throw new ReleaseVersionPolicyError(
      `Unable to resolve release run commit ${runSha}: ${
        error instanceof Error ? error.message : String(error)
      }`
    );
  }
  if (remote !== expected) {
    throw new ReleaseVersionPolicyError(
      `Remote release tag ${tag} resolves to ${remote}, not this run's commit ${expected}.`
    );
  }
  return remote;
}

const writeOutput = (name, value) => {
  const outputPath = process.env.GITHUB_OUTPUT;
  if (outputPath) {
    writeFileSync(outputPath, `${name}=${value}\n`, {
      encoding: "utf8",
      flag: "a",
    });
  }
};

export async function main({ env = process.env } = {}) {
  const packageJson = JSON.parse(
    readFileSync(path.join(root, "package.json"), "utf8")
  );
  const version = packageJson.version;
  const tag = env.GITHUB_REF_NAME;
  const repository = env.GITHUB_REPOSITORY;
  const token = env.GH_TOKEN ?? env.GITHUB_TOKEN;
  const runSha = env.GITHUB_SHA;

  if (!tag || !repository || !runSha) {
    throw new ReleaseVersionPolicyError(
      "Release version guard requires GITHUB_REF_NAME, GITHUB_REPOSITORY, and GITHUB_SHA."
    );
  }
  if (env.GITHUB_EVENT_NAME !== "push") {
    throw new ReleaseVersionPolicyError(
      "Production release guard may run only for a push event."
    );
  }

  assertImmutableTagPush(readPushEvent(env.GITHUB_EVENT_PATH), tag);
  remotePeeledTagCommit(tag, runSha);
  const releases = await fetchGitHubReleaseLedger({ repository, token });
  const decision = evaluateReleaseVersion({
    version,
    tag,
    releases,
    runAttempt: env.GITHUB_RUN_ATTEMPT ?? "1",
  });
  writeOutput("release_mode", decision.mode);
  writeOutput("release_version", decision.version);
  console.log(
    `[release-version] ${decision.mode}: ${decision.version} (max published: ${
      decision.maxPublishedVersion ?? "none"
    })`
  );
  return decision;
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  main().catch((error) => {
    console.error(
      `[release-version] ${
        error instanceof Error ? error.message : String(error)
      }`
    );
    process.exitCode = 1;
  });
}
