import React, { FormEventHandler, MouseEventHandler, useCallback, useState } from 'react';
import {
  Box,
  Button,
  Dialog,
  Header,
  Icon,
  IconButton,
  Icons,
  Input,
  Menu,
  Overlay,
  OverlayBackdrop,
  OverlayCenter,
  PopOut,
  RectCords,
  Spinner,
  Text,
  config,
} from 'folds';
import FocusTrap from 'focus-trap-react';
import { Link } from 'react-router-dom';
import { getMxIdLocalPart, getMxIdServer, isUserId } from '../../../utils/matrix';
import { EMAIL_REGEX } from '../../../utils/regex';
import { useAutoDiscoveryInfo } from '../../../hooks/useAutoDiscoveryInfo';
import { AsyncStatus, useAsyncCallback } from '../../../hooks/useAsyncCallback';
import { useAuthServer } from '../../../hooks/useAuthServer';
import { useClientConfig } from '../../../hooks/useClientConfig';
import {
  LoginError,
  PasswordLoginError,
  PasswordLoginResponse,
  STORE_RECOVERY_CONFIRMATION_TEXT,
  StoreRecoveryError,
  archiveAndRebuildNativeStore,
  canOfferNativeStoreRecovery,
  factoryGetBaseUrl,
  loginPassword,
  useLoginComplete,
} from './loginUtil';
import { PasswordInput } from '../../../components/password-input';
import { FieldError } from '../FiledError';
import { getResetPasswordPath } from '../../pathUtils';
import { stopPropagation } from '../../../utils/keyboard';
import { synaraDeviceDisplayName } from '../../../utils/user-agent';

function UsernameHint({ server }: { server: string }) {
  const [anchor, setAnchor] = useState<RectCords>();

  const handleOpenMenu: MouseEventHandler<HTMLElement> = (evt) => {
    setAnchor(evt.currentTarget.getBoundingClientRect());
  };
  return (
    <PopOut
      anchor={anchor}
      position="Top"
      align="End"
      content={
        <FocusTrap
          focusTrapOptions={{
            initialFocus: false,
            onDeactivate: () => setAnchor(undefined),
            clickOutsideDeactivates: true,
            escapeDeactivates: stopPropagation,
          }}
        >
          <Menu>
            <Header size="300" style={{ padding: `0 ${config.space.S200}` }}>
              <Text size="L400">Hint</Text>
            </Header>
            <Box
              style={{ padding: config.space.S200, paddingTop: 0 }}
              direction="Column"
              tabIndex={0}
              gap="100"
            >
              <Text size="T300">
                <Text as="span" size="Inherit" priority="300">
                  Username:
                </Text>{' '}
                user123
              </Text>
              <Text size="T300">
                <Text as="span" size="Inherit" priority="300">
                  Matrix ID:
                </Text>
                {` @user123:${server}`}
              </Text>
              <Text size="T300">
                <Text as="span" size="Inherit" priority="300">
                  Email:
                </Text>
                {` user123@${server}`}
              </Text>
            </Box>
          </Menu>
        </FocusTrap>
      }
    >
      <IconButton
        tabIndex={-1}
        onClick={handleOpenMenu}
        type="button"
        variant="Background"
        size="300"
        radii="300"
        aria-pressed={!!anchor}
      >
        <Icon style={{ opacity: config.opacity.P300 }} size="100" src={Icons.Info} />
      </IconButton>
    </PopOut>
  );
}

