import { UAParser } from 'ua-parser-js';

export const ua = () => UAParser(window.navigator.userAgent);

export const isMacOS = () => ua().os.name === 'Mac OS';

export const isLinuxOS = () => ua().os.name === 'Linux';

export const synaraDeviceDisplayName = (): string => {
  const osName = ua().os.name;

  if (osName === 'Mac OS') return 'Synara macOS';
  if (osName === 'Linux') return 'Synara Linux';
  if (osName === 'Windows') return 'Synara Windows';
  if (osName === 'iOS') return 'Synara iOS';
  if (osName === 'Android') return 'Synara Android';

  return 'Synara Desktop';
};

export const mobileOrTablet = (): boolean => {
  const userAgent = ua();
  const { os, device } = userAgent;
  if (device.type === 'mobile' || device.type === 'tablet') return true;
  if (os.name === 'Android' || os.name === 'iOS') return true;
  return false;
};
