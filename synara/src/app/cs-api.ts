import { trimTrailingSlash } from './utils/common';

export enum AutoDiscoveryAction {
  PROMPT = 'PROMPT',
  IGNORE = 'IGNORE',
  FAIL_PROMPT = 'FAIL_PROMPT',
  FAIL_ERROR = 'FAIL_ERROR',
}

export type AutoDiscoveryError = {
  host: string;
  action: AutoDiscoveryAction;
};

export type AutoDiscoveryInfo = Record<string, unknown> & {
  'm.homeserver': {
    base_url: string;
  };
  'm.identity_server'?: {
    base_url: string;
  };
  'org.matrix.msc2965.authentication'?: {
    account?: string;
    issuer?: string;
  };
  'org.matrix.msc4143.rtc_foci'?: [
    {
      livekit_service_url: string;
      type: 'livekit';
    }
  ];
};

const isLoopbackHostname = (hostname: string): boolean =>
  hostname === 'localhost' ||
  hostname === '127.0.0.1' ||
  hostname === '[::1]' ||
  hostname === '::1';

/** Remote homeservers must use TLS; plain HTTP is reserved for local development. */
export const normalizeSecureHomeserverUrl = (value: string): string | undefined => {
  try {
    const parsed = new URL(value.trim());
    if (
      !parsed.hostname ||
      parsed.username ||
      parsed.password ||
      parsed.search ||
      parsed.hash ||
      (parsed.protocol !== 'https:' &&
        !(parsed.protocol === 'http:' && isLoopbackHostname(parsed.hostname)))
    ) {
      return undefined;
    }
    return trimTrailingSlash(parsed.toString());
  } catch {
    return undefined;
  }
};

export const autoDiscovery = async (
  request: typeof fetch,
  server: string
): Promise<[AutoDiscoveryError, undefined] | [undefined, AutoDiscoveryInfo]> => {
  const candidateHost = /^https?:\/\//.test(server) ? server : `https://${server}`;
  const host = normalizeSecureHomeserverUrl(candidateHost);
  if (!host) {
    return [
      {
        host: candidateHost,
        action: AutoDiscoveryAction.FAIL_ERROR,
      },
      undefined,
    ];
  }
  const autoDiscoveryUrl = `${host}/.well-known/matrix/client`;

  let response: Response;
  try {
    response = await request(autoDiscoveryUrl, { method: 'GET' });
  } catch {
    response = new Response(null, { status: 404 });
  }

  if (response.status === 404) {
    // AutoDiscoveryAction.IGNORE
    // We will use default value for IGNORE action
    return [
      undefined,
      {
        'm.homeserver': {
          base_url: host,
        },
      },
    ];
  }
  if (response.status !== 200) {
    return [
      {
        host,
        action: AutoDiscoveryAction.FAIL_PROMPT,
      },
      undefined,
    ];
  }

  let content: AutoDiscoveryInfo;
  try {
    content = (await response.json()) as AutoDiscoveryInfo;
  } catch {
    return [
      {
        host,
        action: AutoDiscoveryAction.FAIL_PROMPT,
      },
      undefined,
    ];
  }

  if (!content || typeof content !== 'object') {
    return [
      {
        host,
        action: AutoDiscoveryAction.FAIL_PROMPT,
      },
      undefined,
    ];
  }

  const baseUrl = content['m.homeserver']?.base_url;
  if (typeof baseUrl !== 'string') {
    return [
      {
        host,
        action: AutoDiscoveryAction.FAIL_PROMPT,
      },
      undefined,
    ];
  }

  const secureBaseUrl = normalizeSecureHomeserverUrl(baseUrl);
  if (!secureBaseUrl) {
    return [
      {
        host,
        action: AutoDiscoveryAction.FAIL_ERROR,
      },
      undefined,
    ];
  }

  content['m.homeserver'].base_url = secureBaseUrl;
  if (content['m.identity_server']) {
    const identityBaseUrl = content['m.identity_server'].base_url;
    const secureIdentityBaseUrl =
      typeof identityBaseUrl === 'string'
        ? normalizeSecureHomeserverUrl(identityBaseUrl)
        : undefined;
    if (secureIdentityBaseUrl) {
      content['m.identity_server'].base_url = secureIdentityBaseUrl;
    } else {
      delete content['m.identity_server'];
    }
  }

  return [undefined, content];
};

export type SpecVersions = {
  versions: string[];
  unstable_features?: Record<string, boolean>;
};
export const specVersions = async (
  request: typeof fetch,
  baseUrl: string
): Promise<SpecVersions> => {
  const secureBaseUrl = normalizeSecureHomeserverUrl(baseUrl);
  if (!secureBaseUrl) {
    throw new Error('Homeserver URL must use HTTPS (HTTP is loopback-only)');
  }
  const res = await request(`${secureBaseUrl}/_matrix/client/versions`);

  const data = (await res.json()) as unknown;

  if (data && typeof data === 'object' && 'versions' in data && Array.isArray(data.versions)) {
    return data as SpecVersions;
  }
  throw new Error('Homeserver URL does not appear to be a valid Matrix homeserver');
};
