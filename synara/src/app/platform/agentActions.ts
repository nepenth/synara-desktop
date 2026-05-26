import { sendDesktopAgentAction, type DesktopAgentActionPayload } from '../utils/desktop';

export type PlatformAgentActionPayload = DesktopAgentActionPayload;

export const sendPlatformAgentAction = sendDesktopAgentAction;
