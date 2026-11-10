const LOOPBACK_HOSTNAMES = new Set(['localhost', '127.0.0.1', '[::1]']);

/**
 * A typed homeserver as the server name the auth pages work with. A URL
 * becomes its host, except a plain-HTTP URL on this machine, which keeps its
 * scheme: a bare `localhost:8008` would be discovered over HTTPS, and a
 * development homeserver does not speak it.
 */
export const normalizeAuthServerInput = (raw: string): string => {
  const server = raw.trim();
  if (!/^https?:\/\//i.test(server)) return server;

  try {
    const url = new URL(server);
    if (url.username || url.password) return server;
    if (url.protocol === 'http:' && LOOPBACK_HOSTNAMES.has(url.hostname)) {
      return `http://${url.host}`;
    }
    return url.host;
  } catch {
    return server;
  }
};
