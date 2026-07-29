import { useCallback, useEffect, useState } from 'react';
import { useAlive } from './useAlive';
import { fulfilledPromiseSettledResult } from '../utils/common';
import { getNativeDeviceVerificationStatus } from '../features/verification/nativeVerification';

export enum VerificationStatus {
  Unknown,
  Unverified,
  Verified,
  Unsupported,
}

export const useDeviceVerificationDetect = (
  deviceId: string | undefined,
  callback: (status: VerificationStatus) => void
): void => {
  const updateStatus = useCallback(async () => {
    if (!deviceId) {
      callback(VerificationStatus.Unknown);
      return;
    }
    try {
      const status = await getNativeDeviceVerificationStatus(deviceId);
      callback(
        status === 'verified'
          ? VerificationStatus.Verified
          : status === 'unverified'
          ? VerificationStatus.Unverified
          : VerificationStatus.Unsupported
      );
    } catch {
      callback(VerificationStatus.Unsupported);
    }
  }, [deviceId, callback]);

  useEffect(() => {
    updateStatus();
  }, [updateStatus]);

  useEffect(() => {
    const interval = window.setInterval(() => void updateStatus(), 1_000);
    return () => window.clearInterval(interval);
  }, [updateStatus]);
};

export const useDeviceVerificationStatus = (deviceId: string | undefined): VerificationStatus => {
  const [verificationStatus, setVerificationStatus] = useState(VerificationStatus.Unknown);

  useDeviceVerificationDetect(deviceId, setVerificationStatus);

  return verificationStatus;
};

export const useUnverifiedDeviceCount = (devices: string[]): number | undefined => {
  const [unverifiedCount, setUnverifiedCount] = useState<number>();
  const alive = useAlive();

  const updateCount = useCallback(async () => {
    let count = 0;
    const result = await Promise.allSettled(
      devices.map((deviceId) => getNativeDeviceVerificationStatus(deviceId))
    );
    const settledResult = fulfilledPromiseSettledResult(result);
    settledResult.forEach((status) => {
      if (status === 'unverified') count += 1;
    });
    if (alive()) {
      setUnverifiedCount(count);
    }
  }, [devices, alive]);

  useEffect(() => {
    updateCount();
  }, [updateCount]);

  useEffect(() => {
    const interval = window.setInterval(() => void updateCount(), 1_000);
    return () => window.clearInterval(interval);
  }, [updateCount]);

  return unverifiedCount;
};
