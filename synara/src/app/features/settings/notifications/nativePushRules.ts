import { invokeDesktopWithAvailability } from '../../../utils/desktop';

export type NativePushRuleMode = 'all' | 'mentions' | 'mute';

export type NativePushRuleMentions = {
  userMention: boolean;
  displayName: boolean;
  userName: boolean;
  roomMention: boolean;
  atRoom: boolean;
};

export type NativePushRulesSnapshot = {
  dm: NativePushRuleMode;
  dmEncrypted: NativePushRuleMode;
  group: NativePushRuleMode;
  groupEncrypted: NativePushRuleMode;
  mentions: NativePushRuleMentions;
  keywords: string[];
};

const isMode = (value: unknown): value is NativePushRuleMode =>
  value === 'all' || value === 'mentions' || value === 'mute';

export async function nativePushRulesSnapshot(): Promise<NativePushRulesSnapshot> {
  const result = await invokeDesktopWithAvailability<NativePushRulesSnapshot>(
    'matrix_push_rules_snapshot'
  );
  if (!result.available || !result.value) {
    throw new Error('Native push rules are unavailable.');
  }
  const body = result.value;
  if (
    !isMode(body.dm) ||
    !isMode(body.dmEncrypted) ||
    !isMode(body.group) ||
    !isMode(body.groupEncrypted) ||
    !body.mentions ||
    !Array.isArray(body.keywords)
  ) {
    throw new Error('Native push rules are unavailable.');
  }
  return {
    dm: body.dm,
    dmEncrypted: body.dmEncrypted,
    group: body.group,
    groupEncrypted: body.groupEncrypted,
    mentions: {
      userMention: Boolean(body.mentions.userMention),
      displayName: Boolean(body.mentions.displayName),
      userName: Boolean(body.mentions.userName),
      roomMention: Boolean(body.mentions.roomMention),
      atRoom: Boolean(body.mentions.atRoom),
    },
    keywords: body.keywords.filter((keyword) => typeof keyword === 'string' && keyword.length > 0),
  };
}

export async function nativePushRulesSetDefault(
  encrypted: boolean,
  oneToOne: boolean,
  mode: NativePushRuleMode
): Promise<void> {
  const result = await invokeDesktopWithAvailability<{ status: string }>(
    'matrix_push_rules_set_default',
    {
      encrypted,
      oneToOne,
      mode,
    }
  );
  if (!result.available || result.value?.status !== 'ok') {
    throw new Error('Native push-rule update is unavailable.');
  }
}

export async function nativePushRulesSetMention(ruleId: string, enabled: boolean): Promise<void> {
  const result = await invokeDesktopWithAvailability<{ status: string }>(
    'matrix_push_rules_set_mention',
    {
      ruleId,
      enabled,
    }
  );
  if (!result.available || result.value?.status !== 'ok') {
    throw new Error('Native mention-rule update is unavailable.');
  }
}

export async function nativePushRulesAddKeyword(keyword: string): Promise<void> {
  const result = await invokeDesktopWithAvailability<{ status: string }>(
    'matrix_push_rules_add_keyword',
    {
      keyword,
    }
  );
  if (!result.available || result.value?.status !== 'ok') {
    throw new Error('Native keyword update is unavailable.');
  }
}

export async function nativePushRulesRemoveKeyword(keyword: string): Promise<void> {
  const result = await invokeDesktopWithAvailability<{ status: string }>(
    'matrix_push_rules_remove_keyword',
    {
      keyword,
    }
  );
  if (!result.available || result.value?.status !== 'ok') {
    throw new Error('Native keyword update is unavailable.');
  }
}
