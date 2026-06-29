import React from 'react';
import { useNavigate } from 'react-router-dom';
import { JoinAddressPrompt } from '../../../components/join-address-prompt';
import { getHomePath, getHomeRoomPathWithViaServers } from '../../pathUtils';

export function HomeJoin() {
  const navigate = useNavigate();

  return (
    <JoinAddressPrompt
      onCancel={() => navigate(getHomePath())}
      onOpen={(roomIdOrAlias, viaServers, eventId) => {
        navigate(getHomeRoomPathWithViaServers(roomIdOrAlias, eventId, viaServers));
      }}
    />
  );
}