type PasswordLoginFormProps = {
  defaultUsername?: string;
  defaultEmail?: string;
};
export function PasswordLoginForm({ defaultUsername, defaultEmail }: PasswordLoginFormProps) {
  const server = useAuthServer();
  const clientConfig = useClientConfig();

  const serverDiscovery = useAutoDiscoveryInfo();
  const baseUrl = serverDiscovery['m.homeserver'].base_url;

  const [loginState, startLogin] = useAsyncCallback<
    PasswordLoginResponse,
    PasswordLoginError,
    Parameters<typeof loginPassword>
  >(useCallback(loginPassword, []));

  useLoginComplete(loginState.status === AsyncStatus.Success ? loginState.data : undefined);

  const [storeRecoveryOpen, setStoreRecoveryOpen] = useState(false);
  const [storeRecoveryConfirmation, setStoreRecoveryConfirmation] = useState('');
  const [storeRecoveryBusy, setStoreRecoveryBusy] = useState(false);
  const [storeRecoveryDiagnostic, setStoreRecoveryDiagnostic] = useState<string>();
  const [storeRecoveryCompleted, setStoreRecoveryCompleted] = useState(false);

  const openStoreRecovery = () => {
    // Opening this UI has no native side effect. The only archive request is
    // behind the typed acknowledgement and the second explicit button below.
    setStoreRecoveryConfirmation('');
    setStoreRecoveryDiagnostic(undefined);
    setStoreRecoveryOpen(true);
  };

  const closeStoreRecovery = () => {
    if (storeRecoveryBusy) return;
    setStoreRecoveryOpen(false);
    setStoreRecoveryConfirmation('');
  };

  const confirmStoreRecovery = async () => {
    if (storeRecoveryConfirmation !== STORE_RECOVERY_CONFIRMATION_TEXT) return;
    setStoreRecoveryBusy(true);
    setStoreRecoveryDiagnostic(undefined);
    try {
      await archiveAndRebuildNativeStore(storeRecoveryConfirmation);
      setStoreRecoveryCompleted(true);
      setStoreRecoveryOpen(false);
      setStoreRecoveryConfirmation('');
    } catch (error) {
      // The helper already discards raw Tauri/SDK errors and logs only a
      // fixed allowlisted diagnostic. Do not turn unknown error text into UI.
      setStoreRecoveryDiagnostic(
        error instanceof StoreRecoveryError
          ? error.diagnosticId
          : 'p3.2-login-store-recovery-failed'
      );
    } finally {
      setStoreRecoveryBusy(false);
    }
  };

  const handleUsernameLogin = (username: string, password: string) => {
    startLogin(baseUrl, {
      type: 'm.login.password',
      identifier: {
        type: 'm.id.user',
        user: `@${username}:${server}`,
      },
      password,
      initial_device_display_name: synaraDeviceDisplayName(),
    });
  };

  const handleMxIdLogin = async (mxId: string, password: string) => {
    const mxIdServer = getMxIdServer(mxId);
    const mxIdUsername = getMxIdLocalPart(mxId);
    if (!mxIdServer || !mxIdUsername) return;

    const getBaseUrl = factoryGetBaseUrl(clientConfig, mxIdServer);

    startLogin(getBaseUrl, {
      type: 'm.login.password',
      identifier: {
        type: 'm.id.user',
        user: mxId,
      },
      password,
      initial_device_display_name: synaraDeviceDisplayName(),
    });
  };
  const handleEmailLogin = (email: string, password: string) => {
    startLogin(baseUrl, {
      type: 'm.login.password',
      identifier: {
        type: 'm.id.thirdparty',
        medium: 'email',
        address: email,
      },
      password,
      initial_device_display_name: synaraDeviceDisplayName(),
    });
  };

  const handleSubmit: FormEventHandler<HTMLFormElement> = (evt) => {
    evt.preventDefault();
    const { usernameInput, passwordInput } = evt.target as HTMLFormElement & {
      usernameInput: HTMLInputElement;
      passwordInput: HTMLInputElement;
    };

    const username = usernameInput.value.trim();
    const password = passwordInput.value;
    if (!username) {
      usernameInput.focus();
      return;
    }
    if (!password) {
      passwordInput.focus();
      return;
    }

    if (isUserId(username)) {
      handleMxIdLogin(username, password);
      return;
    }
    if (EMAIL_REGEX.test(username)) {
      handleEmailLogin(username, password);
      return;
    }
    handleUsernameLogin(username, password);
  };

  return (
    <Box as="form" onSubmit={handleSubmit} direction="Inherit" gap="400">
      <Box direction="Column" gap="100">
        <Text as="label" size="L400" priority="300">
          Username
        </Text>
        <Input
          defaultValue={defaultUsername ?? defaultEmail}
          style={{ paddingRight: config.space.S300 }}
          name="usernameInput"
          variant="Background"
          size="500"
          required
          outlined
          after={<UsernameHint server={server} />}
        />
        {loginState.status === AsyncStatus.Error && (
          <>
            {loginState.error.errcode === LoginError.ServerNotAllowed && (
              <FieldError message="Login with custom server not allowed by your client instance." />
            )}
            {loginState.error.errcode === LoginError.InvalidServer && (
              <FieldError message="Failed to find your Matrix ID server." />
            )}
          </>
        )}
      </Box>
      <Box direction="Column" gap="100">
        <Text as="label" size="L400" priority="300">
          Password
        </Text>
        <PasswordInput name="passwordInput" variant="Background" size="500" outlined required />
        <Box alignItems="Start" justifyContent="SpaceBetween" gap="200">
          {loginState.status === AsyncStatus.Error && (
            <>
              {loginState.error.errcode === LoginError.Forbidden && (
                <FieldError message="Invalid Username or Password." />
              )}
              {loginState.error.errcode === LoginError.UserDeactivated && (
                <FieldError message="This account has been deactivated." />
              )}
              {loginState.error.errcode === LoginError.InvalidRequest && (
                <FieldError message="Failed to login. Part of your request data is invalid." />
              )}
              {loginState.error.errcode === LoginError.RateLimited && (
                <FieldError message="Failed to login. Your login request has been rate-limited by server, Please try after some time." />
              )}
              {loginState.error.errcode === LoginError.Unknown && (
                <>
                  <FieldError message="Failed to login. Unknown reason." />
                  {loginState.error.diagnosticId && (
                    <Text as="span" size="T200" priority="400">
                      Diagnostic code: {loginState.error.diagnosticId}
                    </Text>
                  )}
                </>
              )}
              {canOfferNativeStoreRecovery(loginState.error.diagnosticId) && (
                <Box direction="Column" gap="100">
                  <Text size="T300" priority="400">
                    Synara stopped before changing this local Matrix store. You can review an
                    archive-and-rebuild recovery action instead of deleting local data.
                  </Text>
                  <Button type="button" variant="Secondary" fill="Soft" onClick={openStoreRecovery}>
                    <Text as="span" size="B300">
                      Review Local Store Recovery
                    </Text>
                  </Button>
                </Box>
              )}
              {storeRecoveryCompleted && (
                <Text size="T300" priority="400">
                  Local state, crypto, cache, and media were archived and an empty local layout was
                  rebuilt. Sign in again to try the normal native login path.
                </Text>
              )}
            </>
          )}
          <Box grow="Yes" shrink="No" justifyContent="End">
            <Text as="span" size="T200" priority="400" align="Right">
              <Link to={getResetPasswordPath(server)}>Forgot Password?</Link>
            </Text>
          </Box>
        </Box>
      </Box>
      <Button type="submit" variant="Primary" size="500">
        <Text as="span" size="B500">
          Login
        </Text>
      </Button>

      <Overlay open={storeRecoveryOpen} backdrop={<OverlayBackdrop />}>
        <OverlayCenter>
          <Dialog variant="Surface">
            <Box style={{ padding: config.space.S400, maxWidth: 520 }} direction="Column" gap="400">
              <Text size="H4">Archive and Rebuild Local Store?</Text>
              <Text size="T300" priority="400">
                Synara will archive this account&apos;s local state, crypto, cache, and media before
                rebuilding an empty local layout. It does not delete the archive, change any
                Keychain store key, send your password, or start another login automatically.
              </Text>
              <Text size="T300" priority="400">
                If the original Keychain store key is missing or corrupt, recovery remains blocked;
                this action does not replace it.
              </Text>
              <Box direction="Column" gap="100">
                <Text as="label" size="L400" priority="300">
                  Type {STORE_RECOVERY_CONFIRMATION_TEXT} to enable this action
                </Text>
                <Input
                  aria-label="Confirm archive and rebuild local Matrix store"
                  value={storeRecoveryConfirmation}
                  onChange={(evt) => setStoreRecoveryConfirmation(evt.currentTarget.value)}
                  variant="Background"
                  size="500"
                  outlined
                  disabled={storeRecoveryBusy}
                />
              </Box>
              {storeRecoveryDiagnostic && (
                <>
                  <FieldError message="Local store recovery could not be completed." />
                  <Text as="span" size="T200" priority="400">
                    Diagnostic code: {storeRecoveryDiagnostic}
                  </Text>
                </>
              )}
              <Box direction="Column" gap="200">
                <Button
                  type="button"
                  variant="Critical"
                  onClick={() => void confirmStoreRecovery()}
                  disabled={
                    storeRecoveryBusy ||
                    storeRecoveryConfirmation !== STORE_RECOVERY_CONFIRMATION_TEXT
                  }
                  before={
                    storeRecoveryBusy && <Spinner variant="Critical" fill="Solid" size="200" />
                  }
                >
                  <Text as="span" size="B400">
                    Archive and Rebuild
                  </Text>
                </Button>
                <Button
                  type="button"
                  variant="Secondary"
                  fill="Soft"
                  onClick={closeStoreRecovery}
                  disabled={storeRecoveryBusy}
                >
                  <Text as="span" size="B400">
                    Cancel
                  </Text>
                </Button>
              </Box>
            </Box>
          </Dialog>
        </OverlayCenter>
      </Overlay>

      <Overlay
        open={
          loginState.status === AsyncStatus.Loading || loginState.status === AsyncStatus.Success
        }
        backdrop={<OverlayBackdrop />}
      >
        <OverlayCenter>
          <Spinner variant="Secondary" size="600" />
        </OverlayCenter>
      </Overlay>
    </Box>
  );
}
