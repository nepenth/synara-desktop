import { ReactNode } from 'react';
import {
  useDeviceVerificationStatus,
  VerificationStatus,
} from '../hooks/useDeviceVerificationStatus';

type DeviceVerificationStatusProps = {
  deviceId: string;
  children: (verificationStatus: VerificationStatus) => ReactNode;
};

export function DeviceVerificationStatus({ deviceId, children }: DeviceVerificationStatusProps) {
  const status = useDeviceVerificationStatus(deviceId);

  return children(status);
}
