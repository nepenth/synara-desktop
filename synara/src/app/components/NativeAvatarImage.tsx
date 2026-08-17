import { AvatarImage } from 'folds';
import React, { ComponentProps } from 'react';
import { useNativeMatrixMediaSrc } from '../hooks/useNativeMatrixMediaSrc';

export function NativeAvatarImage({ src, ...props }: ComponentProps<typeof AvatarImage>) {
  const resolved = useNativeMatrixMediaSrc(src);
  if (!resolved) return null;
  return <AvatarImage src={resolved} {...props} />;
}
