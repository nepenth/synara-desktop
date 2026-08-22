import React from 'react';
import { Box, Text, IconButton, Icon, Icons, Scroll } from 'folds';
import { Page, PageContent, PageHeader } from '../../../components/page';
import { SequenceCard } from '../../../components/sequence-card';
import { SequenceCardStyle } from '../styles.css';
import { SettingTile } from '../../../components/setting-tile';
import { useDeviceList, useSplitCurrentDevice } from '../../../hooks/useDeviceList';
import { LocalBackup } from './LocalBackup';
import { DeviceLogoutBtn, DeviceTile, DeviceTilePlaceholder } from './DeviceTile';
import { OtherDevices } from './OtherDevices';
import { resolveDeviceVerificationStatus } from './deviceVerificationStatus';
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
  const [devices, refreshDeviceList] = useDeviceList();

  const [currentDevice, otherDevices] = useSplitCurrentDevice(devices);
  const verificationStatus = resolveDeviceVerificationStatus(
    currentDevice?.trust,
    crossSigning.nativeStatus?.ownIdentityVerification,
    crossSigning.loading
  );
  const unverifiedDeviceCount =
    otherDevices?.filter((device) => device.trust === 'unverified').length ?? 0;
  const offerCurrentVerification =
    canOfferNativeDeviceVerification(crossSigning.nativeStatus) &&
    verificationStatus !== 'verified';

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
                  {offerCurrentVerification && (
                    <VerifyCurrentDeviceTile onVerified={() => void refreshDeviceList()} />
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
              {devices === undefined && <DevicesPlaceholder />}
              {otherDevices && (
                <OtherDevices
                  devices={otherDevices}
                  refreshDeviceList={refreshDeviceList}
                  showVerification={crossSigningActive && verificationStatus === 'verified'}
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
