import {
  setDesktopTrayState,
  subscribeDesktopTrayDndToggle,
  type DesktopTrayState,
} from '../utils/desktop';

export type PlatformTrayState = DesktopTrayState;

export const setPlatformTrayState = setDesktopTrayState;
export const subscribePlatformTrayDndToggle = subscribeDesktopTrayDndToggle;
