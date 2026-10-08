import React, { useCallback, useMemo } from 'react';
import { ImagePackContent } from './ImagePackContent';
import { ImagePack, PackContent } from '../../plugins/custom-emoji';
import { useUserImagePack } from '../../hooks/useImagePacks';
import { setUserImagePackNative } from '../../features/room/nativeImagePack';
import { useMyUserId } from '../../state/nativeIdentity';

export function UserImagePack() {
  const myUserId = useMyUserId();

  const defaultPack = useMemo(() => new ImagePack(myUserId, {}, undefined), [myUserId]);
  const imagePack = useUserImagePack();

  const handleUpdate = useCallback(async (packContent: PackContent) => {
    // V-SEND.R-PACK-WRITE: the native personal-pack write is fail-closed.
    // Outside a native session there is no other write path.
    await setUserImagePackNative(packContent);
  }, []);

  return <ImagePackContent imagePack={imagePack ?? defaultPack} canEdit onUpdate={handleUpdate} />;
}
