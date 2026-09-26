import React, { useEffect, useMemo, useState } from 'react';
import FocusTrap from 'focus-trap-react';
import {
  Box,
  Button,
  Header,
  Icon,
  IconButton,
  Icons,
  Modal,
  Overlay,
  OverlayBackdrop,
  OverlayCenter,
  Scroll,
  Spinner,
  Text,
  config,
} from 'folds';
import { copyToClipboard } from '../../utils/dom';
import { useTimeoutToggle } from '../../hooks/useTimeoutToggle';
import { previewMatrixMediaText } from '../../matrix/media';
import { stopPropagation } from '../../utils/keyboard';
import { NativeFormattedBody } from './nativeTimelineFormattedBody';
import {
  nativeTimelineFileDownloadName,
  saveNativeTimelineFileAttachment,
} from './nativeTimelineFileSave';
import {
  projectNativeTimelineMarkdownPreview,
  type NativeTimelineFilePreviewTarget,
} from './nativeTimelineFilePreview';
import * as htmlCss from './nativeTimelineHtml.css';

export function NativeTimelineMarkdownPreview({
  target,
  onClose,
  onActionError,
}: {
  target: NativeTimelineFilePreviewTarget;
  onClose: () => void;
  onActionError?: (message: string) => void;
}) {
  const label = nativeTimelineFileDownloadName(target.filename);
  const [busySave, setBusySave] = useState(false);
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
          message: error instanceof Error ? error.message : 'Could not open file.',
        });
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [target.handleId, target.mimeType]);

  const projection = useMemo(
    () => (status.kind === 'ready' ? projectNativeTimelineMarkdownPreview(status.text) : undefined),
    [status]
  );

  const download = () => {
    if (busySave) return;
    setBusySave(true);
    void saveNativeTimelineFileAttachment({
      handleId: target.handleId,
      filename: target.filename,
      mimeType: target.mimeType,
    })
      .catch((error) => {
        onActionError?.(error instanceof Error ? error.message : 'Could not download file.');
      })
      .finally(() => setBusySave(false));
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
                    data-native-timeline-file-download="true"
                    onClick={download}
                    aria-label={`Download ${label}`}
                  >
                    <Text size="B300">{busySave ? 'Downloading…' : 'Download'}</Text>
                  </Button>
                </Box>
              </Header>
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
