import { pathToFileURL } from "node:url";

const REQUEST_TIMEOUT_MS = 10_000;

function normalizeServerName(value) {
  const candidate = value.includes("://") ? value : `https://${value}`;
  const url = new URL(candidate);
  if (
    url.protocol !== "https:" ||
    url.username ||
    url.password ||
    url.pathname !== "/" ||
    url.search ||
    url.hash
  ) {
    throw new Error(
      "Server name must be a bare HTTPS host with no credentials or path.",
    );
  }
  return url.host;
}

const versionParts = (version) => {
  const match = /^v?(\d+)\.(\d+)\.(\d+)(?:(rc|a|b)(\d+))?$/.exec(version);
  if (!match) {
    throw new Error(`Invalid Synapse version: ${version}`);
  }
  const prereleaseRank =
    match[4] === undefined ? 3 : { a: 0, b: 1, rc: 2 }[match[4]];
  return [
    Number.parseInt(match[1], 10),
    Number.parseInt(match[2], 10),
    Number.parseInt(match[3], 10),
    prereleaseRank,
    Number.parseInt(match[5] ?? "0", 10),
  ];
};

function compareVersions(left, right) {
  const leftParts = versionParts(left);
  const rightParts = versionParts(right);
  for (
    let index = 0;
    index < Math.max(leftParts.length, rightParts.length);
    index += 1
  ) {
    const difference = (leftParts[index] ?? 0) - (rightParts[index] ?? 0);
    if (difference !== 0) return Math.sign(difference);
  }
  return 0;
}

