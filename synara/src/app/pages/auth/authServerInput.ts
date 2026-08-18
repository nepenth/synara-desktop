export const normalizeAuthServerInput = (raw: string): string => {
  const server = raw.trim();
  if (!/^https?:\/\//i.test(server)) return server;

  try {
    const url = new URL(server);
    if (url.username || url.password) return server;
    return url.host;
  } catch {
    return server;
  }
};
