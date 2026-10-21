import React, { useEffect, useMemo, useState } from 'react';
import { Box, Icon, IconButton, Icons, Text } from 'folds';
import { isDesktopPlatform, openPlatformExternalUrl } from '../../platform';
import {
  convertDesktopFileSrc,
  invokeDesktopWithAvailability,
  isSynaraDesktop,
} from '../../utils/desktop';
import type { RoomEncryptionStatus } from '../matrix-dto/room';
import * as htmlCss from './nativeTimelineHtml.css';
import {
  fetchMediaPreviewWithNativeOwner,
  firstTimelinePreviewUrl,
  shouldSkipMessageUnfurl,
  type NativeMediaPreview,
} from './nativeLinkUnfurl';
import { NativeLinkifiedText } from './nativeTimelineFormattedBody';

const handleDesktopExternalLinkClick: React.MouseEventHandler<HTMLAnchorElement> = (evt) => {
  if (
    evt.defaultPrevented ||
    evt.button !== 0 ||
    evt.metaKey ||
    evt.ctrlKey ||
    evt.shiftKey ||
    evt.altKey ||
    !isDesktopPlatform()
  ) {
    return;
  }
  const href = evt.currentTarget.href;
  if (!href) return;
  evt.preventDefault();
  void openPlatformExternalUrl(href);
};

/** A message without HTML: its text and links, as the formatted renderer draws links. */
export function NativePlainMessageBody({
  body,
  style,
}: {
  body: string;
  style?: React.CSSProperties;
}) {
  return (
    <Text size="T300" style={style}>
      <NativeLinkifiedText text={body} />
    </Text>
  );
}

export function NativeLinkUnfurlCard({
  preview,
  onDismiss,
}: {
  preview: NativeMediaPreview;
  onDismiss?: () => void;
}) {
  const thumbnailSrc = preview.thumbnailHandleId
    ? convertDesktopFileSrc(preview.thumbnailHandleId, 'synara-media')
    : undefined;
  const hostname = (() => {
    try {
      return new URL(preview.url).hostname;
    } catch {
      return preview.siteName ?? preview.url;
    }
  })();

  return (
    <div className={htmlCss.UnfurlCard}>
      <a
        className={htmlCss.UnfurlLink}
        href={preview.url}
        target="_blank"
        rel="noreferrer noopener"
        onClick={handleDesktopExternalLinkClick}
      >
        {thumbnailSrc ? <img className={htmlCss.UnfurlThumb} src={thumbnailSrc} alt="" /> : null}
        <Box direction="Column" gap="100" grow="Yes" style={{ minWidth: 0, padding: '8px 10px' }}>
          <Text size="T200" truncate>
            {preview.siteName ?? hostname}
          </Text>
          {preview.title ? (
            <Text size="T300" style={{ fontWeight: 600 }}>
              {preview.title}
            </Text>
          ) : null}
          {preview.description ? (
            <Text size="T200" style={{ opacity: 0.8 }}>
              {preview.description}
            </Text>
          ) : null}
        </Box>
      </a>
      {onDismiss ? (
        <IconButton
          onClick={(event) => {
            event.preventDefault();
            event.stopPropagation();
            onDismiss();
          }}
          variant="SurfaceVariant"
          size="300"
          radii="300"
          aria-label="Dismiss link preview"
        >
          <Icon src={Icons.Cross} size="50" />
        </IconButton>
      ) : null}
    </div>
  );
}

export function NativeTimelineLinkUnfurl({
  roomId,
  sessionGeneration,
  encryptionStatus,
  body,
  formattedBody,
  messageType,
  hasMedia,
  skip,
  originServerTs,
}: {
  roomId: string;
  sessionGeneration: number;
  encryptionStatus?: RoomEncryptionStatus;
  body: string;
  formattedBody?: string;
  messageType?: string;
  hasMedia?: boolean;
  skip?: boolean;
  originServerTs?: number;
}) {
  const url = useMemo(() => {
    if (shouldSkipMessageUnfurl({ messageType, hasMedia, skip })) return undefined;
    return firstTimelinePreviewUrl(body, formattedBody);
  }, [body, formattedBody, hasMedia, messageType, skip]);
  const [preview, setPreview] = useState<NativeMediaPreview | null>(null);

  useEffect(() => {
    if (!url || encryptionStatus !== 'not_encrypted') {
      setPreview(null);
      return undefined;
    }
    let cancelled = false;
    void fetchMediaPreviewWithNativeOwner({
      roomId,
      url,
      ts: originServerTs,
      encryptionStatus,
      desktopAvailable: isSynaraDesktop(),
      invoke: invokeDesktopWithAvailability,
    }).then((next) => {
      if (!cancelled) setPreview(next);
    });
    return () => {
      cancelled = true;
    };
  }, [encryptionStatus, originServerTs, roomId, sessionGeneration, url]);

  if (!preview || preview.url !== url) return null;
  return <NativeLinkUnfurlCard preview={preview} />;
}
