import React from 'react';
import { Badge, Box, Button, Text, Spinner } from 'folds';
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
export function Devices({ requestClose }: DevicesProps) {
  const nativeSession = isNativeMatrixSession();
  const crossSigning = useCrossSigning();
  const crossSigningActive = crossSigning.active;
  const [deviceSnapshot, refreshDeviceList, deviceLoadState] = useDeviceList();
  const devices = deviceSnapshot?.devices;

  const [currentDevice, otherDevices] = useSplitCurrentDevice(devices);
  const verificationStatus = resolveDeviceVerificationStatus(deviceSnapshot);
  const unverifiedDeviceCount =
    otherDevices?.filter((device) => device.trust === 'unverified').length ?? 0;
  // The verification prompt needs an authoritative snapshot; before the first
  // one lands we show the loading placeholder rather than a false "could not
  // check" failure, and a rejected snapshot gets its own retryable card.
  const snapshotFailed = deviceSnapshot === undefined && deviceLoadState.error !== undefined;
  const snapshotPending = deviceSnapshot === undefined && !snapshotFailed;
  const offerCurrentVerification =
    deviceSnapshot !== undefined &&
    canOfferNativeDeviceVerification(crossSigning.nativeStatus) &&
    verificationStatus !== 'verified';
  const canStartCurrentVerification = canStartCurrentDeviceVerification(deviceSnapshot);

  return (
    <SettingsPage
      title="Devices"
      description="Signed-in devices, verification and encrypted message backup."
      requestClose={requestClose}
    >
      <SettingsSection title="Security">
        <SequenceCard
          className={SequenceCardStyle}
          variant="SurfaceVariant"
          direction="Column"
          gap="400"
        >
          <SettingTile
            title="Device Verification"
            description="To verify device identity and grant access to encrypted messages."
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
          {nativeSession && (
            <SettingTile
              title="Message history for new sessions"
              description="New sessions read older encrypted messages from your key backup. Sessions don't hand room keys to each other, because a forwarded key can't prove who sent the message. Keep key backup on and verify new logins."
              after={
                <Badge variant="Secondary" fill="Soft" radii="Pill" outlined>
                  <Text as="span" size="L400">
                    Key backup
                  </Text>
                </Badge>
              }
            />
          )}
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
            <X509IdentityCard />
          </SequenceCard>
        )}
        {nativeSession && (
          <SequenceCard
            className={SequenceCardStyle}
            variant="SurfaceVariant"
            direction="Column"
            gap="400"
          >
            <NativeSecretStorageTile />
            <BackupRestoreTile />
          </SequenceCard>
        )}
      </SettingsSection>
      <SettingsSection title="Current">
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
