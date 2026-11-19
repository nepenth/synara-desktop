import React from 'react';
import { MatrixId } from './MatrixId';
import { Profile } from './Profile';
import { ContactInformation } from './ContactInfo';
import { Password } from './Password';
import { SettingsPage } from '../../../components/settings-layout';

type AccountProps = {
  requestClose: () => void;
};
/** Account: who you are on Matrix. Blocked users live on Privacy. */
export function Account({ requestClose }: AccountProps) {
  return (
    <SettingsPage title="Account" requestClose={requestClose}>
      <Profile />
      <MatrixId />
      <Password />
      <ContactInformation />
    </SettingsPage>
  );
}
