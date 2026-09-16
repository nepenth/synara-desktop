const TOKEN_QUERY_KEYS = new Set(['access_token', 'logintoken', 'accesstoken']);

const normalizeHost = (host: string): string =>
  host.replace(/^\[|\]$/g, '').replace(/\.$/, '').toLowerCase();

const isLoopbackHost = (host: string): boolean => {
  const normalized = normalizeHost(host);
  return normalized === '127.0.0.1' || normalized === 'localhost' || normalized === '::1';
};

const isPrivateIpv4 = (host: string): boolean => {
  const normalized = normalizeHost(host);
  if (
    normalized.startsWith('0.') ||
    normalized.startsWith('10.') ||
    normalized.startsWith('127.') ||
    normalized.startsWith('169.254.') ||
    normalized.startsWith('192.168.')
  ) {
    return true;
  }
  const octets = normalized.split('.');
  const second = Number(octets[1]);
  if (normalized.startsWith('172.') && second >= 16 && second <= 31) return true;
  if (normalized.startsWith('100.') && second >= 64 && second <= 127) return true;
  return false;
};

const isPrivateIpv6 = (host: string): boolean => {
  const normalized = normalizeHost(host);
  return (
    normalized === '::1' ||
    normalized === '::' ||
    normalized.startsWith('fc') ||
    normalized.startsWith('fd') ||
    normalized.startsWith('fe80') ||
    normalized.startsWith('::ffff:')
  );
};

const isLocalHostname = (host: string): boolean => {
  const normalized = normalizeHost(host);
  if (normalized === 'localhost' || normalized === '0.0.0.0') return true;
  return ['.localhost', '.local', '.localdomain', '.internal', '.lan', '.home.arpa'].some(
    (suffix) => normalized.endsWith(suffix)
  );
};

const isSafePublicHttpsHost = (host: string): boolean => {
  const normalized = normalizeHost(host);
  if (!normalized || isLocalHostname(normalized)) return false;
  if (normalized.includes(':')) return !isPrivateIpv6(normalized);
  return !isPrivateIpv4(normalized);
};

/** Mirrors Core `is_safe_widget_url`. Loopback is agent-list only. */
export const isSafeWidgetUrl = (value: string, allowLoopback: boolean): boolean => {
  let url: URL;
  try {
    url = new URL(value);
  } catch {
    return false;
  }
  if (url.username || url.password) return false;
  if ([...url.searchParams.keys()].some((key) => TOKEN_QUERY_KEYS.has(key.toLowerCase()))) {
    return false;
  }
  const host = url.hostname;
  if (!host) return false;
  if (url.protocol === 'https:') {
    return isLoopbackHost(host) ? allowLoopback : isSafePublicHttpsHost(host);
  }
  if (url.protocol === 'http:') {
    return allowLoopback && isLoopbackHost(host);
  }
  return false;
};
