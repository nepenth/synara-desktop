import React from 'react';
import { MatrixId } from './MatrixId';
import { Profile } from './Profile';
import { ContactInformation } from './ContactInfo';
import { IgnoredUserList } from './IgnoredUserList';
import { SettingsPage } from '../../../components/settings-layout';

type AccountProps = {
  requestClose: () => void;
};
export function Account({ requestClose }: AccountProps) {
  return (
    <SettingsPage
      title="Account"
      description="Your profile, Matrix ID, contact details and ignored users."
      requestClose={requestClose}
    >
      <Profile />
      <MatrixId />
      <ContactInformation />
      <IgnoredUserList />
    </SettingsPage>
  );
}
