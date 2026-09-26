import { Badge, Box, Icon, IconButton, Icons, Spinner, Text, as, toRem } from 'folds';
import React, { ReactNode, useCallback } from 'react';
import { EncryptedAttachmentInfo } from '../../../types/matrix/common';
import { mimeTypeToExt } from '../../utils/mimeTypes';
import { AsyncStatus, useAsyncCallback } from '../../hooks/useAsyncCallback';
import { saveMatrixMediaFile } from '../../matrix/media';

const badgeStyles = { maxWidth: toRem(100) };

type FileDownloadButtonProps = {
  filename: string;
  url: string;
  mimeType: string;
  encInfo?: EncryptedAttachmentInfo;
};
export function FileDownloadButton({ filename, url }: FileDownloadButtonProps) {
  const [downloadState, download] = useAsyncCallback(
    useCallback(async () => {
      await saveMatrixMediaFile(url, filename);
    }, [url, filename])
  );

  const downloading = downloadState.status === AsyncStatus.Loading;
  const hasError = downloadState.status === AsyncStatus.Error;
  return (
    <IconButton
      disabled={downloading}
      onClick={download}
      variant={hasError ? 'Critical' : 'SurfaceVariant'}
      size="300"
      radii="300"
    >
      {downloading ? (
        <Spinner size="100" variant={hasError ? 'Critical' : 'Secondary'} />
      ) : (
        <Icon size="100" src={Icons.Download} />
      )}
    </IconButton>
  );
}

export type FileHeaderProps = {
  body: string;
  mimeType: string;
  after?: ReactNode;
};
export const FileHeader = as<'div', FileHeaderProps>(({ body, mimeType, after, ...props }, ref) => (
  <Box alignItems="Center" gap="200" grow="Yes" {...props} ref={ref}>
    <Box shrink="No">
      <Badge style={badgeStyles} variant="Secondary" radii="Pill">
        <Text size="O400" truncate>
          {mimeTypeToExt(mimeType)}
        </Text>
      </Badge>
    </Box>
    <Box grow="Yes">
      <Text size="T300" truncate>
        {body}
      </Text>
    </Box>
    {after}
  </Box>
));
