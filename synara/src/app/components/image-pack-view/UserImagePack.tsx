import React, { useCallback, useMemo } from 'react';
import { ImagePackContent } from './ImagePackContent';
import { ImagePack, PackContent } from '../../plugins/custom-emoji';
import { useMatrixClient } from '../../hooks/useMatrixClient';
import { AccountDataEvent } from '../../../types/matrix/accountData';
import { useUserImagePack } from '../../hooks/useImagePacks';
import { setUserImagePackNative } from '../../features/room/nativeImagePack';

export function UserImagePack() {
  const mx = useMatrixClient();

  const defaultPack = useMemo(() => new ImagePack(mx.getUserId() ?? '', {}, undefined), [mx]);
  const imagePack = useUserImagePack();

  const handleUpdate = useCallback(
    async (packContent: PackContent) => {
      // V-SEND.R-PACK-WRITE: native personal-pack write is fail-closed on
      // desktop. The JS mx.setAccountData path is only for non-native web.
      const result = await setUserImagePackNative(packContent);
      if (result === 'legacy') {
        await mx.setAccountData(AccountDataEvent.PoniesUserEmotes as any, packContent as any);
      }
    },
    [mx]
  );

  return <ImagePackContent imagePack={imagePack ?? defaultPack} canEdit onUpdate={handleUpdate} />;
}
