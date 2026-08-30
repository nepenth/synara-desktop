import React, { useCallback, useEffect, useRef } from 'react';
import { Chip, Icon, IconButton, Icons, Text, color } from 'folds';
import { useAtom } from 'jotai';
import { UploadCard, UploadCardError, CompactUploadCardProgress } from './UploadCard';
import {
  TUploadAtom,
  UploadStatus,
  UploadSuccess,
  useBindUploadAtom,
  makeUploadError,
} from '../../state/upload';
import { useMatrixClient } from '../../hooks/useMatrixClient';
import { TUploadContent } from '../../utils/matrix';
import { bytesToSize, getFileTypeIcon } from '../../utils/common';
import { useMediaConfig } from '../../hooks/useMediaConfig';
import { isSynaraDesktop } from '../../utils/desktop';
import { uploadMediaNative } from '../../state/nativeMediaUpload';
import { effectiveNativeAttachmentLimit } from '../../utils/nativeMediaLimits';

type CompactUploadCardRendererProps = {
  isEncrypted?: boolean;
  uploadAtom: TUploadAtom;
  onRemove: (file: TUploadContent) => void;
  onComplete?: (upload: UploadSuccess) => void;
};
export function CompactUploadCardRenderer({
  isEncrypted,
  uploadAtom,
  onRemove,
  onComplete,
}: CompactUploadCardRendererProps) {
  const mx = useMatrixClient();
  const mediaConfig = useMediaConfig();
  const desktop = isSynaraDesktop();
  const serverAllowSize = mediaConfig['m.upload.size'];
  const allowSize = desktop
    ? effectiveNativeAttachmentLimit(serverAllowSize)
    : serverAllowSize || Infinity;
  const [, setUpload] = useAtom(uploadAtom);

  const { upload, startUpload, cancelUpload } = useBindUploadAtom(mx, uploadAtom, isEncrypted);
  const { file } = upload;
  const fileSizeExceeded = file.size > allowSize;
  const nativeStarted = useRef(false);

  const startNativeUpload = useCallback(async () => {
    // V-SEND.R-PACK-UPLOAD: fail-closed native media upload on desktop.
    // Reuses matrix_upload_media; never falls through to mx.uploadContent.
    const loadingPromise = Promise.resolve({ content_uri: '' } as never);
    setUpload({ promise: loadingPromise });
    try {
      const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
      const mimeType = file.type || 'image/png';
      const uploaded = await uploadMediaNative(mimeType, bytes);
      if (uploaded === 'legacy') {
        throw new Error('Native Matrix media upload is unavailable.');
      }
      setUpload({ mxc: uploaded.mxc });
    } catch (e: unknown) {
      const message =
        e instanceof Error && typeof e.message === 'string'
          ? e.message
          : 'Native Matrix media upload is unavailable.';
      setUpload({ error: makeUploadError(message) });
    }
  }, [file, setUpload]);

  if (upload.status === UploadStatus.Idle && !fileSizeExceeded) {
    if (desktop) {
      if (!nativeStarted.current) {
        nativeStarted.current = true;
        void startNativeUpload();
      }
    } else {
      startUpload();
    }
  }

  const removeUpload = () => {
    if (!desktop) {
      cancelUpload();
    }
    nativeStarted.current = false;
    onRemove(file);
  };

  useEffect(() => {
    if (upload.status === UploadStatus.Success) {
      onComplete?.(upload);
    }
  }, [upload, onComplete]);

  return (
    <UploadCard
      compact
      outlined
      radii="300"
      before={<Icon src={getFileTypeIcon(Icons, file.type)} />}
      after={
        <>
          {upload.status === UploadStatus.Error && (
            <Chip
              as="button"
              onClick={() => {
                if (desktop) {
                  nativeStarted.current = false;
                  void startNativeUpload();
                } else {
                  startUpload();
                }
              }}
              aria-label="Retry Upload"
              variant="Critical"
              radii="Pill"
              outlined
            >
              <Text size="B300">Retry</Text>
            </Chip>
          )}
          <IconButton
            onClick={removeUpload}
            aria-label="Cancel Upload"
            variant="SurfaceVariant"
            radii="Pill"
            size="300"
          >
            <Icon src={Icons.Cross} size="200" />
          </IconButton>
        </>
      }
    >
      {upload.status === UploadStatus.Success ? (
        <>
          <Text size="H6" truncate>
            {file.name}
          </Text>
          <Icon style={{ color: color.Success.Main }} src={Icons.Check} size="100" />
        </>
      ) : (
        <>
          {upload.status === UploadStatus.Idle && !fileSizeExceeded && (
            <CompactUploadCardProgress sentBytes={0} totalBytes={file.size} />
          )}
          {upload.status === UploadStatus.Loading && (
            <CompactUploadCardProgress sentBytes={upload.progress.loaded} totalBytes={file.size} />
          )}
          {upload.status === UploadStatus.Error && (
            <UploadCardError>
              <Text size="T200">{upload.error.message}</Text>
            </UploadCardError>
          )}
          {upload.status === UploadStatus.Idle && fileSizeExceeded && (
            <UploadCardError>
              <Text size="T200">
                The file size exceeds the limit. Maximum allowed size is{' '}
                <b>{bytesToSize(allowSize)}</b>, but the uploaded file is{' '}
                <b>{bytesToSize(file.size)}</b>.
              </Text>
            </UploadCardError>
          )}
        </>
      )}
    </UploadCard>
  );
}
