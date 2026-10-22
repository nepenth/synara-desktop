import React, { useState } from 'react';
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
  Text,
} from 'folds';
import { Modal } from '../../components/modal-dialog';
import { stopPropagation } from '../../utils/keyboard';
import {
  nativeTimelineActionErrorMessage,
  TIMELINE_ACTION_FAILED,
} from './nativeTimelineErrorCopy';
import { saveNativeTimelineFileAttachment } from './nativeTimelineFileSave';
import { nativeTimelineMediaSrc } from './nativeTimelineView';
import { NativeTimelineNotice } from './NativeTimelineNotice';
import * as htmlCss from './nativeTimelineHtml.css';

/** An image or video opened from the timeline, shown at its original size. */
export type NativeTimelineMediaViewerTarget = {
  kind: 'image' | 'video';
  handleId: string;
  /** What the media shows: its caption or filename. */
  label: string;
  filename?: string;
  mimeType?: string;
  /** The inline media that opened the viewer; focus returns to it. */
  opener?: HTMLElement;
};

/** The opener, or its remounted copy when virtualization replaced the row meanwhile. */
const viewerReturnFocus = (target: NativeTimelineMediaViewerTarget): HTMLElement | undefined => {
  if (target.opener?.isConnected) return target.opener;
  return (
    document.querySelector<HTMLElement>(
      `[data-native-timeline-media-open="${CSS.escape(target.handleId)}"]`
    ) ?? undefined
  );
};

/**
 * The timeline's image and video viewer: the original through the row's Core
 * media handle (never a thumbnail), with Save. Escape, the back button or a
 * click outside closes it, and focus returns to the media that opened it.
 */
export function NativeTimelineMediaViewer({
  target,
  onClose,
}: {
  target: NativeTimelineMediaViewerTarget;
  onClose: () => void;
}) {
  const src = nativeTimelineMediaSrc({ handleId: target.handleId, mimeType: target.mimeType });
  const [failed, setFailed] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string>();

  const save = () => {
    if (saving) return;
    setSaving(true);
    setSaveError(undefined);
    void saveNativeTimelineFileAttachment({
      handleId: target.handleId,
      filename: target.filename,
      mimeType: target.mimeType,
    })
      .catch((error) => {
        setSaveError(nativeTimelineActionErrorMessage(error, TIMELINE_ACTION_FAILED.download));
      })
      .finally(() => setSaving(false));
  };

  return (
    <Overlay open backdrop={<OverlayBackdrop />}>
      <OverlayCenter>
        <FocusTrap
          focusTrapOptions={{
            initialFocus: false,
            // WebKit does not focus a clicked button, so name the opener.
            setReturnFocus: (previous) => viewerReturnFocus(target) ?? previous,
            onDeactivate: onClose,
            clickOutsideDeactivates: true,
            escapeDeactivates: stopPropagation,
          }}
        >
          <Modal
            className={htmlCss.MediaViewerModal}
            variant="Surface"
            size="500"
            aria-label={`View ${target.kind}: ${target.label}`}
            data-native-timeline-media-viewer={target.kind}
          >
            <Header className={htmlCss.FilePreviewHeader} variant="Surface" size="400">
              <Box grow="Yes" alignItems="Center" gap="200">
                <IconButton size="300" radii="300" onClick={onClose} aria-label="Close viewer">
                  <Icon size="50" src={Icons.ArrowLeft} />
                </IconButton>
                <Text size="T300" truncate>
                  {target.label}
                </Text>
              </Box>
              <Box shrink="No" alignItems="Center" gap="200">
                <Button
                  variant="Primary"
                  fill="Soft"
                  size="300"
                  radii="300"
                  disabled={saving}
                  data-native-timeline-media-save="true"
                  onClick={save}
                  aria-label={`Save ${target.label}`}
                >
                  <Text size="B300">{saving ? 'Saving…' : 'Save'}</Text>
                </Button>
              </Box>
            </Header>
            {saveError ? (
              <NativeTimelineNotice
                tone="error"
                message={saveError}
                onDismiss={() => setSaveError(undefined)}
              />
            ) : null}
            <div className={htmlCss.MediaViewerBody}>
              {failed || !src ? (
                <Text size="T300" data-native-timeline-media-viewer-failed="true">
                  This {target.kind} could not be shown here. Save it to open it in another app.
                </Text>
              ) : target.kind === 'image' ? (
                <img
                  className={htmlCss.MediaViewerMedia}
                  src={src}
                  alt={target.label}
                  onError={() => setFailed(true)}
                />
              ) : (
                // Matrix media metadata does not provide a captions track.
                // eslint-disable-next-line jsx-a11y/media-has-caption
                <video
                  className={htmlCss.MediaViewerMedia}
                  src={src}
                  controls
                  aria-label={target.label}
                  onError={() => setFailed(true)}
                />
              )}
            </div>
          </Modal>
        </FocusTrap>
      </OverlayCenter>
    </Overlay>
  );
}
