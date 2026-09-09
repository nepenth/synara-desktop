import React from 'react';
import { Box, Button, Text, IconButton, Icon, Icons, Scroll, Spinner } from 'folds';
import { Page, PageContent, PageHeader } from '../../../components/page';
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
    <Page>
      <PageHeader outlined={false}>
        <Box grow="Yes" gap="200">
          <Box grow="Yes" alignItems="Center" gap="200">
            <Text size="H3" truncate>
              Devices
            </Text>
          </Box>
          <Box shrink="No">
            <IconButton onClick={requestClose} variant="Surface">
              <Icon src={Icons.Cross} />
            </IconButton>
          </Box>
        </Box>
      </PageHeader>
      <Box grow="Yes">
        <Scroll hideTrack visibility="Hover">
          <PageContent>
            <Box direction="Column" gap="700">
              <Box direction="Column" gap="100">
                <Text size="L400">Security</Text>
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
                    <NativeSecretStorageTile />
                    <BackupRestoreTile />
                  </SequenceCard>
                )}
              </Box>
              <Box direction="Column" gap="100">
                <Text size="L400">Current</Text>
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
              </Box>
              {snapshotPending && <DevicesPlaceholder />}
              {otherDevices && (
                <OtherDevices
                  devices={otherDevices}
                  refreshDeviceList={refreshDeviceList}
                  showVerification={verificationStatus === 'verified'}
                />
              )}
              <LocalBackup />
            </Box>
          </PageContent>
        </Scroll>
      </Box>
    </Page>
  );
}
