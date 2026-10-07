import { AvatarImage } from 'folds';
import React, { ComponentProps } from 'react';
import { useNativeMatrixMediaSrc } from '../hooks/useNativeMatrixMediaSrc';
import { NATIVE_AVATAR_THUMBNAIL_PX } from '../matrix/media';

export function NativeAvatarImage({ src, ...props }: ComponentProps<typeof AvatarImage>) {
  const resolved = useNativeMatrixMediaSrc(src, { size: NATIVE_AVATAR_THUMBNAIL_PX });
  if (!resolved) return null;
  return <AvatarImage src={resolved} {...props} />;
}
