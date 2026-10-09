import React, { useCallback, useEffect } from 'react';
import { hasSessionExpiryNotice } from '../../utils/sessionExpiry';
import { Icon, Icons, Scroll, Spinner, Text } from 'folds';
import {
  Outlet,
  generatePath,
  matchPath,
  useLocation,
  useNavigate,
  useParams,
} from 'react-router-dom';
import classNames from 'classnames';

import { AuthFooter } from './AuthFooter';
import * as css from './styles.css';
import {
  clientAllowedServer,
  clientDefaultServer,
  useClientConfig,
} from '../../hooks/useClientConfig';
import { LOGIN_PATH, REGISTER_PATH, RESET_PASSWORD_PATH } from '../paths';
import { ServerPicker } from './ServerPicker';
import { SpecVersionsProvider } from '../../hooks/useSpecVersions';
import { AutoDiscoveryInfoProvider } from '../../hooks/useAutoDiscoveryInfo';
import { AuthFlowsProvider } from '../../hooks/useAuthFlows';
import { AuthServerProvider } from '../../hooks/useAuthServer';
import { tryDecodeURIComponent } from '../../utils/dom';
import { normalizeAuthServerInput } from './authServerInput';
import {
  AuthServerStatus,
  AuthServerStatusProvider,
  useAuthServerResolution,
} from './authServerResolution';
import { SynaraMark } from './SynaraMark';

const currentAuthPath = (pathname: string): string => {
  if (matchPath(LOGIN_PATH, pathname)) {
    return LOGIN_PATH;
  }
  if (matchPath(RESET_PASSWORD_PATH, pathname)) {
    return RESET_PASSWORD_PATH;
  }
  if (matchPath(REGISTER_PATH, pathname)) {
    return REGISTER_PATH;
  }
  return LOGIN_PATH;
};

const highestSpecVersion = (versions: string[]): string | undefined =>
  versions
    .filter((version) => /^v\d+\.\d+$/.test(version))
    .sort((a, b) => {
      const [aMajor, aMinor] = a.slice(1).split('.').map(Number);
      const [bMajor, bMinor] = b.slice(1).split('.').map(Number);
      return aMajor - bMajor || aMinor - bMinor;
    })
    .pop();

function ServerStatusLine({ status, onRetry }: { status: AuthServerStatus; onRetry: () => void }) {
  if (status.pending) {
    return (
      <div className={css.ServerStatus} role="status">
        <Spinner size="50" variant="Secondary" />
        <span className={css.ServerStatusText}>Connecting to {status.serverName}…</span>
      </div>
    );
  }
  if (status.error) {
    return (
      <div className={classNames(css.ServerStatus, css.ServerStatusError)} role="alert">
        <Icon size="50" src={Icons.Warning} />
        <span className={css.ServerStatusText} title={status.error}>
          {status.error}
        </span>
        <button className={css.ServerStatusRetry} type="button" onClick={onRetry}>
          Retry
        </button>
      </div>
    );
  }
  const version = status.resolved && highestSpecVersion(status.resolved.specVersions.versions);
  return (
    <div className={css.ServerStatus} role="status">
      <span className={css.ServerStatusDot} />
      <span className={css.ServerStatusText}>Connected{version ? ` · Matrix ${version}` : ''}</span>
    </div>
  );
}

function AuthBrand() {
  return (
    <section className={css.AuthBrand} aria-label="Synara">
      <div className={css.AuthBrandMark}>
        <SynaraMark />
      </div>
      <h1 className={css.AuthWordmark}>Synara</h1>
      <p className={css.AuthTagline}>Secure Matrix messaging for you and your agents.</p>
      <ul className={css.AuthFeatures}>
        <li className={css.AuthFeature}>
          <span className={css.AuthFeatureIcon}>
            <Icon size="100" src={Icons.ShieldLock} />
          </span>
          End-to-end encrypted rooms and direct messages
        </li>
        <li className={css.AuthFeature}>
          <span className={css.AuthFeatureIcon}>
            <Icon size="100" src={Icons.Terminal} />
          </span>
          Agent approvals you can act on from anywhere
        </li>
        <li className={css.AuthFeature}>
          <span className={css.AuthFeatureIcon}>
            <Icon size="100" src={Icons.Monitor} />
          </span>
          Native on macOS, Linux and iOS
        </li>
      </ul>
    </section>
  );
}

