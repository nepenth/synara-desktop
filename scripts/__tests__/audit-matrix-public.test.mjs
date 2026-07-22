import assert from "node:assert/strict";
import test from "node:test";

import { auditMatrixPublic, parseArguments } from "../audit-matrix-public.mjs";

const jsonResponse = (value, status = 200, headers = {}) =>
  new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json", ...headers },
  });

const responseWithUrl = (body, status, url) => {
  const response = new Response(body, { status });
  Object.defineProperty(response, "url", { value: url });
  return response;
};

function fixtureFetch({
  version = "1.157.0",
  publicAdmin = false,
  publicRegistration = publicAdmin,
  federationAvailable = true,
  serverWellKnownBody,
  homeserverBaseUrl = "https://matrix.example.com",
} = {}) {
  return async (url) => {
    const route = new URL(url).pathname;
    switch (route) {
      case "/.well-known/matrix/client":
        return jsonResponse(
          {
            "m.homeserver": { base_url: homeserverBaseUrl },
            "m.authentication": { issuer: "https://auth.example.com/" },
          },
          200,
          { "access-control-allow-origin": "*" },
        );
      case "/.well-known/matrix/server":
        if (serverWellKnownBody !== undefined) {
          return jsonResponse(serverWellKnownBody);
        }
        return federationAvailable
          ? jsonResponse({ "m.server": "matrix.example.com:443" })
          : jsonResponse({ errcode: "M_NOT_FOUND" }, 404);
      case "/_matrix/client/versions":
        return jsonResponse({ versions: ["v1.15"] });
      case "/_matrix/federation/v1/version":
        return federationAvailable
          ? jsonResponse({ server: { name: "Synapse", version } })
          : jsonResponse({ errcode: "M_NOT_FOUND" }, 404);
      case "/_matrix/key/v2/server":
        return federationAvailable
          ? jsonResponse({ server_name: "matrix.example.com" })
          : jsonResponse({ errcode: "M_NOT_FOUND" }, 404);
      case "/_matrix/client/v3/login":
        return jsonResponse({ flows: [{ type: "m.login.sso" }] });
      case "/_synapse/admin/v1/server_version":
        return publicAdmin
          ? jsonResponse({ server_version: version })
          : jsonResponse({ errcode: "M_FORBIDDEN" }, 403);
      case "/_synapse/admin/v1/register":
        return publicRegistration
          ? jsonResponse({ nonce: "redacted" })
          : jsonResponse({ errcode: "M_FORBIDDEN" }, 403);
      case "/_synapse/metrics":
      case "/":
        return new Response("not found", { status: 404 });
      default:
        throw new Error(`Unexpected route ${route}`);
    }
  };
}

test("accepts healthy public discovery with protected operational endpoints", async () => {
  const report = await auditMatrixPublic({
    serverName: "matrix.example.com",
    minimumSynapseVersion: "1.157.0",
    fetchImplementation: fixtureFetch(),
  });
  assert.deepEqual(report.findings, []);
  assert.equal(report.implementation.version, "1.157.0");
});

test("flags an old Synapse and publicly routed Admin API", async () => {
  const report = await auditMatrixPublic({
    serverName: "matrix.example.com",
    minimumSynapseVersion: "1.157.0",
    fetchImplementation: fixtureFetch({
      version: "1.155.0",
      publicAdmin: true,
    }),
  });
  assert.deepEqual(
    report.findings
      .filter((finding) => finding.severity === "error")
      .map((finding) => finding.code),
    ["synapse_outdated", "admin_api_public", "admin_registration_public"],
  );
});

test("accepts an intentionally non-federated homeserver", async () => {
  const report = await auditMatrixPublic({
    serverName: "matrix.example.com",
    federationEnabled: false,
    fetchImplementation: fixtureFetch({ federationAvailable: false }),
  });

  assert.deepEqual(report.findings, []);
  assert.equal(report.federationExpected, false);
});

test("does not treat an empty server well-known document as federation delegation", async () => {
  const report = await auditMatrixPublic({
    serverName: "matrix.example.com",
    federationEnabled: false,
    fetchImplementation: fixtureFetch({
      federationAvailable: false,
      serverWellKnownBody: {},
    }),
  });

  assert.equal(
    report.findings.some(
      (finding) => finding.code === "federation_delegation_present",
    ),
    false,
  );
});

test("does not silently claim a version check on a non-federated public surface", async () => {
  const report = await auditMatrixPublic({
    serverName: "matrix.example.com",
    minimumSynapseVersion: "1.157.0",
    federationEnabled: false,
    fetchImplementation: fixtureFetch({ federationAvailable: false }),
  });

  assert.equal(
    report.findings.some(
      (finding) => finding.code === "synapse_version_unverifiable",
    ),
    true,
  );
});

