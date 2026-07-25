/**
 * Widget / Element Call session DTO.
 */

import type { RoomId, WidgetId } from './ids';
import {
  hasForbiddenWireFields,
  isObject,
  optString,
  reqBoolean,
  reqString,
} from './parseUtil';

export const WIDGET_KINDS = ['element_call', 'custom'] as const;
export type WidgetKind = (typeof WIDGET_KINDS)[number];
const KIND_SET = new Set<string>(WIDGET_KINDS);

export const WIDGET_SESSION_STATES = [
  'idle',
  'creating',
  'active',
  'ending',
  'failed',
] as const;
export type WidgetSessionState = (typeof WIDGET_SESSION_STATES)[number];
const STATE_SET = new Set<string>(WIDGET_SESSION_STATES);

export type WidgetSession = {
  widgetId: WidgetId;
  roomId: RoomId;
  kind: WidgetKind;
  state: WidgetSessionState;
  url?: string;
  hasActiveCall: boolean;
};

export function parseWidgetSession(value: unknown): WidgetSession | null {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const widgetId = reqString(value, 'widgetId');
  const roomId = reqString(value, 'roomId');
  const url = optString(value, 'url');
  const hasActiveCall = reqBoolean(value, 'hasActiveCall');
  if (
    widgetId === null ||
    roomId === null ||
    url === null ||
    hasActiveCall === null ||
    typeof value.kind !== 'string' ||
    !KIND_SET.has(value.kind) ||
    typeof value.state !== 'string' ||
    !STATE_SET.has(value.state)
  ) {
    return null;
  }
  return {
    widgetId,
    roomId,
    kind: value.kind as WidgetKind,
    state: value.state as WidgetSessionState,
    url,
    hasActiveCall,
  };
}
