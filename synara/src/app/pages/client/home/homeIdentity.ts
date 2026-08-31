export const homeserverDisplayName = (baseUrl?: string, userId?: string): string => {
  const serverUrl = baseUrl?.trim();
  if (serverUrl) {
    try {
      const hostname = new URL(serverUrl).hostname;
      if (hostname) return hostname;
    } catch {
      // Fall through to the authenticated Matrix user id. Never display a URL
      // path, query, or credential-shaped value in the navigation identity.
    }
  }

  const separator = userId?.lastIndexOf(':') ?? -1;
  if (userId && separator >= 0 && separator < userId.length - 1) {
    return userId.slice(separator + 1);
  }
  return 'Home';
};
