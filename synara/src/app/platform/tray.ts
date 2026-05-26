import { setDesktopTrayState, type DesktopTrayState } from '../utils/desktop';

export type PlatformTrayState = DesktopTrayState;

export const setPlatformTrayState = setDesktopTrayState;
