const LOCALHOST_NAMES = new Set(['localhost', 'localhost.', '0.0.0.0']);

const IPV4_PRIVATE_RANGES = [
  /^0\./,
  /^10\./,
  /^127\./,
  /^169\.254\./,
  /^172\.(1[6-9]|2\d|3[0-1])\./,
  /^192\.168\./,
  /^100\.(6[4-9]|[7-9]\d|1[01]\d|12[0-7])\./,
];

const LOCAL_HOST_SUFFIXES = [
  '.localhost',
  '.local',
  '.localdomain',
  '.internal',
  '.lan',
  '.home.arpa',
];

const normalizeHost = (hostname: string): string =>
  hostname
    .replace(/^\[|\]$/g, '')
    .replace(/\.$/, '')
    .toLowerCase();

export const isPrivateIpv4 = (host: string): boolean =>
  IPV4_PRIVATE_RANGES.some((r) => r.test(normalizeHost(host)));

export const isPrivateIpv6 = (host: string): boolean => {
  const lowerHost = normalizeHost(host);
  return (
    lowerHost === '::1' ||
    lowerHost === '::' ||
    lowerHost.startsWith('fc') ||
    lowerHost.startsWith('fd') ||
    lowerHost.startsWith('fe80') ||
    lowerHost.startsWith('::ffff:')
  );
};

export const isLocalHostname = (hostname: string): boolean => {
  const host = normalizeHost(hostname);
  return LOCALHOST_NAMES.has(host) || LOCAL_HOST_SUFFIXES.some((suffix) => host.endsWith(suffix));
};

export const isSafeHttpsUrl = (value: string): boolean => {
  let url: URL;
  try {
    url = new URL(value);
  } catch {
    return false;
  }

  if (url.protocol !== 'https:') return false;
  if (url.username || url.password) return false;
  const host = normalizeHost(url.hostname);
  if (isLocalHostname(host)) return false;
  if (isPrivateIpv4(host)) return false;
  if (host.includes(':') && isPrivateIpv6(host)) {
    return false;
  }

  return true;
};

export const safeRemoteContentUrl = (value: string): string | undefined => {
  if (!isSafeHttpsUrl(value)) return undefined;
  return value;
};
