import React, { FormEventHandler, ReactNode, useCallback, useState } from 'react';
import { Box, Text, Button, Spinner, color } from 'folds';
import { PasswordInput } from './password-input';
import { AsyncStatus, useAsyncCallback } from '../hooks/useAsyncCallback';
import { useNativeSecretStorage } from '../hooks/useNativeSecretStorage';
import {
  bootstrapNativeSecretStorage,
  NativeSecretStorageOperationResult,
  NativeSecretStorageStatus,
  resetNativeSecretStorage,
  unlockNativeSecretStorage,
} from '../features/secret-storage/nativeSecretStorage';
import { InfoCard } from './info-card';

type NativeSecretStorageActionProps = {
  status: NativeSecretStorageStatus;
  onComplete?: (result: NativeSecretStorageOperationResult) => void;
  allowReset?: boolean;
};

export function NativeSecretStorageAction({
  status,
  onComplete,
  allowReset = false,
}: NativeSecretStorageActionProps) {
  const [resetting, setResetting] = useState(false);
  const action = resetting ? 'reset' : status.action;
  const [operationState, runOperation] = useAsyncCallback<
    NativeSecretStorageOperationResult,
    Error,
    [string]
  >(
    useCallback(
      async (value) => {
        if (action === 'bootstrap_required') return bootstrapNativeSecretStorage(value);
        if (action === 'unlock_required') return unlockNativeSecretStorage(value);
        return resetNativeSecretStorage(value);
      },
      [action]
    )
  );
  const working = operationState.status === AsyncStatus.Loading;
  const setup = action === 'bootstrap_required' || action === 'reset';

  const handleSubmit: FormEventHandler<HTMLFormElement> = (event) => {
    event.preventDefault();
    if (working) return;
    const form = event.currentTarget;
    const input = form.elements.namedItem('secretStorageInput') as HTMLInputElement | null;
    const confirm = form.elements.namedItem('confirmSecretStorageInput') as HTMLInputElement | null;
    if (!input?.value) return;
    if (confirm && input.value !== confirm.value) {
      confirm.setCustomValidity('Recovery passphrases do not match.');
      confirm.reportValidity();
      return;
    }
    confirm?.setCustomValidity('');
    runOperation(input.value)
      .then((result) => {
        input.value = '';
        if (confirm) confirm.value = '';
        setResetting(false);
        onComplete?.(result);
      })
      .catch(() => undefined);
  };

  if (action === 'none' && !resetting) {
    return allowReset ? (
      <Button size="300" fill="Soft" variant="Warning" onClick={() => setResetting(true)}>
        <Text size="B300">Replace Recovery Key</Text>
      </Button>
    ) : null;
  }

  return (
    <Box as="form" onSubmit={handleSubmit} direction="Column" gap="200">
      <Text size="T200">
        {action === 'unlock_required'
          ? 'Enter your recovery key or recovery passphrase to import encrypted account secrets into this device.'
          : 'Choose a recovery passphrase. Synara will also save a private recovery-key document directly to Downloads.'}
      </Text>
      <Box direction="Column" gap="100">
        <Text size="L400">
          {action === 'unlock_required' ? 'Recovery Key or Passphrase' : 'Recovery Passphrase'}
        </Text>
        <PasswordInput
          name="secretStorageInput"
          size="400"
          variant="Secondary"
          radii="300"
          required
          autoFocus
          readOnly={working}
        />
      </Box>
      {setup && (
        <Box direction="Column" gap="100">
          <Text size="L400">Confirm Recovery Passphrase</Text>
          <PasswordInput
            name="confirmSecretStorageInput"
            size="400"
            variant="Secondary"
            radii="300"
            required
            readOnly={working}
            onChange={(event) => event.currentTarget.setCustomValidity('')}
          />
        </Box>
      )}
      <Box gap="200">
        <Button
          type="submit"
          variant={action === 'reset' ? 'Warning' : 'Success'}
          disabled={working}
          before={working && <Spinner size="200" variant="Secondary" fill="Soft" />}
        >
          <Text size="B400">
            {action === 'unlock_required'
              ? 'Unlock & Import'
              : action === 'reset'
              ? 'Replace Recovery Key'
              : 'Set Up Recovery'}
          </Text>
        </Button>
        {action === 'reset' && (
          <Button type="button" fill="Soft" disabled={working} onClick={() => setResetting(false)}>
            <Text size="B400">Cancel</Text>
          </Button>
        )}
      </Box>
      {operationState.status === AsyncStatus.Error && (
        <Text size="T200" style={{ color: color.Critical.Main }}>
          <b>{operationState.error.message}</b>
        </Text>
      )}
      {operationState.status === AsyncStatus.Success &&
        operationState.data.recoveryDocumentSaved && (
          <Text size="T200" style={{ color: color.Success.Main }}>
            <b>
              Recovery is ready. {operationState.data.recoveryDocumentName} was saved privately to
              Downloads.
            </b>
          </Text>
        )}
    </Box>
  );
}

export function NativeSecretStorageTile() {
  const { status, loading, error, refresh } = useNativeSecretStorage();

  return (
    <InfoCard
      variant="Surface"
      title="Secure Backup & Recovery"
      description={
        status?.unlocked
          ? 'Secret storage is unlocked and account recovery secrets are available on this device.'
          : 'Set up or unlock secret storage to recover verification and encryption backup.'
      }
      after={
        loading ? (
          <Spinner size="100" variant="Secondary" fill="Soft" />
        ) : (
          <Text size="L400">{status?.unlocked ? 'Ready' : 'Action Required'}</Text>
        )
      }
    >
      {status && (
        <>
          <Text size="T200">
            Default key: {status.defaultKeySet ? 'configured' : 'not configured'} · Passphrase:{' '}
            {status.passphraseConfigured ? 'configured' : 'not configured'}
          </Text>
          {status.missingSecrets.length > 0 && (
            <Text size="T200">
              Missing protected data: {status.missingSecrets.length} required item
              {status.missingSecrets.length === 1 ? '' : 's'}.
            </Text>
          )}
          {status.action === 'bootstrap_required' && !status.bootstrapReady ? (
            <Text size="T200">
              Set up device verification first so its identity can be protected by secret storage.
            </Text>
          ) : (
            <NativeSecretStorageAction
              status={status}
              allowReset
              onComplete={() => {
                refresh();
              }}
            />
          )}
        </>
      )}
      {error && (
        <Text size="T200" style={{ color: color.Critical.Main }}>
          <b>{error}</b>
        </Text>
      )}
    </InfoCard>
  );
}

export function NativeSecretStorageGate({
  children,
}: {
  children: (refresh: () => void) => ReactNode;
}) {
  const { status, loading, error, refresh } = useNativeSecretStorage();
  if (loading) return <Spinner size="200" variant="Secondary" />;
  if (error || !status) {
    return (
      <Text size="T200" style={{ color: color.Critical.Main }}>
        Native secret storage is unavailable. Restart Synara and try again.
      </Text>
    );
  }
  if (!status.unlocked) {
    return <NativeSecretStorageAction status={status} onComplete={refresh} />;
  }
  return <>{children(refresh)}</>;
}
