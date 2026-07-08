import { normalizeSynaraRoute } from '../routes/synaraRoutes';

export type SystemNotificationPrivacy = 'standard' | 'private';
export type SystemNotificationSoundPolicy = 'default' | 'silent';

export type SystemNotificationAction = {
  id: string;
  label: string;
};

export type SystemNotificationActionContext = {
  kind: string;
  roomId?: string;
  eventId?: string;
};

export type SystemNotificationRequest = {
  title: string;
  body?: string;
  route?: string;
  actions?: SystemNotificationAction[];
  actionContext?: SystemNotificationActionContext;
  privacy?: SystemNotificationPrivacy;
  sound?: SystemNotificationSoundPolicy;
};

const MAX_TITLE_LENGTH = 120;
const MAX_BODY_LENGTH = 1_000;
const MAX_ACTION_ID_LENGTH = 96;
const MAX_ACTION_LABEL_LENGTH = 80;
const MAX_ACTION_CONTEXT_LENGTH = 255;
const MAX_NOTIFICATION_ACTIONS = 4;
const ACTION_ID_RE = /^[a-z0-9._:-]+$/i;

const normalizeText = (value: unknown, maxLength: number): string | undefined => {
  if (typeof value !== 'string') return undefined;
  const normalized = value.trim();
  if (!normalized) return undefined;
  return normalized.slice(0, maxLength);
};

const normalizeRoute = (value: unknown): string | undefined => {
  return normalizeSynaraRoute(value);
};

const normalizeNotificationActions = (
  actions: SystemNotificationAction[] | undefined
): SystemNotificationAction[] | undefined => {
  if (!Array.isArray(actions)) return undefined;

  const normalized: SystemNotificationAction[] = [];
  const seen = new Set<string>();

  actions.slice(0, MAX_NOTIFICATION_ACTIONS).forEach((action) => {
    const id = normalizeText(action.id, MAX_ACTION_ID_LENGTH);
    const label = normalizeText(action.label, MAX_ACTION_LABEL_LENGTH);
    if (!id || !label || !ACTION_ID_RE.test(id) || seen.has(id)) return;
    seen.add(id);
    normalized.push({ id, label });
  });

  return normalized.length > 0 ? normalized : undefined;
};

const normalizeActionContext = (
  context: SystemNotificationActionContext | undefined
): SystemNotificationActionContext | undefined => {
  if (!context || typeof context !== 'object') return undefined;

  const kind = normalizeText(context.kind, MAX_ACTION_CONTEXT_LENGTH);
  if (!kind) return undefined;

  const roomId = normalizeText(context.roomId, MAX_ACTION_CONTEXT_LENGTH);
  const eventId = normalizeText(context.eventId, MAX_ACTION_CONTEXT_LENGTH);
  return {
    kind,
    roomId,
    eventId,
  };
};

export const normalizeSystemNotificationRequest = (
  request: SystemNotificationRequest
): SystemNotificationRequest | undefined => {
  const title = normalizeText(request.title, MAX_TITLE_LENGTH);
  if (!title) return undefined;

  const body = normalizeText(request.body, MAX_BODY_LENGTH);
  const route = normalizeRoute(request.route);
  const actions = normalizeNotificationActions(request.actions);
  const actionContext = actions ? normalizeActionContext(request.actionContext) : undefined;

  const normalized: SystemNotificationRequest = {
    title,
    body,
    route,
    privacy: request.privacy ?? 'standard',
    sound: request.sound ?? 'default',
  };
  if (actions) {
    normalized.actions = actions;
    normalized.actionContext = actionContext;
  }
  return normalized;
};
