import React, { useCallback, useEffect } from 'react';
import { Box, Button, Chip, color, Spinner, Switch, Text } from 'folds';
import { SettingTile } from '../../../components/setting-tile';
import { AsyncStatus, useAsyncCallback } from '../../../hooks/useAsyncCallback';
import {
  getNativeX509IdentityStatus,
  importNativeX509Ca,
  importNativeX509Signer,
  reloadNativeSession,
  removeNativeX509Ca,
  setNativeX509IdentityEnabled,
  type NativeX509IdentityStatus,
} from './nativeX509Identity';

const EMPTY_STATUS: NativeX509IdentityStatus = {
  enabled: false,
  hasCa: false,
  cas: [],
  signerImported: false,
  verifierConfigured: false,
  reloadRequired: false,
  certificateVerifiedIdentities: [],
};

export function X509IdentityCard() {
  const [loadState, loadStatus] = useAsyncCallback(
    useCallback(() => getNativeX509IdentityStatus(), [])
  );
  const [enableState, setEnabled] = useAsyncCallback(
    useCallback((enabled: boolean) => setNativeX509IdentityEnabled(enabled), [])
  );
  const [importState, importCa] = useAsyncCallback(useCallback(() => importNativeX509Ca(), []));
  const [removeState, removeCa] = useAsyncCallback(
    useCallback((fingerprint: string) => removeNativeX509Ca(fingerprint), [])
  );
  const [signerState, importSigner] = useAsyncCallback(
    useCallback(() => importNativeX509Signer(), [])
  );

  useEffect(() => {
    void loadStatus();
  }, [loadStatus]);

  const status =
    (enableState.status === AsyncStatus.Success && enableState.data) ||
    (importState.status === AsyncStatus.Success && importState.data) ||
    (removeState.status === AsyncStatus.Success && removeState.data) ||
    (signerState.status === AsyncStatus.Success && signerState.data) ||
    (loadState.status === AsyncStatus.Success ? loadState.data : EMPTY_STATUS);

  const busy =
    loadState.status === AsyncStatus.Loading ||
    enableState.status === AsyncStatus.Loading ||
    importState.status === AsyncStatus.Loading ||
    removeState.status === AsyncStatus.Loading ||
    signerState.status === AsyncStatus.Loading;

  const error =
    (enableState.status === AsyncStatus.Error && enableState.error.message) ||
    (importState.status === AsyncStatus.Error && importState.error.message) ||
    (removeState.status === AsyncStatus.Error && removeState.error.message) ||
    (signerState.status === AsyncStatus.Error && signerState.error.message) ||
    (loadState.status === AsyncStatus.Error && loadState.error.message) ||
    undefined;

  const enableHint = !status.hasCa
    ? 'Import a certificate authority first. Turning this on without a CA does nothing.'
    : undefined;

  return (
    <Box direction="Column" gap="400">
      <SettingTile
        title="X.509 identity (experimental)"
        description="Sit beside emoji verification. Trusts whoever the imported CA issued a Matrix ID for — including bots — via matrix:u/… or email SAN. Enabling this shares encrypted room keys with those users’ devices without an emoji prompt. A CA-re-signed identity reset is accepted without the usual pin warning; that is not the same as comparing emoji. Changes apply after a session reload."
        after={
          <Switch
            variant="Primary"
            value={status.enabled}
            onChange={(enabled) => void setEnabled(enabled)}
            disabled={busy}
          />
        }
      />
      {enableHint && (
        <Text size="T200" priority="300">
          {enableHint}
        </Text>
      )}
      <Box gap="200" wrap="Wrap">
        <Button
          size="300"
          radii="300"
          variant="Secondary"
          fill="Soft"
          disabled={busy}
          onClick={() => void importCa()}
        >
          <Text size="B300">Import CA</Text>
        </Button>
        <Button
          size="300"
          radii="300"
          variant="Secondary"
          fill="Soft"
          disabled={busy}
          onClick={() => void importSigner()}
        >
          <Text size="B300">{status.signerImported ? 'Replace signer' : 'Import signer'}</Text>
        </Button>
        {status.reloadRequired && (
          <Button
            size="300"
            radii="300"
            variant="Primary"
            onClick={() => reloadNativeSession()}
          >
            <Text size="B300">Reload session</Text>
          </Button>
        )}
        {busy && <Spinner size="200" variant="Secondary" />}
      </Box>
      {status.cas.length > 0 && (
        <Box direction="Column" gap="200">
          <Text size="L400">Imported certificate authorities</Text>
          {status.cas.map((ca) => (
            <Box key={ca.fingerprint} gap="200" alignItems="Center" wrap="Wrap">
              <Text size="T200" style={{ minWidth: 0, flex: '1 1 12rem' }}>
                {ca.label}
                <Text as="span" size="Inherit" priority="300">
                  {' '}
                  {ca.fingerprint}
                </Text>
              </Text>
              <Chip
                variant="Secondary"
                radii="Pill"
                onClick={() => void removeCa(ca.fingerprint)}
                disabled={busy}
              >
                <Text size="B300">Remove</Text>
              </Chip>
            </Box>
          ))}
        </Box>
      )}
      <Box direction="Column" gap="100">
        <Text size="L400">Certificate-verified identities</Text>
        {status.certificateVerifiedIdentities.length === 0 ? (
          <Text size="T200" priority="300">
            None while this is off, or no CA-signed users are in joined rooms yet.
          </Text>
        ) : (
          status.certificateVerifiedIdentities.map((userId) => (
            <Text key={userId} size="T200">
              {userId}
            </Text>
          ))
        )}
      </Box>
      {status.verifierConfigured && (
        <Text size="T200" priority="300">
          This session is currently using the imported CA for identity trust.
        </Text>
      )}
      {error && (
        <Text size="T200" style={{ color: color.Critical.Main }}>
          {error}
        </Text>
      )}
    </Box>
  );
}
