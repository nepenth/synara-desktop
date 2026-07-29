import React, { FormEventHandler, useCallback, useState } from 'react';
import { Box, Button, color, Icon, Icons, Spinner, Text, toRem } from 'folds';
import { SequenceCard } from '../../../components/sequence-card';
import { SettingTile } from '../../../components/setting-tile';
import { SequenceCardStyle } from '../styles.css';
import { PasswordInput } from '../../../components/password-input';
import { ConfirmPasswordMatch } from '../../../components/ConfirmPasswordMatch';
import { AsyncStatus, useAsyncCallback } from '../../../hooks/useAsyncCallback';
import { useAlive } from '../../../hooks/useAlive';
import {
  exportNativeRoomKeys,
  importNativeRoomKeys,
  NativeRoomKeyFileSelection,
  NativeRoomKeyTransferResult,
  selectNativeRoomKeyImport,
} from '../../room-keys/nativeRoomKeys';

function NativeExportKeys() {
  const alive = useAlive();
  const [result, setResult] = useState<NativeRoomKeyTransferResult>();
  const [exportState, exportKeys] = useAsyncCallback<void, Error, [string]>(
    useCallback(async (passphrase) => {
      setResult(await exportNativeRoomKeys(passphrase));
    }, [])
  );
  const exporting = exportState.status === AsyncStatus.Loading;

  const handleSubmit: FormEventHandler<HTMLFormElement> = (evt) => {
    evt.preventDefault();
    if (exporting) return;
    const { passwordInput, confirmPasswordInput } = evt.target as HTMLFormElement & {
      passwordInput: HTMLInputElement;
      confirmPasswordInput: HTMLInputElement;
    };
    if (passwordInput.value !== confirmPasswordInput.value) return;
    exportKeys(passwordInput.value).then(() => {
      if (alive()) {
        passwordInput.value = '';
        confirmPasswordInput.value = '';
      }
    });
  };

  return (
    <SettingTile>
      <Box as="form" onSubmit={handleSubmit} direction="Column" gap="100">
        <Box gap="200" alignItems="End">
          <ConfirmPasswordMatch initialValue>
            {(match, doMatch, passRef, confPassRef) => (
              <>
                <Box grow="Yes" direction="Column" gap="100">
                  <Text size="L400">New Password</Text>
                  <PasswordInput
                    ref={passRef}
                    name="passwordInput"
                    size="400"
                    variant="Secondary"
                    radii="300"
                    required
                    onChange={doMatch}
                    readOnly={exporting}
                    autoFocus
                  />
                </Box>
                <Box grow="Yes" direction="Column" gap="100">
                  <Text size="L400">Confirm Password</Text>
                  <PasswordInput
                    ref={confPassRef}
                    style={{ color: match ? undefined : color.Critical.Main }}
                    name="confirmPasswordInput"
                    size="400"
                    variant="Secondary"
                    radii="300"
                    required
                    onChange={doMatch}
                    readOnly={exporting}
                  />
                </Box>
              </>
            )}
          </ConfirmPasswordMatch>
          <Button
            type="submit"
            size="400"
            variant="Secondary"
            fill="Soft"
            outlined
            radii="300"
            disabled={exporting}
            before={exporting ? <Spinner size="200" variant="Secondary" fill="Soft" /> : undefined}
          >
            <Text as="span" size="B400">
              Export
            </Text>
          </Button>
        </Box>
        {result && (
          <Text size="T200">
            Exported {result.keysProcessed} keys from {result.roomsTouched} rooms to{' '}
            {result.fileLabel} in Downloads.
          </Text>
        )}
        {exportState.status === AsyncStatus.Error && (
          <Text size="T200" style={{ color: color.Critical.Main }}>
            <b>{exportState.error.message}</b>
          </Text>
        )}
      </Box>
    </SettingTile>
  );
}

function NativeExportKeysTile() {
  const [expand, setExpand] = useState(false);
  return (
    <>
      <SettingTile
        title="Export Messages Data"
        description="Save a password-protected copy of room keys directly to Downloads."
        after={
          <Button
            type="button"
            onClick={() => setExpand(!expand)}
            size="300"
            variant="Secondary"
            fill="Soft"
            outlined
            radii="300"
            before={
              <Icon size="100" src={expand ? Icons.ChevronTop : Icons.ChevronBottom} filled />
            }
          >
            <Text as="span" size="B300">
              {expand ? 'Collapse' : 'Expand'}
            </Text>
          </Button>
        }
      />
      {expand && <NativeExportKeys />}
    </>
  );
}

