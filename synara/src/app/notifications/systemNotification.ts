import { normalizeSynaraRoute } from '../routes/synaraRoutes';

export type SystemNotificationPrivacy = 'standard' | 'private';
export type SystemNotificationSoundPolicy = 'default' | 'silent';

export type SystemNotificationRequest = {
  title: string;
  body?: string;
  route?: string;
  privacy?: SystemNotificationPrivacy;
  sound?: SystemNotificationSoundPolicy;
};

const MAX_TITLE_LENGTH = 120;
const MAX_BODY_LENGTH = 1_000;

const normalizeText = (value: unknown, maxLength: number): string | undefined => {
  if (typeof value !== 'string') return undefined;
  const normalized = value.trim();
  if (!normalized) return undefined;
  return normalized.slice(0, maxLength);
};

const normalizeRoute = (value: unknown): string | undefined => {
  return normalizeSynaraRoute(value);
};

export const normalizeSystemNotificationRequest = (
  request: SystemNotificationRequest
): SystemNotificationRequest | undefined => {
  const title = normalizeText(request.title, MAX_TITLE_LENGTH);
  if (!title) return undefined;

  const body = normalizeText(request.body, MAX_BODY_LENGTH);
  const route = normalizeRoute(request.route);

  return {
    title,
    body,
    route,
    privacy: request.privacy ?? 'standard',
    sound: request.sound ?? 'default',
  };
};
