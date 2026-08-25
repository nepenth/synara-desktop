import assert from "node:assert/strict";
import test from "node:test";

import {
  ReleaseVersionPolicyError,
  assertImmutableTagPush,
  compareProductionVersions,
  evaluateReleaseVersion,
  fetchGitHubReleaseLedger,
  parseApplicationRelease,
  parseProductionVersion,
} from "../assert-release-version.mjs";

const release = (tag_name, extra = {}) => ({
  tag_name,
  draft: false,
  prerelease: false,
  ...extra,
});

const policyError = (callback, message) =>
  assert.throws(
    callback,
    (error) =>
      error instanceof ReleaseVersionPolicyError && message.test(error.message)
  );

test("production versions are strict final SemVer and compare numerically", () => {
  assert.equal(parseProductionVersion("2.0.0").raw, "2.0.0");
  assert.equal(compareProductionVersions("1.2.10", "1.2.9"), 1);
  assert.equal(
    compareProductionVersions("9007199254740993.0.0", "9007199254740992.99.99"),
    1
  );
  for (const invalid of [
    "01.2.3",
    "1.02.3",
    "1.2.03",
    "1.2",
    "v1.2.3",
    "1.2.3-rc.1",
    "1.2.3+build",
  ]) {
    policyError(() => parseProductionVersion(invalid), /canonical X\.Y\.Z/);
  }
});

test("fresh higher versions may create while lower or exact published versions cannot", () => {
  const ledger = [release("v1.2.9"), release("v1.2.10")];
  assert.deepEqual(
    evaluateReleaseVersion({
      version: "1.2.11",
      tag: "v1.2.11",
      releases: ledger,
    }),
    {
      mode: "create",
      version: "1.2.11",
      maxPublishedVersion: "1.2.10",
    }
  );
  policyError(
    () =>
      evaluateReleaseVersion({
        version: "1.2.10",
        tag: "v1.2.10",
        releases: ledger,
      }),
    /already published/
  );
  policyError(
    () =>
      evaluateReleaseVersion({
        version: "1.2.8",
        tag: "v1.2.8",
        releases: ledger,
      }),
    /must be greater/
  );
  policyError(
    () =>
      evaluateReleaseVersion({
        version: "1.2.11",
        tag: "v1.2.12",
        releases: ledger,
      }),
    /must exactly equal/
  );
});

test("a GitHub-prerelease-flagged stable tag consumes its production version", () => {
  const ledger = [release("v2.0.0", { prerelease: true })];
  policyError(
    () =>
      evaluateReleaseVersion({
        version: "2.0.0",
        tag: "v2.0.0",
        releases: ledger,
      }),
    /already published/
  );
  policyError(
    () =>
      evaluateReleaseVersion({
        version: "1.9.99",
        tag: "v1.9.99",
        releases: ledger,
      }),
    /must be greater/
  );
});

test("drafts reserve exact versions but same-run retries can resume only the same release", () => {
  const drafts = [release("v2.0.0", { draft: true })];
  policyError(
    () =>
      evaluateReleaseVersion({
        version: "2.0.0",
        tag: "v2.0.0",
        releases: drafts,
      }),
    /reserved by a draft/
  );
  assert.equal(
    evaluateReleaseVersion({
      version: "2.0.0",
      tag: "v2.0.0",
      releases: drafts,
      runAttempt: 2,
    }).mode,
    "resume-draft"
  );
  assert.equal(
    evaluateReleaseVersion({
      version: "2.0.0",
      tag: "v2.0.0",
      releases: [release("v2.0.0")],
      runAttempt: 2,
    }).mode,
    "resume-published"
  );
});

test("fixed and malformed non-app release tags never influence the application ledger", () => {
  const ledger = [
    release("pacman-repo"),
    release("apt-repo"),
    release("vbad"),
    release("v1.2.3-rc.1"),
  ];
  assert.deepEqual(
    evaluateReleaseVersion({
      version: "1.0.0",
      tag: "v1.0.0",
      releases: ledger,
    }),
    {
      mode: "create",
      version: "1.0.0",
      maxPublishedVersion: undefined,
    }
  );
  assert.equal(parseApplicationRelease(release("pacman-repo")), undefined);
  assert.equal(parseApplicationRelease(release("apt-repo")), undefined);
});

test("all GitHub Release pages are collected, including a later duplicate", async () => {
  const calls = [];
  const fetchImpl = async (url, init) => {
    calls.push({ url, init });
    if (calls.length === 1) {
      return new Response(JSON.stringify([release("v1.2.59")]), {
        status: 200,
        headers: {
          link: '<https://api.github.com/repos/acme/synara/releases?per_page=100&page=2>; rel="next"',
        },
      });
    }
    return new Response(JSON.stringify([release("v2.0.0", { draft: true })]), {
      status: 200,
    });
  };
  const ledger = await fetchGitHubReleaseLedger({
    repository: "acme/synara",
    token: "test-token",
    fetchImpl,
  });
  assert.equal(calls.length, 2);
  assert.equal(calls[0].init.headers.Authorization, "Bearer test-token");
  policyError(
    () =>
      evaluateReleaseVersion({
        version: "2.0.0",
        tag: "v2.0.0",
        releases: ledger,
      }),
    /reserved by a draft/
  );
});

test("tag push event must be a fresh immutable tag creation", () => {
  // This is the real GitHub `push` payload shape. `ref_type` is a create-event
  // field and is absent for a normal tag push.
  const valid = {
    ref: "refs/tags/v2.0.0",
    created: true,
    deleted: false,
    forced: false,
  };
  assert.doesNotThrow(() => assertImmutableTagPush(valid, "v2.0.0"));
  assert.doesNotThrow(() =>
    assertImmutableTagPush({ ...valid, ref_type: "tag" }, "v2.0.0")
  );
  for (const event of [
    { ...valid, created: false },
    { ...valid, deleted: true },
    { ...valid, forced: true },
    { ...valid, ref: "refs/tags/v2.0.1" },
    { ...valid, ref: "v2.0.0" },
    { ...valid, ref: "refs/heads/main" },
    { ...valid, ref_type: "branch" },
  ]) {
    policyError(
      () => assertImmutableTagPush(event, "v2.0.0"),
      /tag|push|created/i
    );
  }
});