function NativeImportKeys({
  selection,
  onDone,
}: {
  selection: NativeRoomKeyFileSelection;
  onDone: () => void;
}) {
  const alive = useAlive();
  const [importState, importKeys] = useAsyncCallback<NativeRoomKeyTransferResult, Error, [string]>(
    useCallback(
      (passphrase) => importNativeRoomKeys(selection.selectionId, passphrase),
      [selection.selectionId]
    )
  );
  const importing = importState.status === AsyncStatus.Loading;
  const handleSubmit: FormEventHandler<HTMLFormElement> = (evt) => {
    evt.preventDefault();
    if (importing) return;
    const { passwordInput } = evt.target as HTMLFormElement & {
      passwordInput: HTMLInputElement;
    };
    importKeys(passwordInput.value).then(() => {
      if (alive()) passwordInput.value = '';
    });
  };

  return (
    <SettingTile>
      <Box as="form" onSubmit={handleSubmit} direction="Column" gap="100">
        <Box gap="200" alignItems="End">
          <Box grow="Yes" direction="Column" gap="100">
            <Text size="L400">Password</Text>
            <PasswordInput
              name="passwordInput"
              size="400"
              variant="Secondary"
              radii="300"
              required
              autoFocus
              readOnly={importing}
            />
          </Box>
          <Button
            type="submit"
            size="400"
            variant="Secondary"
            fill="Soft"
            outlined
            radii="300"
            disabled={importing}
            before={importing ? <Spinner size="200" variant="Secondary" fill="Soft" /> : undefined}
          >
            <Text as="span" size="B400">
              Decrypt and Import
            </Text>
          </Button>
        </Box>
        {importState.status === AsyncStatus.Success && (
          <Box direction="Column" gap="100">
            <Text size="T200">
              Imported {importState.data.keysProcessed} of{' '}
              {importState.data.totalKeysFound ?? importState.data.keysProcessed} keys.
            </Text>
            <Button type="button" size="300" variant="Secondary" fill="Soft" onClick={onDone}>
              <Text as="span" size="B300">
                Done
              </Text>
            </Button>
          </Box>
        )}
        {importState.status === AsyncStatus.Error && (
          <Text size="T200" style={{ color: color.Critical.Main }}>
            <b>{importState.error.message}</b>
          </Text>
        )}
      </Box>
    </SettingTile>
  );
}

function NativeImportKeysTile() {
  const [selection, setSelection] = useState<NativeRoomKeyFileSelection>();
  const [selectionError, setSelectionError] = useState<string>();
  const [selecting, setSelecting] = useState(false);
  const selectFile = async () => {
    setSelecting(true);
    setSelectionError(undefined);
    try {
      const selected = await selectNativeRoomKeyImport();
      if (selected) setSelection(selected);
    } catch (error) {
      setSelectionError(error instanceof Error ? error.message : 'Room-key file selection failed.');
    } finally {
      setSelecting(false);
    }
  };

  return (
    <>
      <SettingTile
        title="Import Messages Data"
        description="Choose an encrypted room-key file; decryption and import stay in the native host."
        after={
          selection ? (
            <Button
              style={{ maxWidth: toRem(200) }}
              type="button"
              onClick={() => setSelection(undefined)}
              size="300"
              variant="Warning"
              fill="Solid"
              radii="300"
              before={<Icon size="100" src={Icons.File} filled />}
              after={<Icon size="100" src={Icons.Cross} />}
            >
              <Text as="span" size="B300" truncate>
                {selection.fileLabel}
              </Text>
            </Button>
          ) : (
            <Button
              type="button"
              onClick={selectFile}
              size="300"
              variant="Secondary"
              fill="Soft"
              outlined
              radii="300"
              disabled={selecting}
              before={
                selecting ? (
                  <Spinner size="200" variant="Secondary" fill="Soft" />
                ) : (
                  <Icon size="100" src={Icons.ArrowRight} />
                )
              }
            >
              <Text as="span" size="B300">
                Choose File
              </Text>
            </Button>
          )
        }
      />
      {selectionError && (
        <Text size="T200" style={{ color: color.Critical.Main }}>
          <b>{selectionError}</b>
        </Text>
      )}
      {selection && (
        <NativeImportKeys selection={selection} onDone={() => setSelection(undefined)} />
      )}
    </>
  );
}

export function LocalBackup() {
  return (
    <Box direction="Column" gap="100">
      <Text size="L400">Local Backup</Text>
      <SequenceCard
        className={SequenceCardStyle}
        variant="SurfaceVariant"
        direction="Column"
        gap="400"
      >
        <NativeExportKeysTile />
      </SequenceCard>
      <SequenceCard
        className={SequenceCardStyle}
        variant="SurfaceVariant"
        direction="Column"
        gap="400"
      >
        <NativeImportKeysTile />
      </SequenceCard>
    </Box>
  );
}
