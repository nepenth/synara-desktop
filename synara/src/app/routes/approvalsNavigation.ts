import type { Location } from 'react-router-dom';
import { normalizeSynaraRoute, parseSynaraRouteDestination } from './synaraRoutes';

/** History-entry state, not a global remembered route shared across sessions. */
export const getApprovalsReturnPath = (state: unknown): string | undefined => {
  if (!state || typeof state !== 'object') return undefined;
  const path = normalizeSynaraRoute((state as { approvalsReturnTo?: unknown }).approvalsReturnTo);
  return path && parseSynaraRouteDestination(path)?.kind !== 'approvals' ? path : undefined;
};

export const createApprovalsNavigationState = (
  location: Pick<Location, 'pathname' | 'search' | 'hash' | 'state'>
): { approvalsReturnTo?: string } => ({
  approvalsReturnTo:
    parseSynaraRouteDestination(location.pathname)?.kind === 'approvals'
      ? getApprovalsReturnPath(location.state)
      : normalizeSynaraRoute(`${location.pathname}${location.search}${location.hash}`),
});

export const getApprovalsOriginSpace = (state: unknown): string | undefined => {
  const path = getApprovalsReturnPath(state);
  const origin = path ? parseSynaraRouteDestination(path) : undefined;
  if (origin?.kind === 'space' || origin?.kind === 'spaceLobby') return origin.spaceIdOrAlias;
  return origin?.kind === 'room' ? origin.parentSpaceIdOrAlias : undefined;
};
