import React from 'react';
import { Box, Button, Text, Spinner } from 'folds';
import { SequenceCard } from '../../../components/sequence-card';
import { SequenceCardStyle } from '../styles.css';
import { SettingTile } from '../../../components/setting-tile';
import { InfoCard } from '../../../components/info-card';
import { useDeviceList, useSplitCurrentDevice } from '../../../hooks/useDeviceList';
import { LocalBackup } from './LocalBackup';
import { DeviceLogoutBtn, DeviceTile, DeviceTilePlaceholder } from './DeviceTile';
import { OtherDevices } from './OtherDevices';
import {
  canStartCurrentDeviceVerification,
  resolveDeviceVerificationStatus,
} from './deviceVerificationStatus';
import {
  EnableVerification,
  VerificationStatusBadge,
  VerifyCurrentDeviceTile,
} from './Verification';
import { useCrossSigning } from '../../../hooks/useCrossSigning';
import { BackupRestoreTile } from '../../../components/BackupRestore';
import { isNativeMatrixSession } from '../../verification/nativeVerification';
import { canOfferNativeDeviceVerification } from '../../cross-signing/nativeCrossSigning';
import { NativeSecretStorageTile } from '../../../components/SecretStorage';
import { X509IdentityCard } from './X509IdentityCard';
import { SettingsPage, SettingsSection } from '../../../components/settings-layout';
import { IdentityResetButton } from '../../cross-signing/IdentityReset';
import { IDENTITY_RESET_COPY } from '../../cross-signing/nativeIdentityReset';

function DevicesPlaceholder() {
  return (
    <Box direction="Column" gap="100">
      <DeviceTilePlaceholder />
      <DeviceTilePlaceholder />
    </Box>
  );
}

type DevicesProps = {
  requestClose: () => void;
};

const useDeviceSnapshotState = () => {
  const crossSigning = useCrossSigning();
  const [deviceSnapshot, refreshDeviceList, deviceLoadState] = useDeviceList();
  const [currentDevice, otherDevices] = useSplitCurrentDevice(deviceSnapshot?.devices);
  const verificationStatus = resolveDeviceVerificationStatus(deviceSnapshot);
  // The verification prompt needs an authoritative snapshot; before the first
  // one lands we show the loading placeholder rather than a false "could not
  // check" failure, and a rejected snapshot gets its own retryable card.
  const snapshotFailed = deviceSnapshot === undefined && deviceLoadState.error !== undefined;
  const snapshotPending = deviceSnapshot === undefined && !snapshotFailed;
  return {
    crossSigning,
    deviceSnapshot,
    refreshDeviceList,
    deviceLoadState,
    currentDevice,
    otherDevices,
    verificationStatus,
    snapshotFailed,
    snapshotPending,
  };
};

