import { createContext, useCallback, useContext, useEffect, useState } from 'react';
import {
  AutoDiscoveryAction,
  AutoDiscoveryInfo,
  SpecVersions,
  autoDiscovery,
  specVersions,
} from '../../cs-api';
import type { AuthFlows } from '../../hooks/useAuthFlows';
import { RegisterFlowStatus } from '../../hooks/useAuthFlows';
import { discoverLoginFlows } from './login/nativeLoginFlows';

/** Everything the auth pages need from one homeserver. */
export type ResolvedAuthServer = {
  serverName: string;
  autoDiscoveryInfo: AutoDiscoveryInfo;
  specVersions: SpecVersions;
  authFlows: AuthFlows;
};

export type AuthServerResolution =
  { ok: true; resolved: ResolvedAuthServer } | { ok: false; message: string };

export type AuthServerResolutionDeps = {
  discover: typeof autoDiscovery;
  versions: typeof specVersions;
  loginFlows: typeof discoverLoginFlows;
  request: typeof fetch;
};

const defaultDeps = (): AuthServerResolutionDeps => ({
  discover: autoDiscovery,
  versions: specVersions,
  loginFlows: discoverLoginFlows,
  request: fetch,
});

/**
 * Discovery, spec versions and login flows as one step, so the auth pages
 * swap servers once, when the new server is fully usable.
 */
export const resolveAuthServer = async (
  serverName: string,
  deps: AuthServerResolutionDeps = defaultDeps()
): Promise<AuthServerResolution> => {
  let discovery: Awaited<ReturnType<typeof autoDiscovery>>;
  try {
    discovery = await deps.discover(deps.request, serverName);
  } catch {
    return { ok: false, message: `Couldn't find a homeserver at ${serverName}.` };
  }
  const [discoveryError, autoDiscoveryInfo] = discovery;
  if (discoveryError) {
    return {
      ok: false,
      message:
        discoveryError.action === AutoDiscoveryAction.FAIL_PROMPT
          ? `${discoveryError.host} advertises a homeserver that can't be used.`
          : `${serverName} has an invalid homeserver address.`,
    };
  }
  const baseUrl = autoDiscoveryInfo['m.homeserver'].base_url;
  let versions: SpecVersions;
  try {
    versions = await deps.versions(deps.request, baseUrl);
  } catch {
    return { ok: false, message: `${serverName} isn't responding right now.` };
  }
  let authFlows: AuthFlows;
  try {
    authFlows = {
      loginFlows: await deps.loginFlows(baseUrl),
      // Registration flows are probed natively by the Register page.
      registerFlows: { status: RegisterFlowStatus.InvalidRequest },
    };
  } catch {
    return { ok: false, message: `Couldn't load sign-in options from ${serverName}.` };
  }
  return {
    ok: true,
    resolved: { serverName, autoDiscoveryInfo, specVersions: versions, authFlows },
  };
};

export type AuthServerStatus = {
  /** The server the user picked; may differ from `resolved` while pending. */
  serverName: string;
  pending: boolean;
  error?: string;
  /** The last server that resolved; kept while another one loads. */
  resolved?: ResolvedAuthServer;
  /** True only when `resolved` is the picked server and nothing is pending. */
  ready: boolean;
};

type ResolutionState = {
  attempt: { serverName: string; seq: number };
  pending: boolean;
  error?: string;
  resolved?: ResolvedAuthServer;
};

/**
 * Keeps the last resolved server mounted while a new one resolves, so the
 * form below the server field never unmounts (and never loses typed input)
 * when the server changes.
 */
export const useAuthServerResolution = (
  serverName: string,
  resolve: (serverName: string) => Promise<AuthServerResolution> = resolveAuthServer
): [AuthServerStatus, () => void] => {
  const [state, setState] = useState<ResolutionState>({
    attempt: { serverName, seq: 0 },
    pending: true,
  });
  const [seq, setSeq] = useState(0);

  useEffect(() => {
    if (!serverName) return undefined;
    let cancelled = false;
    setState((prev) => ({
      ...prev,
      attempt: { serverName, seq },
      pending: true,
      error: undefined,
    }));
    resolve(serverName).then(
      (result) => {
        if (cancelled) return;
        setState((prev) =>
          result.ok
            ? { attempt: prev.attempt, pending: false, resolved: result.resolved }
            : { ...prev, pending: false, error: result.message }
        );
      },
      () => {
        if (cancelled) return;
        setState((prev) => ({
          ...prev,
          pending: false,
          error: `Couldn't connect to ${serverName}.`,
        }));
      }
    );
    return () => {
      cancelled = true;
    };
  }, [resolve, serverName, seq]);

  const retry = useCallback(() => setSeq((value) => value + 1), []);

  // Until the effect for a new server runs, this render is already stale.
  const current = state.attempt.serverName === serverName;
  const pending = !current || state.pending;
  const error = current ? state.error : undefined;
  return [
    {
      serverName,
      pending,
      error,
      resolved: state.resolved,
      ready: !pending && !error && state.resolved?.serverName === serverName,
    },
    retry,
  ];
};

const AuthServerStatusContext = createContext<AuthServerStatus | null>(null);

export const AuthServerStatusProvider = AuthServerStatusContext.Provider;

/** Submit buttons stay disabled until the picked server is ready. */
export const useAuthServerReady = (): boolean => useContext(AuthServerStatusContext)?.ready ?? true;

/** The server the user picked, even while it is still resolving. */
export const usePickedAuthServer = (fallback: string): string =>
  useContext(AuthServerStatusContext)?.serverName ?? fallback;

export const useAuthServerPending = (): boolean =>
  useContext(AuthServerStatusContext)?.pending ?? false;
