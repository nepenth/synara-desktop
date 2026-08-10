/**
 * SDK-neutral structural projection + literal constants for Matrix push rules
 * (mirrors the subset of matrix-js-sdk/lib/@types/PushRules used by Synara).
 * Shapes are kept identical to the SDK so values flow both ways at the few
 * remaining js-sdk call boundaries (client.setPushRuleActions/addPushRule/...).
 */

export const PushRuleActionName = {
  DontNotify: 'dont_notify',
  Notify: 'notify',
  Coalesce: 'coalesce',
} as const;
export type PushRuleActionName = typeof PushRuleActionName[keyof typeof PushRuleActionName];

export const TweakName = {
  Highlight: 'highlight',
  Sound: 'sound',
} as const;
export type TweakName = typeof TweakName[keyof typeof TweakName];

export type PushRuleAction =
  | PushRuleActionName
  | { set_tweak: 'highlight'; value?: boolean }
  | { set_tweak: 'sound'; value?: string };

export const ConditionKind = {
  EventMatch: 'event_match',
  EventPropertyIs: 'event_property_is',
  EventPropertyContains: 'event_property_contains',
  ContainsDisplayName: 'contains_display_name',
  RoomMemberCount: 'room_member_count',
  SenderNotificationPermission: 'sender_notification_permission',
} as const;

export type PushRuleCondition =
  | { [k: string]: any; kind: 'event_match'; key: string; pattern?: string; value?: string }
  | {
      [k: string]: any;
      kind: 'event_property_is';
      key: string;
      value: string | boolean | null | number;
    }
  | {
      [k: string]: any;
      kind: 'event_property_contains';
      key: string;
      value: string | boolean | null | number;
    }
  | { [k: string]: any; kind: 'room_member_count'; is: string }
  | { [k: string]: any; kind: 'contains_display_name' }
  | { [k: string]: any; kind: 'sender_notification_permission'; key: string }
  | { [k: string]: any; kind: string };

export const PushRuleKind = {
  Override: 'override',
  ContentSpecific: 'content',
  RoomSpecific: 'room',
  SenderSpecific: 'sender',
  Underride: 'underride',
} as const;
export type PushRuleKind = typeof PushRuleKind[keyof typeof PushRuleKind];

export type RuleId = string;
export const RuleId = {
  Master: '.m.rule.master',
  IsUserMention: '.m.rule.is_user_mention',
  IsRoomMention: '.m.rule.is_room_mention',
  ContainsDisplayName: '.m.rule.contains_display_name',
  ContainsUserName: '.m.rule.contains_user_name',
  AtRoomNotification: '.m.rule.roomnotif',
  DM: '.m.rule.room_one_to_one',
  EncryptedDM: '.m.rule.encrypted_room_one_to_one',
  Message: '.m.rule.message',
  EncryptedMessage: '.m.rule.encrypted',
  InviteToSelf: '.m.rule.invite_for_me',
  MemberEvent: '.m.rule.member_event',
  IncomingCall: '.m.rule.call',
  SuppressNotices: '.m.rule.suppress_notices',
  Tombstone: '.m.rule.tombstone',
} as const;

export type IPushRule = {
  actions: PushRuleAction[];
  conditions?: PushRuleCondition[];
  default: boolean;
  enabled: boolean;
  pattern?: string;
  rule_id: RuleId | string;
};

export type IPushRules = {
  global: { [k in PushRuleKind]?: IPushRule[] };
  device?: { [k in PushRuleKind]?: IPushRule[] };
};

export type IPusherRequest = {
  app_display_name: string;
  app_id: string;
  data: { format?: string; url?: string; brand?: string };
  device_display_name: string;
  kind: string;
  lang: string;
  profile_tag?: string;
  pushkey: string;
  enabled?: boolean | null;
  append?: boolean;
};

/**
 * The js-sdk client methods used to mutate push rules take nominal SDK enums
 * (PushRuleKind) that plain string literals do not satisfy. This structural
 * client projection re-types only those methods so the slice's own literal
 * constants stay the source of truth; the runtime object is unchanged.
 */
export type PushRuleClient = {
  addPushRule(
    scope: string,
    kind: PushRuleKind,
    ruleId: string,
    body: { actions?: PushRuleAction[]; conditions?: PushRuleCondition[]; pattern?: string }
  ): Promise<unknown>;
  deletePushRule(scope: string, kind: PushRuleKind, ruleId: string): Promise<unknown>;
  setPushRuleActions(
    scope: string,
    kind: PushRuleKind,
    ruleId: string,
    actions: PushRuleAction[]
  ): Promise<unknown>;
};

export const asPushRuleClient = (mx: unknown): PushRuleClient => mx as unknown as PushRuleClient;
