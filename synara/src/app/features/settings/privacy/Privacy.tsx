import React from 'react';
import { Switch } from 'folds';
import { SequenceCard } from '../../../components/sequence-card';
import { useSetting } from '../../../state/hooks/settings';
import { settingsAtom } from '../../../state/settings';
import { SettingTile } from '../../../components/setting-tile';
import { SequenceCardStyle } from '../styles.css';
import { IgnoredUserList } from '../account/IgnoredUserList';
import { SettingsPage, SettingsSection } from '../../../components/settings-layout';

function Activity() {
  const [hideActivity, setHideActivity] = useSetting(settingsAtom, 'hideActivity');

  return (
    <SettingsSection title="Activity">
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Hide typing & read receipts"
          description="Don't send typing status, and hide who is typing and who has read a room. Reading still marks rooms as read privately, so badges clear and your other devices stay in sync."
          after={<Switch variant="Primary" value={hideActivity} onChange={setHideActivity} />}
        />
      </SequenceCard>
    </SettingsSection>
  );
}

type PrivacyProps = {
  requestClose: () => void;
};
/** Privacy: what others see of your activity, and who can reach you. */
export function Privacy({ requestClose }: PrivacyProps) {
  return (
    <SettingsPage
      title="Privacy"
      description="What other people see of your activity, and who can reach you."
      requestClose={requestClose}
    >
      <Activity />
      <IgnoredUserList />
    </SettingsPage>
  );
}