async function request(fetchImplementation, url) {
  const startedAt = performance.now();
  try {
    const response = await fetchImplementation(url, {
      redirect: "follow",
      signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
    });
    const text = await response.text();
    let json;
    try {
      json = JSON.parse(text);
    } catch {
      json = undefined;
    }
    return {
      ok: response.ok,
      status: response.status,
      finalUrl: response.url || url,
      elapsedMs: Math.round(performance.now() - startedAt),
      allowOrigin: response.headers.get("access-control-allow-origin"),
      json,
    };
  } catch (error) {
    return {
      ok: false,
      status: 0,
      elapsedMs: Math.round(performance.now() - startedAt),
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

export async function auditMatrixPublic({
  serverName,
  minimumSynapseVersion,
  federationEnabled = true,
  fetchImplementation = fetch,
}) {
  const host = normalizeServerName(serverName);
  if (minimumSynapseVersion) versionParts(minimumSynapseVersion);
  const baseUrl = `https://${host}`;
  const routes = {
    clientWellKnown: "/.well-known/matrix/client",
    serverWellKnown: "/.well-known/matrix/server",
    clientVersions: "/_matrix/client/versions",
    federationVersion: "/_matrix/federation/v1/version",
    federationKey: "/_matrix/key/v2/server",
    login: "/_matrix/client/v3/login",
    admin: "/_synapse/admin/v1/server_version",
    adminRegistration: "/_synapse/admin/v1/register",
    metrics: "/_synapse/metrics",
    root: "/",
  };
  const entries = await Promise.all(
    Object.entries(routes).map(async ([name, route]) => [
      name,
      await request(fetchImplementation, `${baseUrl}${route}`),
    ]),
  );
  const endpoints = Object.fromEntries(entries);
  const findings = [];
  const finding = (severity, code, message) =>
    findings.push({ severity, code, message });

  for (const [name, result] of Object.entries(endpoints)) {
    if (!result.finalUrl) continue;
    let finalOrigin;
    try {
      finalOrigin = new URL(result.finalUrl).origin;
    } catch {
      finalOrigin = undefined;
    }
    if (finalOrigin && finalOrigin !== baseUrl) {
      finding(
        "error",
        `${name}_cross_origin_redirect`,
        `${name} redirected to a different origin (${finalOrigin}).`,
      );
    }
  }

  if (!federationEnabled && minimumSynapseVersion) {
    finding(
      "warning",
      "synapse_version_unverifiable",
      "A public non-federated audit cannot verify the Synapse version; compare the installed package or image revision on the server.",
    );
  }

  const requiredEndpoints = ["clientWellKnown", "clientVersions", "login"];
  if (federationEnabled) {
    requiredEndpoints.push(
      "serverWellKnown",
      "federationVersion",
      "federationKey",
    );
  }
  for (const name of requiredEndpoints) {
    if (!endpoints[name].ok) {
      finding(
        "error",
        `${name}_unavailable`,
        `${name} returned HTTP ${endpoints[name].status}.`,
      );
    }
  }

  if (endpoints.clientWellKnown.allowOrigin !== "*") {
    finding(
      "warning",
      "client_well_known_cors",
      "Client discovery does not advertise Access-Control-Allow-Origin: *.",
    );
  }
  const homeserverUrl =
    endpoints.clientWellKnown.json?.["m.homeserver"]?.base_url;
  let discoveredHomeserver;
  try {
    discoveredHomeserver = new URL(homeserverUrl);
  } catch {
    discoveredHomeserver = undefined;
  }
  if (discoveredHomeserver?.protocol !== "https:") {
    finding(
      "error",
      "homeserver_discovery",
      "Client discovery lacks an HTTPS homeserver URL.",
    );
  } else if (discoveredHomeserver.origin !== baseUrl) {
    finding(
      "error",
      "homeserver_discovery_mismatch",
      `Client discovery points to ${discoveredHomeserver.origin}, not the audited origin ${baseUrl}.`,
    );
  }

  const serverDelegation = endpoints.serverWellKnown.json?.["m.server"];
  if (federationEnabled && typeof serverDelegation !== "string") {
    finding(
      "error",
      "federation_discovery",
      "Server discovery lacks m.server delegation.",
    );
  } else if (!federationEnabled && typeof serverDelegation === "string") {
    finding(
      "error",
      "federation_delegation_present",
      "Server well-known still advertises federation even though this audit expects federation to be disabled.",
    );
  }

  const implementation = endpoints.federationVersion.json?.server;
  if (!federationEnabled && endpoints.federationVersion.ok) {
    finding(
      "error",
      "federation_unexpectedly_enabled",
      "Federation responds successfully even though this audit expects it to be disabled.",
    );
  } else if (federationEnabled && implementation?.name !== "Synapse") {
    finding(
      "warning",
      "implementation",
      "Federation does not report Synapse as the implementation.",
    );
  }
  if (!federationEnabled && endpoints.federationKey.ok) {
    finding(
      "error",
      "federation_key_public",
      "The federation signing-key endpoint responds successfully even though federation should be disabled.",
    );
  }
  if (
    federationEnabled &&
    minimumSynapseVersion &&
    typeof implementation?.version !== "string"
  ) {
    finding(
      "error",
      "synapse_version_missing",
      "Federation did not report a Synapse version to compare.",
    );
  } else if (federationEnabled && minimumSynapseVersion) {
    try {
      if (compareVersions(implementation.version, minimumSynapseVersion) < 0) {
        finding(
          "error",
          "synapse_outdated",
          `Synapse ${implementation.version} is below required ${minimumSynapseVersion}.`,
        );
      }
    } catch {
      finding(
        "error",
        "synapse_version_invalid",
        `Federation reported an invalid Synapse version (${implementation.version}).`,
      );
    }
  }

  if (![403, 404].includes(endpoints.admin.status)) {
    finding(
      "error",
      "admin_api_public",
      `The Synapse Admin API is not blocked by the proxy (HTTP ${endpoints.admin.status}).`,
    );
  }
  if (![403, 404].includes(endpoints.adminRegistration.status)) {
    finding(
      "error",
      "admin_registration_public",
      `The shared-secret registration path is not blocked by the proxy (HTTP ${endpoints.adminRegistration.status}).`,
    );
  }
  if (![403, 404].includes(endpoints.metrics.status)) {
    finding(
      "error",
      "metrics_public",
      `Synapse metrics are publicly reachable (HTTP ${endpoints.metrics.status}).`,
    );
  }

  const loginFlows =
    endpoints.login.json?.flows?.map((flow) => flow.type) ?? [];
  if (loginFlows.length === 0) {
    finding("error", "login_flows", "No Matrix login flows were advertised.");
  }

  const clientWellKnown = endpoints.clientWellKnown.json ?? {};
  const authentication =
    clientWellKnown["m.authentication"] ??
    clientWellKnown["org.matrix.msc2965.authentication"];
  if (
    clientWellKnown["org.matrix.msc2965.authentication"] &&
    !clientWellKnown["m.authentication"]
  ) {
    finding(
      "warning",
      "unstable_auth_discovery",
      "Authentication discovery still uses the pre-stable org.matrix.msc2965 key.",
    );
  }
  if (authentication?.issuer) {
    let issuer;
    try {
      issuer = new URL(authentication.issuer);
    } catch {
      issuer = undefined;
    }
    if (issuer?.protocol !== "https:") {
      finding(
        "error",
        "authentication_https",
        "Authentication discovery must use HTTPS.",
      );
    }
  }

  const identityServer = clientWellKnown["m.identity_server"]?.base_url;
  if (typeof identityServer === "string") {
    let identityUrl;
    try {
      identityUrl = new URL(identityServer);
    } catch {
      finding(
        "error",
        "identity_server_url",
        "Client discovery contains an invalid identity-service URL.",
      );
    }
    if (identityUrl && identityUrl.hostname !== new URL(baseUrl).hostname) {
      finding(
        "warning",
        "external_identity_server",
        `Client discovery delegates identity-service traffic to ${identityUrl.hostname}; confirm this is intentional.`,
      );
    }
  }
  const rootFinalPath = (() => {
    try {
      return new URL(endpoints.root.finalUrl).pathname;
    } catch {
      return undefined;
    }
  })();
  if (endpoints.root.ok && !rootFinalPath?.startsWith("/_matrix/static")) {
    finding(
      "warning",
      "shared_content_origin",
      "The homeserver origin also serves a website; verify it holds no sensitive application state or broadly scoped cookies.",
    );
  }

  return {
    auditedAt: new Date().toISOString(),
    serverName: host,
    federationExpected: federationEnabled,
    implementation,
    supportedClientVersions: endpoints.clientVersions.json?.versions ?? [],
    loginFlows,
    endpoints: Object.fromEntries(
      Object.entries(endpoints).map(([name, result]) => [
        name,
        {
          status: result.status,
          ...(result.finalUrl ? { finalUrl: result.finalUrl } : {}),
          elapsedMs: result.elapsedMs,
          ...(result.error ? { error: result.error } : {}),
        },
      ]),
    ),
    findings,
  };
}

export function parseArguments(argumentsList) {
  const options = {
    serverName: undefined,
    minimumSynapseVersion: undefined,
    federationEnabled: true,
    json: false,
  };
  for (let index = 0; index < argumentsList.length; index += 1) {
    const argument = argumentsList[index];
    if (argument === "--minimum-synapse") {
      const value = argumentsList[++index];
      if (!value || value.startsWith("--")) {
        throw new Error("--minimum-synapse requires a version value.");
      }
      versionParts(value);
      options.minimumSynapseVersion = value;
    } else if (argument === "--no-federation") {
      options.federationEnabled = false;
    } else if (argument === "--json") {
      options.json = true;
    } else if (argument.startsWith("--")) {
      throw new Error(`Unexpected argument: ${argument}`);
    } else if (!options.serverName) {
      options.serverName = argument;
    } else {
      throw new Error(`Unexpected argument: ${argument}`);
    }
  }
  if (!options.serverName) {
    throw new Error(
      "Usage: node scripts/audit-matrix-public.mjs <server-name> [--minimum-synapse X.Y.Z] [--no-federation] [--json]",
    );
  }
  return options;
}

async function main() {
  const options = parseArguments(process.argv.slice(2));
  const report = await auditMatrixPublic(options);
  if (options.json) {
    console.log(JSON.stringify(report, null, 2));
  } else {
    console.log(
      `${report.serverName}: ${report.implementation?.name ?? "unknown"} ${
        report.implementation?.version ?? "unknown"
      }`,
    );
    for (const item of report.findings) {
      console.log(`[${item.severity}] ${item.code}: ${item.message}`);
    }
  }
  if (report.findings.some((item) => item.severity === "error"))
    process.exit(1);
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(2);
  });
}