export function AuthLayout() {
  const navigate = useNavigate();
  const location = useLocation();
  const { server: urlEncodedServer } = useParams();

  const clientConfig = useClientConfig();

  const defaultServer = clientDefaultServer(clientConfig);
  let server = normalizeAuthServerInput(
    urlEncodedServer ? tryDecodeURIComponent(urlEncodedServer) : defaultServer
  );

  if (!clientAllowedServer(clientConfig, server)) {
    server = defaultServer;
  }

  const [status, retry] = useAuthServerResolution(server);

  // if server is mismatches with path server, update path
  useEffect(() => {
    if (!urlEncodedServer || tryDecodeURIComponent(urlEncodedServer) !== server) {
      navigate(
        generatePath(currentAuthPath(location.pathname), {
          server,
        }),
        { replace: true }
      );
    }
  }, [urlEncodedServer, navigate, location, server]);

  const selectServer = useCallback(
    (newServer: string) => {
      const normalizedServer = normalizeAuthServerInput(newServer);
      if (!normalizedServer) return;
      if (normalizedServer === server) {
        if (!status.pending) retry();
        return;
      }
      navigate(
        generatePath(currentAuthPath(location.pathname), {
          server: normalizedServer,
        }),
        { replace: true }
      );
    },
    [navigate, location, server, status.pending, retry]
  );

  const { resolved } = status;

  return (
    <Scroll variant="Background" visibility="Hover" size="300" hideTrack>
      <div className={css.AuthLayout}>
        <div className={css.AuthAurora} aria-hidden />
        <div className={css.AuthGrid} aria-hidden />
        <main className={css.AuthStage}>
          <AuthBrand />
          <div className={css.AuthCard}>
            <div className={css.AuthCardContent}>
              {hasSessionExpiryNotice() && (
                <Text className={css.AuthNotice} size="T300" role="alert">
                  Your session expired. Sign in again to reconnect.
                </Text>
              )}
              <div>
                <Text as="label" htmlFor="synara-homeserver" size="L400" priority="300">
                  Homeserver
                </Text>
                <div style={{ marginTop: 6, marginBottom: 6 }}>
                  <ServerPicker
                    id="synara-homeserver"
                    server={server}
                    serverList={clientConfig.homeserverList ?? []}
                    allowCustomServer={clientConfig.allowCustomHomeservers}
                    onServerChange={selectServer}
                  />
                </div>
                <ServerStatusLine status={status} onRetry={retry} />
              </div>
              <hr className={css.AuthDivider} />
              {resolved ? (
                <AuthServerStatusProvider value={status}>
                  <AuthServerProvider value={resolved.serverName}>
                    <AutoDiscoveryInfoProvider value={resolved.autoDiscoveryInfo}>
                      <SpecVersionsProvider value={resolved.specVersions}>
                        <AuthFlowsProvider value={resolved.authFlows}>
                          <Outlet />
                        </AuthFlowsProvider>
                      </SpecVersionsProvider>
                    </AutoDiscoveryInfoProvider>
                  </AuthServerProvider>
                </AuthServerStatusProvider>
              ) : (
                <div className={css.AuthLoading}>
                  {status.pending ? (
                    <>
                      <Spinner size="200" variant="Secondary" />
                      <Text size="T300">Preparing sign-in…</Text>
                    </>
                  ) : (
                    <Text size="T300" align="Center">
                      Choose a homeserver to continue.
                    </Text>
                  )}
                </div>
              )}
            </div>
          </div>
        </main>
        <AuthFooter />
      </div>
    </Scroll>
  );
}
