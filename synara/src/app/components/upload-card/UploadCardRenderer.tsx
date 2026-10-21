import React, { ReactNode, useEffect } from 'react';
import { Box, Chip, Icon, IconButton, Icons, Spinner, Text, color, config, toRem } from 'folds';
import { UploadCard, UploadCardError, UploadCardProgress } from './UploadCard';
import {
  UploadStatus,
  UploadSuccess,
  isNativeStagedUpload,
  makeUploadError,
  useBindUploadAtom,
} from '../../state/upload';
import { TUploadContent } from '../../utils/matrix';
import { bytesToSize, getFileTypeIcon } from '../../utils/common';
import {
  roomUploadAtomFamily,
  TUploadItem,
  TUploadMetadata,
} from '../../state/room/roomInputDrafts';
import { useObjectURL } from '../../hooks/useObjectURL';
import { useMediaConfig } from '../../hooks/useMediaConfig';
import {
  attachmentTooLargeMessage,
  effectiveNativeAttachmentLimit,
  mediaConfigUploadLimit,
} from '../../utils/nativeMediaLimits';
import { composerErrorMessage } from '../../features/room/composerSendError';

type PreviewImageProps = {
  fileItem: TUploadItem;
};
function PreviewImage({ fileItem }: PreviewImageProps) {
  const { originalFile, metadata } = fileItem;
  const fileUrl = useObjectURL(originalFile);

  return (
    <img
      style={{
        objectFit: 'contain',
        width: '100%',
        height: toRem(152),
        filter: metadata.markedAsSpoiler ? 'blur(44px)' : undefined,
      }}
      alt={originalFile.name}
      src={fileUrl}
    />
  );
}

type PreviewVideoProps = {
  fileItem: TUploadItem;
};
function PreviewVideo({ fileItem }: PreviewVideoProps) {
  const { originalFile, metadata } = fileItem;
  const fileUrl = useObjectURL(originalFile);

  return (
    // eslint-disable-next-line jsx-a11y/media-has-caption
    <video
      style={{
        objectFit: 'contain',
        width: '100%',
        height: toRem(152),
        filter: metadata.markedAsSpoiler ? 'blur(44px)' : undefined,
      }}
      src={fileUrl}
    />
  );
}

type MediaPreviewProps = {
  fileItem: TUploadItem;
  onSpoiler: (marked: boolean) => void;
  children: ReactNode;
};
function MediaPreview({ fileItem, onSpoiler, children }: MediaPreviewProps) {
  const { originalFile, metadata } = fileItem;
  const fileUrl = useObjectURL(originalFile);

  return fileUrl ? (
    <Box
      style={{
        borderRadius: config.radii.R300,
        overflow: 'hidden',
        backgroundColor: 'black',
        position: 'relative',
      }}
    >
      {children}
      <Box
        justifyContent="End"
        style={{
          position: 'absolute',
          bottom: config.space.S100,
          left: config.space.S100,
          right: config.space.S100,
        }}
      >
        <Chip
          variant={metadata.markedAsSpoiler ? 'Warning' : 'Secondary'}
          fill="Soft"
          radii="Pill"
          aria-pressed={metadata.markedAsSpoiler}
          before={<Icon src={Icons.EyeBlind} size="50" />}
          onClick={() => onSpoiler(!metadata.markedAsSpoiler)}
        >
          <Text size="B300">Spoiler</Text>
        </Chip>
      </Box>
    </Box>
  ) : null;
}

/**
 * Where a natively sent composer file is in its send: queued behind another
 * file, or being uploaded by Core now (which can not be cancelled).
 */
export type NativeUploadSendState = 'waiting' | 'uploading';

