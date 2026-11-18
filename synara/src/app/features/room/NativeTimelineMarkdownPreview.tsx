import React, { useEffect, useState } from 'react';
import FocusTrap from '../../components/FocusTrap';
import {
  Box,
  Button,
  Header,
  Icon,
  IconButton,
  Icons,
  Overlay,
  OverlayBackdrop,
  OverlayCenter,
  Scroll,
  Spinner,
  Text,
  config,
} from 'folds';
import { Modal } from '../../components/modal-dialog';
import { copyToClipboard } from '../../utils/dom';
import { useTimeoutToggle } from '../../hooks/useTimeoutToggle';
import { previewMatrixMediaText } from '../../matrix/media';
import { stopPropagation } from '../../utils/keyboard';
import { NativeFormattedBody } from './nativeTimelineFormattedBody';
import { nativeTimelineActionErrorMessage } from './nativeTimelineErrorCopy';
import { nativeTimelineFileDownloadName } from './nativeTimelineFileSave';
import {
  dismissNativeTimelineFileSave,
  nativeTimelineFileSaveButtonLabel,
  nativeTimelineFileSavedMessage,
  startNativeTimelineFileSave,
  useNativeTimelineFileSave,
  useNativeTimelineFileSaveProgressVisible,
} from './nativeTimelineFileSaveState';
import { revealNativeTimelineSavedFile } from './NativeTimelineFileSaveStatus';
import { NativeTimelineNotice } from './NativeTimelineNotice';
import {
  renderNativeTimelineMarkdownPreview,
  type NativeTimelineFilePreviewTarget,
} from './nativeTimelineFilePreview';
import * as htmlCss from './nativeTimelineHtml.css';

