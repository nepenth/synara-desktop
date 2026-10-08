import React from 'react';
import { Text, Chip } from 'folds';
import { SequenceCard } from '../../../components/sequence-card';
import { SequenceCardStyle, SettingsQuietControl } from '../styles.css';
import { SettingTile } from '../../../components/setting-tile';
import { copyToClipboard } from '../../../utils/dom';
import { getSafeMyUserId } from '../../../state/nativeIdentity';
import { SettingsSection } from '../../../components/settings-layout';

export function MatrixId() {
  const userId = getSafeMyUserId();

  return (
    <SettingsSection title="Matrix ID">
      <SequenceCard
        className={SequenceCardStyle}
        variant="SurfaceVariant"
        direction="Column"
        gap="400"
      >
        <SettingTile
          title={userId}
          after={
            <Chip
              className={SettingsQuietControl}
              variant="Secondary"
              fill="None"
              radii="Pill"
              onClick={() => copyToClipboard(userId)}
            >
              <Text size="T200">Copy</Text>
            </Chip>
          }
        />
      </SequenceCard>
    </SettingsSection>
  );
}
