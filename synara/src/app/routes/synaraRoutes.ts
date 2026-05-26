export type SynaraRouteDestination =
  | { kind: 'home' }
  | { kind: 'direct' }
  | {
      kind: 'room';
      roomIdOrAlias: string;
      eventId?: string;
      parentSpaceIdOrAlias?: string;
    }
  | { kind: 'space'; spaceIdOrAlias: string }
  | { kind: 'spaceLobby'; spaceIdOrAlias: string }
  | { kind: 'inbox'; section?: 'notifications' | 'invites' | 'later' }
  | { kind: 'create' }
  | { kind: 'explore'; server?: string }
  | { kind: 'settings'; section?: string };

const MAX_ROUTE_LENGTH = 2_048;

const STATIC_ROUTE_DESTINATIONS: Record<string, SynaraRouteDestination> = {
  '/': { kind: 'home' },
  '/home/': { kind: 'home' },
  '/direct/': { kind: 'direct' },
  '/inbox/': { kind: 'inbox' },
  '/inbox/notifications/': { kind: 'inbox', section: 'notifications' },
  '/inbox/invites/': { kind: 'inbox', section: 'invites' },
  '/inbox/later/': { kind: 'inbox', section: 'later' },
  '/create': { kind: 'create' },
  '/create/': { kind: 'create' },
  '/explore/': { kind: 'explore' },
  '/settings/': { kind: 'settings' },
};

const RESERVED_TOP_LEVEL_SEGMENTS = new Set([
  'login',
  'register',
  'reset-password',
  'space-settings',
  'room-settings',
  'home',
  'direct',
  'inbox',
  'create',
  'explore',
  'settings',
]);

const normalizeSegment = (segment: string): string | undefined => {
  if (!segment) return undefined;

  try {
    const decoded = decodeURIComponent(segment);
    return decoded.length > 0 ? decoded : undefined;
  } catch {
    return undefined;
  }
};

const splitRoutePath = (route: string): string[] | undefined => {
  const path = route.split(/[?#]/, 1)[0];
  if (path.includes('//')) return undefined;

  const trimmedPath = path === '/' ? path : path.replace(/\/+$/, '');
  if (trimmedPath === '/') return [];

  const segments = trimmedPath.slice(1).split('/').map(normalizeSegment);

  if (segments.some((segment) => !segment)) return undefined;
  return segments as string[];
};

const normalizeRouteString = (value: unknown): string | undefined => {
  if (typeof value !== 'string') return undefined;

  const route = value.trim();
  if (!route) return undefined;
  if (route.length > MAX_ROUTE_LENGTH) return undefined;
  if (!route.startsWith('/') || route.startsWith('//')) return undefined;
  if (route.includes('\\') || route.includes('://')) return undefined;

  return route;
};

export const parseSynaraRouteDestination = (value: unknown): SynaraRouteDestination | undefined => {
  const route = normalizeRouteString(value);
  if (!route) return undefined;

  const path = route.split(/[?#]/, 1)[0];
  const withTrailingSlash = path.endsWith('/') ? path : `${path}/`;
  const staticDestination =
    STATIC_ROUTE_DESTINATIONS[path] ?? STATIC_ROUTE_DESTINATIONS[withTrailingSlash];
  if (staticDestination) return staticDestination;

  const segments = splitRoutePath(route);
  if (!segments) return undefined;

  if (segments.length === 2 && segments[0] === 'explore') {
    return { kind: 'explore', server: segments[1] };
  }

  if ((segments.length === 2 || segments.length === 3) && segments[0] === 'home') {
    return {
      kind: 'room',
      roomIdOrAlias: segments[1],
      eventId: segments[2],
    };
  }

  if ((segments.length === 2 || segments.length === 3) && segments[0] === 'direct') {
    return {
      kind: 'room',
      roomIdOrAlias: segments[1],
      eventId: segments[2],
    };
  }

  const [first, second, third] = segments;
  if (!first || RESERVED_TOP_LEVEL_SEGMENTS.has(first)) return undefined;

  if (segments.length === 1) {
    return { kind: 'space', spaceIdOrAlias: first };
  }

  if (segments.length === 2 && second === 'lobby') {
    return { kind: 'spaceLobby', spaceIdOrAlias: first };
  }

  if (segments.length === 2 || segments.length === 3) {
    return {
      kind: 'room',
      parentSpaceIdOrAlias: first,
      roomIdOrAlias: second,
      eventId: third,
    };
  }

  return undefined;
};

export const normalizeSynaraRoute = (value: unknown): string | undefined => {
  const route = normalizeRouteString(value);
  if (!route) return undefined;
  if (!parseSynaraRouteDestination(route)) return undefined;

  return route;
};