export function NativeTimelineMarkdownPreview({
  target,
  onClose,
}: {
  target: NativeTimelineFilePreviewTarget;
  onClose: () => void;
}) {
  const label = nativeTimelineFileDownloadName(target.filename);
  // The same save as the file in the timeline: one download, one result.
  const saveState = useNativeTimelineFileSave(target.handleId);
  const busySave = saveState?.kind === 'downloading' || saveState?.kind === 'finishing';
  const progressVisible = useNativeTimelineFileSaveProgressVisible(saveState);
  const [revealError, setRevealError] = useState<string>();
  const [copied, setCopied] = useTimeoutToggle();
  const [status, setStatus] = useState<
    | { kind: 'loading' }
    | { kind: 'error'; message: string }
    | { kind: 'tooLarge' }
    | { kind: 'ready'; text: string }
  >({ kind: 'loading' });

  useEffect(() => {
    let cancelled = false;
    setStatus({ kind: 'loading' });
    void (async () => {
      try {
        const preview = await previewMatrixMediaText(target.handleId);
        if (cancelled) return;
        if (preview.kind === 'tooLarge') {
          setStatus({ kind: 'tooLarge' });
          return;
        }
        setStatus({ kind: 'ready', text: preview.text });
      } catch (error) {
        if (cancelled) return;
        setStatus({
          kind: 'error',
          message: nativeTimelineActionErrorMessage(
            error,
            'Synara could not open this file. Try again.'
          ),
        });
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [target.handleId, target.mimeType]);

  const [projection, setProjection] =
    useState<Awaited<ReturnType<typeof renderNativeTimelineMarkdownPreview>>>();
  useEffect(() => {
    setProjection(undefined);
    if (status.kind !== 'ready') return undefined;
    let disposed = false;
    void renderNativeTimelineMarkdownPreview(status.text).then((next) => {
      if (!disposed) setProjection(next);
    });
    return () => {
      disposed = true;
    };
  }, [status]);

  const download = () => {
    setRevealError(undefined);
    void startNativeTimelineFileSave({
      handleId: target.handleId,
      filename: target.filename,
      mimeType: target.mimeType,
    });
  };

  return (
    <Overlay open backdrop={<OverlayBackdrop />}>
      <OverlayCenter>
        <FocusTrap
          focusTrapOptions={{
            initialFocus: false,
            onDeactivate: onClose,
            clickOutsideDeactivates: true,
            escapeDeactivates: stopPropagation,
          }}
        >
          <Modal
            className={htmlCss.FilePreviewModal}
            variant="Surface"
            size="500"
            aria-label={`Preview ${label}`}
            data-native-timeline-file-preview="true"
          >
            <Box direction="Column" style={{ maxHeight: '88vh' }}>
              <Header className={htmlCss.FilePreviewHeader} variant="Surface" size="400">
                <Box grow="Yes" alignItems="Center" gap="200">
                  <IconButton size="300" radii="300" onClick={onClose} aria-label="Close preview">
                    <Icon size="50" src={Icons.ArrowLeft} />
                  </IconButton>
                  <Text size="T300" truncate>
                    {label}
                  </Text>
                </Box>
                <Box shrink="No" alignItems="Center" gap="200">
                  <Button
                    variant="Secondary"
                    fill="Soft"
                    size="300"
                    radii="300"
                    disabled={status.kind !== 'ready'}
                    data-native-timeline-file-copy="true"
                    onClick={() => {
                      if (status.kind !== 'ready') return;
                      copyToClipboard(status.text);
                      setCopied();
                    }}
                    aria-label="Copy markdown"
                  >
                    <Text size="B300">{copied ? 'Copied' : 'Copy'}</Text>
                  </Button>
                  <Button
                    variant="Primary"
                    fill="Soft"
                    size="300"
                    radii="300"
                    disabled={busySave}
                    aria-busy={busySave ? true : undefined}
                    data-native-timeline-file-download="true"
                    onClick={download}
                    aria-label={`Download ${label}`}
                  >
                    <Text size="B300">
                      {nativeTimelineFileSaveButtonLabel(
                        saveState,
                        busySave && progressVisible,
                        'Download'
                      )}
                    </Text>
                  </Button>
                </Box>
              </Header>
              {saveState?.kind === 'done' ? (
                <NativeTimelineNotice
                  tone={revealError ? 'error' : 'info'}
                  message={revealError ?? nativeTimelineFileSavedMessage(saveState.filename)}
                  action={{
                    label: 'Show in folder',
                    onClick: () =>
                      revealNativeTimelineSavedFile(saveState.filename, setRevealError),
                  }}
                  onDismiss={() => dismissNativeTimelineFileSave(target.handleId)}
                />
              ) : null}
              {saveState?.kind === 'failed' ? (
                <NativeTimelineNotice
                  tone="error"
                  message={saveState.message}
                  action={saveState.retryable ? { label: 'Retry', onClick: download } : undefined}
                  onDismiss={() => dismissNativeTimelineFileSave(target.handleId)}
                />
              ) : null}
              <Scroll hideTrack variant="Background" visibility="Hover">
                <div className={htmlCss.FilePreviewBody}>
                  {status.kind === 'loading' ? (
                    <Box alignItems="Center" gap="200" style={{ padding: config.space.S400 }}>
                      <Spinner size="200" variant="Secondary" aria-label="Loading preview" />
                      <Text size="T300">Opening {label}…</Text>
                    </Box>
                  ) : null}
                  {status.kind === 'error' ? <Text size="T300">{status.message}</Text> : null}
                  {status.kind === 'tooLarge' || projection?.tooLarge ? (
                    <Text size="T300">
                      This markdown file is too large to preview. Download it to read the full
                      document.
                    </Text>
                  ) : null}
                  {status.kind === 'ready' && projection && !projection.tooLarge ? (
                    <NativeFormattedBody html={projection.html} fallbackBody={projection.plain} />
                  ) : null}
                </div>
              </Scroll>
            </Box>
          </Modal>
        </FocusTrap>
      </OverlayCenter>
    </Overlay>
  );
}
