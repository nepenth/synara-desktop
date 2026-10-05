import assert from 'node:assert/strict';
import test from 'node:test';
import {
  acceptsAgentNotificationPreferences,
  loadAgentNotificationPreferences,
  saveAgentNotificationPreferences,
} from '../nativeAgentNotificationPreferences';

const preferences = {
  schemaVersion: 1 as const,
  agentUserIds: ['@hermes:example.org'],
  notifyToolActivity: false,
  notifyCommentary: false,
  notifyFinalResponses: true,
};

test('settings adapter rejects unsupported schemas and incomplete category state', () => {
  assert.equal(acceptsAgentNotificationPreferences(preferences), true);
  assert.equal(acceptsAgentNotificationPreferences({ ...preferences, schemaVersion: 2 }), false);
  assert.equal(
    acceptsAgentNotificationPreferences({ ...preferences, notifyToolActivity: 'false' }),
    false
  );
  assert.equal(
    acceptsAgentNotificationPreferences({ ...preferences, agentUserIds: [null] }),
    false
  );
  assert.equal(acceptsAgentNotificationPreferences({ schemaVersion: 1 }), false);
});

test('settings read rejects unavailable native owner without silently fabricating defaults', async () => {
  await assert.rejects(
    loadAgentNotificationPreferences(async () => ({ available: false })),
    /could not be loaded/
  );
  await assert.rejects(
    loadAgentNotificationPreferences(async () => ({ available: true, value: null })),
    /could not be loaded/
  );
});

test('settings save uses native account-data owner and returns confirmed server state', async () => {
  const confirmed = { ...preferences, agentUserIds: ['@hermes:example.org', '@forge:example.org'] };
  const saved = await saveAgentNotificationPreferences(preferences, async (command, args) => {
    assert.equal(command, 'matrix_agent_notification_preferences_set');
    assert.deepEqual(args, { preferences });
    return { available: true, value: confirmed };
  });
  assert.deepEqual(saved, confirmed);
  await assert.rejects(
    saveAgentNotificationPreferences(preferences, async () => ({ available: false })),
    /were not saved/
  );
  await assert.rejects(
    saveAgentNotificationPreferences(preferences, async () => ({
      available: true,
      value: { status: 'ok' },
    })),
    /were not saved/
  );
});
