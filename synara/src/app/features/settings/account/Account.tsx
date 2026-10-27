import React from 'react';
import { MatrixId } from './MatrixId';
import { Profile } from './Profile';
import { ContactInformation } from './ContactInfo';
import { Password } from './Password';
import { IgnoredUserList } from './IgnoredUserList';
import { SettingsPage } from '../../../components/settings-layout';

type AccountProps = {
  requestClose: () => void;
};
export function Account({ requestClose }: AccountProps) {
  return (
    <SettingsPage
      title="Account"
      description="Your profile, Matrix ID, password, contact details and ignored users."
      requestClose={requestClose}
    >
      <Profile />
      <MatrixId />
      <Password />
      <ContactInformation />
      <IgnoredUserList />
    </SettingsPage>
  );
}
