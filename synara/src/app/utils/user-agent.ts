import { UAParser } from 'ua-parser-js';

export const ua = () => UAParser(window.navigator.userAgent);

const SHELL_OS_NAMES: Record<string, string> = {
  linux: 'Linux',
  macos: 'macOS',
  windows: 'Windows',
};

/**
 * Operating system name, preferring the desktop shell's own report.
 *
 * WebKitGTK can send a Safari "Macintosh" user agent for site compatibility,
 * so inside the shell the user agent is not evidence of the OS. The shell
 * publishes `std::env::consts::OS` on the bridge; browsers fall back to the
 * user agent.
 */
export const osName = (): string | undefined => {
  const shellOs = window.__SYNARA_DESKTOP__?.os;
  if (typeof shellOs === 'string' && shellOs in SHELL_OS_NAMES) return SHELL_OS_NAMES[shellOs];
  return ua().os.name;
};

export const isMacOS = () => osName() === 'macOS';

export const isLinuxOS = () => osName() === 'Linux';

export const synaraDeviceDisplayName = (): string => {
  const name = osName();

  if (name === 'macOS') return 'Synara macOS';
  if (name === 'Linux') return 'Synara Linux';
  if (name === 'Windows') return 'Synara Windows';
  if (name === 'iOS') return 'Synara iOS';
  if (name === 'Android') return 'Synara Android';

  return 'Synara Desktop';
};

export const mobileOrTablet = (): boolean => {
  const userAgent = ua();
  const { os, device } = userAgent;
  if (device.type === 'mobile' || device.type === 'tablet') return true;
  if (os.name === 'Android' || os.name === 'iOS') return true;
  return false;
};
