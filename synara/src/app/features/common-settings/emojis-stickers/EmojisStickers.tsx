import React, { useState } from 'react';
import { ImagePack } from '../../../plugins/custom-emoji';
import { ImagePackView } from '../../../components/image-pack-view';
import { RoomPacks } from './RoomPacks';
import { SettingsPage } from '../../../components/settings-layout';

type EmojisStickersProps = {
  requestClose: () => void;
};
export function EmojisStickers({ requestClose }: EmojisStickersProps) {
  const [imagePack, setImagePack] = useState<ImagePack>();

  const handleImagePackViewClose = () => {
    setImagePack(undefined);
  };

  if (imagePack) {
    return <ImagePackView address={imagePack.address} requestClose={handleImagePackViewClose} />;
  }

  return (
    <SettingsPage
      title="Custom Emoji"
      description="Emoji and sticker packs available in this room."
      requestClose={requestClose}
    >
      <RoomPacks onViewPack={setImagePack} />
    </SettingsPage>
  );
}