/** Security & recovery: verification of this session, key backup, recovery, identity. */
export function SecurityRecovery({ requestClose }: DevicesProps) {
  const nativeSession = isNativeMatrixSession();
  const {
    crossSigning,
    deviceSnapshot,
    refreshDeviceList,
    deviceLoadState,
    otherDevices,
    verificationStatus,
    snapshotFailed,
  } = useDeviceSnapshotState();
  const crossSigningActive = crossSigning.active;
  const unverifiedDeviceCount =
    otherDevices?.filter((device) => device.trust === 'unverified').length ?? 0;
  const offerCurrentVerification =
    deviceSnapshot !== undefined &&
    canOfferNativeDeviceVerification(crossSigning.nativeStatus) &&
    verificationStatus !== 'verified';
  const canStartCurrentVerification = canStartCurrentDeviceVerification(deviceSnapshot);

  return (
    <SettingsPage
      title="Security & recovery"
      description="Verification and key backup protect your encrypted history."
      requestClose={requestClose}
    >
      <SettingsSection title="Verification">
        <SequenceCard
          className={SequenceCardStyle}
          variant="SurfaceVariant"
          direction="Column"
          gap="400"
        >
          <SettingTile
            title="Verify this session"
            description="Verify this session so other devices trust it and it can read your encrypted history."
            after={
              <>
                <EnableVerification
                  visible={!crossSigningActive}
                  nativeStatus={crossSigning.nativeStatus}
                  loading={crossSigning.loading}
                  error={crossSigning.error}
                />
                {crossSigningActive && (
                  <Box gap="200" alignItems="Center">
                    <VerificationStatusBadge
                      verificationStatus={verificationStatus}
                      otherUnverifiedCount={unverifiedDeviceCount}
                    />
                  </Box>
                )}
              </>
            }
          />
          {snapshotFailed && (
            <InfoCard
              variant="Critical"
              title="Device list unavailable"
              description={deviceLoadState.error}
              after={
                <Button
                  size="300"
                  radii="300"
                  disabled={deviceLoadState.fetching}
                  before={
                    deviceLoadState.fetching ? (
                      <Spinner size="100" variant="Secondary" fill="Soft" />
                    ) : undefined
                  }
                  onClick={() => void refreshDeviceList()}
                >
                  <Text as="span" size="B300">
                    Retry
                  </Text>
                </Button>
              }
            />
          )}
          {offerCurrentVerification && (
            <VerifyCurrentDeviceTile
              hasDevicesToVerifyAgainst={deviceSnapshot.hasDevicesToVerifyAgainst}
              canStart={canStartCurrentVerification}
              refreshing={deviceLoadState.fetching}
              onRetry={() => void refreshDeviceList()}
              onVerified={() => void refreshDeviceList()}
            />
          )}
        </SequenceCard>
        {nativeSession && (
          <SequenceCard
            className={SequenceCardStyle}
            variant="SurfaceVariant"
            direction="Column"
            gap="400"
          >
            <SettingTile
              title="Message history for new sessions"
              description="New sessions read older encrypted messages from your key backup. Sessions don't hand room keys to each other, because a forwarded key can't prove who sent the message. Keep key backup on and verify new logins."
            />
          </SequenceCard>
        )}
      </SettingsSection>
      {nativeSession && (
        <SettingsSection
          title="Recovery"
          description="Key backup and a recovery key let a new session read your history."
        >
          <SequenceCard
            className={SequenceCardStyle}
            variant="SurfaceVariant"
            direction="Column"
            gap="400"
          >
            <NativeSecretStorageTile />
            <BackupRestoreTile />
          </SequenceCard>
        </SettingsSection>
      )}
      {nativeSession && (
        <SettingsSection title="Certificate identities">
          <SequenceCard
            className={SequenceCardStyle}
            variant="SurfaceVariant"
            direction="Column"
            gap="400"
          >
            <X509IdentityCard />
          </SequenceCard>
        </SettingsSection>
      )}
      {nativeSession && crossSigningActive && (
        <SettingsSection
          title="Danger zone"
          description="Changes here cannot be undone."
          tone="critical"
        >
          <SequenceCard
            className={SequenceCardStyle}
            variant="SurfaceVariant"
            direction="Column"
            gap="400"
          >
            <SettingTile
              title={IDENTITY_RESET_COPY.title}
              description={IDENTITY_RESET_COPY.settingsDescription}
              after={<IdentityResetButton />}
            />
          </SequenceCard>
        </SettingsSection>
      )}
    </SettingsPage>
  );
}

/** Sessions: this session, the account's other sessions, and key export/import. */
export function Devices({ requestClose }: DevicesProps) {
  const { currentDevice, otherDevices, refreshDeviceList, verificationStatus, snapshotPending } =
    useDeviceSnapshotState();

  return (
    <SettingsPage
      title="Sessions"
      description="Every login to your account. Log out a session to revoke it on the homeserver."
      requestClose={requestClose}
    >
      <SettingsSection title="This session">
        {currentDevice ? (
          <SequenceCard
            className={SequenceCardStyle}
            variant="SurfaceVariant"
            direction="Column"
            gap="400"
          >
            <DeviceTile
              device={currentDevice}
              refreshDeviceList={refreshDeviceList}
              options={<DeviceLogoutBtn />}
            ></DeviceTile>
          </SequenceCard>
        ) : (
          <DeviceTilePlaceholder />
        )}
      </SettingsSection>
      {snapshotPending && <DevicesPlaceholder />}
      {otherDevices && (
        <OtherDevices
          devices={otherDevices}
          refreshDeviceList={refreshDeviceList}
          showVerification={verificationStatus === 'verified'}
        />
      )}
      <LocalBackup />
    </SettingsPage>
  );
}
