import {
  getDirectPath,
  getExplorePath,
  getHomePath,
  getInboxPath,
  getSpacePath,
} from '../pages/pathUtils';
import { getApprovalsReturnPath } from '../routes/approvalsNavigation';
import { parseSynaraRouteDestination } from '../routes/synaraRoutes';

export const getBackRoutePath = (pathname: string, state?: unknown): string => {
  // Check fixed sections first: /:spaceIdOrAlias/ also matches inbox, explore,
  // approvals, and auth paths. The shared parser reserves those names.
  if (/^\/home(?:\/|$)/.test(pathname)) return getHomePath();
  if (/^\/direct(?:\/|$)/.test(pathname)) return getDirectPath();
  if (/^\/inbox(?:\/|$)/.test(pathname)) return getInboxPath();
  if (/^\/explore(?:\/|$)/.test(pathname)) return getExplorePath();
  const destination = parseSynaraRouteDestination(pathname);
  if (destination?.kind === 'approvals') return getApprovalsReturnPath(state) ?? getHomePath();
  if (destination?.kind === 'space' || destination?.kind === 'spaceLobby') {
    return getSpacePath(destination.spaceIdOrAlias);
  }
  if (destination?.kind === 'room' && destination.parentSpaceIdOrAlias) {
    return getSpacePath(destination.parentSpaceIdOrAlias);
  }
  return getHomePath();
};
