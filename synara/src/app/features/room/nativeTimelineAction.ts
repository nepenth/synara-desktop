/**
 * Desktop wrappers for native timeline action owners.
 * Does not select NativeTimelinePresenter or delete RoomTimeline.
 */

import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import {
  forwardMediaWithNativeTimelineOwner,
  forwardTextWithNativeTimelineOwner,
  pinWithNativeTimelineOwner,
  redactWithNativeTimelineOwner,
  reportWithNativeTimelineOwner,
  unpinWithNativeTimelineOwner,
  type NativeTimelineActionReadback,
  type NativeTimelineForwardMediaInput,
  type NativeTimelineForwardTextInput,
  type NativeTimelinePinInput,
  type NativeTimelineRedactInput,
  type NativeTimelineReportInput,
} from './nativeTimelineActions';

const invoke: Parameters<typeof redactWithNativeTimelineOwner>[2] = (command, args) =>
  invokeDesktopWithAvailability(command, args);

const requireNative = async <T>(result: Promise<T | 'unavailable'>): Promise<T> => {
  const value = await result;
  if (value === 'unavailable') {
    throw new Error('Native Matrix timeline action is unavailable.');
  }
  return value;
};

export const redactWithNativeTimelineAction = (
  input: NativeTimelineRedactInput
): Promise<NativeTimelineActionReadback> =>
  requireNative(redactWithNativeTimelineOwner(input, isSynaraDesktop(), invoke));

export const reportWithNativeTimelineAction = (
  input: NativeTimelineReportInput
): Promise<NativeTimelineActionReadback> =>
  requireNative(reportWithNativeTimelineOwner(input, isSynaraDesktop(), invoke));

export const pinWithNativeTimelineAction = (
  input: NativeTimelinePinInput
): Promise<NativeTimelineActionReadback> =>
  requireNative(pinWithNativeTimelineOwner(input, isSynaraDesktop(), invoke));

export const unpinWithNativeTimelineAction = (
  input: NativeTimelinePinInput
): Promise<NativeTimelineActionReadback> =>
  requireNative(unpinWithNativeTimelineOwner(input, isSynaraDesktop(), invoke));

export const forwardTextWithNativeTimelineAction = (
  input: NativeTimelineForwardTextInput
): Promise<NativeTimelineActionReadback> =>
  requireNative(forwardTextWithNativeTimelineOwner(input, isSynaraDesktop(), invoke));

export const forwardMediaWithNativeTimelineAction = (
  input: NativeTimelineForwardMediaInput
): Promise<NativeTimelineActionReadback> =>
  requireNative(forwardMediaWithNativeTimelineOwner(input, isSynaraDesktop(), invoke));