type UploadCardRendererProps = {
  /** Unused: Core encrypts attachments in encrypted rooms. */
  isEncrypted?: boolean;
  /** Core can send attachments; until then the card waits. */
  nativeComposerSend?: boolean;
  /** Set while a composer send owns this file. */
  sendState?: NativeUploadSendState;
  fileItem: TUploadItem;
  setMetadata: (fileItem: TUploadItem, metadata: TUploadMetadata) => void;
  onRemove: (file: TUploadContent) => void;
  onComplete?: (upload: UploadSuccess) => void;
};
export function UploadCardRenderer({
  nativeComposerSend = false,
  sendState,
  fileItem,
  setMetadata,
  onRemove,
  onComplete,
}: UploadCardRendererProps) {
  const mediaConfig = useMediaConfig();
  const serverAllowSize = mediaConfigUploadLimit(mediaConfig);
  const allowSize = nativeComposerSend
    ? effectiveNativeAttachmentLimit(serverAllowSize)
    : serverAllowSize || Infinity;

  const uploadAtom = roomUploadAtomFamily(fileItem.file);
  const { metadata } = fileItem;
  const { upload, cancelUpload, markNativeStaged, setUpload } = useBindUploadAtom(uploadAtom);
  const { file } = upload;
  const fileSizeExceeded = file.size > allowSize;
  const nativeStaged = isNativeStagedUpload(upload);

  useEffect(() => {
    // An oversized native file can never be sent. Mark it failed so the board
    // says so instead of waiting at 0%, and a send names the file. A staged
    // file fails too when the server limit arrives after it was added.
    if (nativeComposerSend && fileSizeExceeded && sendState === undefined) {
      if (upload.status === UploadStatus.Idle || nativeStaged) {
        setUpload({
          error: makeUploadError(attachmentTooLargeMessage(file.name, file.size, allowSize)),
        });
      }
      return;
    }
    // Until Core confirms it can send attachments (on mount and after a room
    // switch) the card waits. It never uploads from the renderer: that path
    // always failed and showed a Retry that failed again (D85).
    if (upload.status !== UploadStatus.Idle || fileSizeExceeded || !nativeComposerSend) return;
    markNativeStaged();
  }, [
    allowSize,
    file,
    fileSizeExceeded,
    markNativeStaged,
    nativeComposerSend,
    nativeStaged,
    sendState,
    setUpload,
    upload.status,
  ]);

  const handleSpoiler = (marked: boolean) => {
    setMetadata(fileItem, { ...metadata, markedAsSpoiler: marked });
  };

  const removeUpload = () => {
    cancelUpload();
    onRemove(file);
  };

  useEffect(() => {
    if (upload.status === UploadStatus.Success) {
      onComplete?.(upload);
    }
  }, [upload, onComplete]);

  return (
    <UploadCard
      radii="300"
      before={<Icon src={getFileTypeIcon(Icons, file.type)} />}
      after={
        <>
          {sendState === 'uploading' ? (
            <Box alignItems="Center" gap="100" role="status">
              <Spinner variant="Secondary" size="100" />
              <Text size="T200" priority="300">
                Sending…
              </Text>
            </Box>
          ) : (
            <IconButton
              onClick={removeUpload}
              aria-label={sendState === 'waiting' ? 'Do not send this file' : 'Cancel Upload'}
              variant="SurfaceVariant"
              radii="Pill"
              size="300"
            >
              <Icon src={Icons.Cross} size="200" />
            </IconButton>
          )}
        </>
      }
      bottom={
        <>
          {fileItem.originalFile.type.startsWith('image') && (
            <MediaPreview fileItem={fileItem} onSpoiler={handleSpoiler}>
              <PreviewImage fileItem={fileItem} />
            </MediaPreview>
          )}
          {fileItem.originalFile.type.startsWith('video') && (
            <MediaPreview fileItem={fileItem} onSpoiler={handleSpoiler}>
              <PreviewVideo fileItem={fileItem} />
            </MediaPreview>
          )}
          {nativeStaged && (
            // Core uploads the file when it is sent and reports no progress,
            // so the card states that instead of showing a finished upload.
            <Text size="T200" priority="300">
              {sendState === 'uploading'
                ? `Uploading ${bytesToSize(file.size)}…`
                : sendState === 'waiting'
                  ? `Waiting to send · ${bytesToSize(file.size)}`
                  : `Uploads when you send · ${bytesToSize(file.size)}`}
            </Text>
          )}
          {upload.status === UploadStatus.Loading && (
            <UploadCardProgress sentBytes={upload.progress.loaded} totalBytes={file.size} />
          )}
          {upload.status === UploadStatus.Error && (
            <UploadCardError>
              <Text size="T200">
                {composerErrorMessage(
                  upload.error,
                  'Synara could not upload this file. Try again.'
                )}
              </Text>
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
      }
    >
      <Text size="H6" truncate>
        {file.name}
      </Text>
      {upload.status === UploadStatus.Success && !nativeStaged && (
        <Icon style={{ color: color.Success.Main }} src={Icons.Check} size="100" />
      )}
    </UploadCard>
  );
}
