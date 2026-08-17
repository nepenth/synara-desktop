import { ReactNode, useCallback, useEffect } from 'react';
import { IThumbnailContent } from '../../../../types/matrix/common';
import { useMatrixClient } from '../../../hooks/useMatrixClient';
import { AsyncStatus, useAsyncCallback } from '../../../hooks/useAsyncCallback';
import { useMediaAuthentication } from '../../../hooks/useMediaAuthentication';
import { FALLBACK_MIMETYPE } from '../../../utils/mimeTypes';
import { createMatrixMediaObjectUrl } from '../../../matrix/media';

export type ThumbnailContentProps = {
  info: IThumbnailContent;
  renderImage: (src: string) => ReactNode;
};
export function ThumbnailContent({ info, renderImage }: ThumbnailContentProps) {
  const mx = useMatrixClient();
  const useAuthentication = useMediaAuthentication();

  const [thumbSrcState, loadThumbSrc] = useAsyncCallback(
    useCallback(async () => {
      const thumbInfo = info.thumbnail_info;
      const thumbMxcUrl = info.thumbnail_file?.url ?? info.thumbnail_url;
      const encInfo = info.thumbnail_file;
      if (typeof thumbMxcUrl !== 'string' || typeof thumbInfo?.mimetype !== 'string') {
        throw new Error('Failed to load thumbnail');
      }

      return createMatrixMediaObjectUrl(mx, thumbMxcUrl, {
        useAuthentication,
        mimeType: thumbInfo.mimetype ?? FALLBACK_MIMETYPE,
        encryptedInfo: encInfo,
      });
    }, [mx, info, useAuthentication])
  );

  useEffect(() => {
    loadThumbSrc();
  }, [loadThumbSrc]);

  return thumbSrcState.status === AsyncStatus.Success ? renderImage(thumbSrcState.data) : null;
}
