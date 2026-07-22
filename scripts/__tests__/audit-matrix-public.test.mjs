import assert from "node:assert/strict";
import test from "node:test";

import { auditMatrixPublic } from "../audit-matrix-public.mjs";

const jsonResponse = (value, status = 200, headers = {}) =>
  new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json", ...headers },
  });

function fixtureFetch({ version = "1.157.0", publicAdmin = false } = {}) {
  return async (url) => {
    const route = new URL(url).pathname;
    switch (route) {
      case "/.well-known/matrix/client":
        return jsonResponse(
          {
            "m.homeserver": { base_url: "https://matrix.example.com" },
            "m.authentication": { issuer: "https://auth.example.com/" },
          },
          200,
          { "access-control-allow-origin": "*" }
        );
      case "/.well-known/matrix/server":
        return jsonResponse({ "m.server": "matrix.example.com:443" });
      case "/_matrix/client/versions":
        return jsonResponse({ versions: ["v1.15"] });
      case "/_matrix/federation/v1/version":
        return jsonResponse({ server: { name: "Synapse", version } });
      case "/_matrix/client/v3/login":
        return jsonResponse({ flows: [{ type: "m.login.sso" }] });
      case "/_synapse/admin/v1/server_version":
        return publicAdmin
          ? jsonResponse({ server_version: version })
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
    ["synapse_outdated", "admin_api_public"]
  );
});

test("rejects unsafe audit targets", async () => {
  await assert.rejects(
    auditMatrixPublic({
      serverName: "https://user:secret@example.com/path",
      fetchImplementation: fixtureFetch(),
    }),
    /bare HTTPS host/
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
    ["homeserver_discovery", "authentication_https", "identity_server_url"]
  );
});
