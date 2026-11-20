import React, { useEffect, useState } from 'react';
import { Switch } from 'folds';
import { useTranslation } from 'react-i18next';
import { SequenceCard } from '../../../components/sequence-card';
import { useSetting } from '../../../state/hooks/settings';
import { settingsAtom } from '../../../state/settings';
import { SettingTile } from '../../../components/setting-tile';
import { KeySymbol } from '../../../utils/key-symbol';
import { isMacOS } from '../../../utils/user-agent';
import { SequenceCardStyle } from '../styles.css';
import { InlineMessage } from '../../../components/inline-message';
import { isSynaraDesktop } from '../../../utils/desktop';
import {
  draftSyncStatus,
  subscribeSyncedDrafts,
  type DraftKeyState,
} from '../../room/nativeDraftSync';
import { SettingsPage, SettingsSection } from '../../../components/settings-layout';

function Composer() {
  const [enterForNewline, setEnterForNewline] = useSetting(settingsAtom, 'enterForNewline');
  const [isMarkdown, setIsMarkdown] = useSetting(settingsAtom, 'isMarkdown');

  return (
    <SettingsSection title="Composer">
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Enter adds a new line"
          description={`Enter starts a new line and ${
            isMacOS() ? KeySymbol.Command : 'Ctrl'
          }+Enter sends the message.`}
          after={<Switch variant="Primary" value={enterForNewline} onChange={setEnterForNewline} />}
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Markdown formatting"
          after={<Switch variant="Primary" value={isMarkdown} onChange={setIsMarkdown} />}
        />
      </SequenceCard>
    </SettingsSection>
  );
}

function Timeline() {
  const [hideMembershipEvents, setHideMembershipEvents] = useSetting(
    settingsAtom,
    'hideMembershipEvents'
  );
  const [hideNickAvatarEvents, setHideNickAvatarEvents] = useSetting(
    settingsAtom,
    'hideNickAvatarEvents'
  );

  return (
    <SettingsSection title="Timeline">
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Hide membership changes"
          description="Do not show join, leave, and invite events in the timeline."
          after={
            <Switch
              variant="Primary"
              value={hideMembershipEvents}
              onChange={setHideMembershipEvents}
            />
          }
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Hide profile changes"
          description="Do not show display name and avatar changes in the timeline."
          after={
            <Switch
              variant="Primary"
              value={hideNickAvatarEvents}
              onChange={setHideNickAvatarEvents}
            />
          }
        />
      </SequenceCard>
    </SettingsSection>
  );
}

/** Says why drafts aren't syncing; without the key, sync is off silently. */
function DraftSyncKeyHint({ enabled }: { enabled: boolean }) {
  const { t } = useTranslation();
  const [keyState, setKeyState] = useState<DraftKeyState>();
  useEffect(() => {
    if (!enabled || !isSynaraDesktop()) {
      setKeyState(undefined);
      return undefined;
    }
    let live = true;
    let unsubscribe: (() => void) | undefined;
    const refresh = () => {
      void draftSyncStatus().then((status) => {
        if (live) setKeyState(status?.keyState);
      });
    };
    refresh();
    // Core announces a key that arrived or was created as an update for every room.
    void subscribeSyncedDrafts((roomId) => {
      if (roomId === undefined) refresh();
    }).then((stop) => {
      if (live) unsubscribe = stop;
      else stop();
    });
    return () => {
      live = false;
      unsubscribe?.();
    };
  }, [enabled]);
  if (keyState === 'needsRecovery') {
    return (
      <InlineMessage tone="info">
        {t(
          'modernization.settings.sync_drafts.needs_recovery',
          'Not syncing yet: this device is not verified. Verify it with another device or your recovery key under Security & recovery.'
        )}
      </InlineMessage>
    );
  }
  if (keyState === 'requesting') {
    return (
      <InlineMessage tone="info">
        {t(
          'modernization.settings.sync_drafts.requesting',
          'Setting up: asking your other verified devices for the drafts key. If none of them has it, this device creates it in a few seconds.'
        )}
      </InlineMessage>
    );
  }
  return null;
}

function Drafts() {
  const { t } = useTranslation();
  const [syncDrafts, setSyncDrafts] = useSetting(settingsAtom, 'syncDrafts');

  return (
    <SettingsSection title="Drafts">
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title={t('modernization.settings.sync_drafts.title', 'Sync drafts across devices')}
          description={t(
            'modernization.settings.sync_drafts.description',
            'Unsent messages follow you to your other Synara devices, end-to-end encrypted. It works on every device you have verified.'
          )}
          after={<Switch variant="Primary" value={syncDrafts} onChange={setSyncDrafts} />}
        />
        <DraftSyncKeyHint enabled={syncDrafts} />
      </SequenceCard>
    </SettingsSection>
  );
}

type MessagesProps = {
  requestClose: () => void;
};
/** Messages: how the composer sends and what the timeline shows. */
export function Messages({ requestClose }: MessagesProps) {
  return (
    <SettingsPage
      title="Messages"
      description="How you write messages and what the timeline shows."
      requestClose={requestClose}
    >
      <Composer />
      <Timeline />
      <Drafts />
    </SettingsPage>
  );
}