test("flags federation when a non-federated deployment unexpectedly serves it", async () => {
  const report = await auditMatrixPublic({
    serverName: "matrix.example.com",
    federationEnabled: false,
    fetchImplementation: fixtureFetch(),
  });

  assert.equal(
    report.findings.some(
      (finding) => finding.code === "federation_unexpectedly_enabled",
    ),
    true,
  );
});

test("does not flag Synapse's built-in static landing-page redirect", async () => {
  const healthyFetch = fixtureFetch();
  const report = await auditMatrixPublic({
    serverName: "matrix.example.com",
    fetchImplementation: async (url, options) => {
      if (new URL(url).pathname === "/") {
        return responseWithUrl(
          "Synapse",
          200,
          "https://matrix.example.com/_matrix/static/",
        );
      }
      return healthyFetch(url, options);
    },
  });

  assert.equal(
    report.findings.some((finding) => finding.code === "shared_content_origin"),
    false,
  );
  assert.equal(report.endpoints.root.status, 200);
});

test("flags a selectively exposed shared-secret registration path", async () => {
  const report = await auditMatrixPublic({
    serverName: "matrix.example.com",
    fetchImplementation: fixtureFetch({ publicRegistration: true }),
  });
  assert.equal(
    report.findings.some(
      (finding) => finding.code === "admin_registration_public",
    ),
    true,
  );
});

test("flags mismatched homeserver discovery", async () => {
  const report = await auditMatrixPublic({
    serverName: "matrix.example.com",
    fetchImplementation: fixtureFetch({
      homeserverBaseUrl: "https://other.example.com",
    }),
  });
  assert.equal(
    report.findings.some(
      (finding) => finding.code === "homeserver_discovery_mismatch",
    ),
    true,
  );
});

test("flags a cross-origin operational redirect even when it ends in 404", async () => {
  const healthyFetch = fixtureFetch();
  const report = await auditMatrixPublic({
    serverName: "matrix.example.com",
    fetchImplementation: async (url, options) => {
      if (new URL(url).pathname === "/_synapse/admin/v1/server_version") {
        return responseWithUrl(
          "not found",
          404,
          "https://unrelated.example.net/not-found",
        );
      }
      return healthyFetch(url, options);
    },
  });
  assert.equal(
    report.findings.some(
      (finding) => finding.code === "admin_cross_origin_redirect",
    ),
    true,
  );
});

test("treats prerelease versions as older than the matching stable release", async () => {
  const report = await auditMatrixPublic({
    serverName: "matrix.example.com",
    minimumSynapseVersion: "1.157.0",
    fetchImplementation: fixtureFetch({ version: "1.157.0rc1" }),
  });
  assert.equal(
    report.findings.some((finding) => finding.code === "synapse_outdated"),
    true,
  );
});

test("reports a malformed server version without crashing", async () => {
  const report = await auditMatrixPublic({
    serverName: "matrix.example.com",
    minimumSynapseVersion: "1.157.0",
    fetchImplementation: fixtureFetch({ version: "not-a-version" }),
  });
  assert.equal(
    report.findings.some(
      (finding) => finding.code === "synapse_version_invalid",
    ),
    true,
  );
});

test("validates CLI option names and minimum-version values", () => {
  assert.throws(
    () => parseArguments(["--unknown", "matrix.example.com"]),
    /Unexpected argument/,
  );
  assert.throws(
    () => parseArguments(["matrix.example.com", "--minimum-synapse", "--json"]),
    /requires a version/,
  );
  assert.throws(
    () => parseArguments(["matrix.example.com", "--minimum-synapse", "latest"]),
    /Invalid Synapse version/,
  );
});

test("rejects unsafe audit targets", async () => {
  await assert.rejects(
    auditMatrixPublic({
      serverName: "https://user:secret@example.com/path",
      fetchImplementation: fixtureFetch(),
    }),
    /bare HTTPS host/,
  );
});

test("reports malformed public discovery instead of crashing", async () => {
  const healthyFetch = fixtureFetch();
  const report = await auditMatrixPublic({
    serverName: "matrix.example.com",
    minimumSynapseVersion: "1.157.0",
    fetchImplementation: async (url, options) => {
      if (new URL(url).pathname === "/.well-known/matrix/client") {
        return jsonResponse({
          "m.homeserver": { base_url: "not a URL" },
          "m.authentication": { issuer: "://invalid" },
          "m.identity_server": { base_url: "also invalid" },
        });
      }
      return healthyFetch(url, options);
    },
  });

  assert.deepEqual(
    report.findings
      .filter((finding) => finding.severity === "error")
      .map((finding) => finding.code),
    ["homeserver_discovery", "authentication_https", "identity_server_url"],
  );
});
