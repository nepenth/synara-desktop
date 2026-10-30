import React, { useEffect, useMemo, useState } from 'react';
import { Box, Icon, IconButton, Icons, Text } from 'folds';
import { isDesktopPlatform, openPlatformExternalUrl } from '../../platform';
import {
  convertDesktopFileSrc,
  invokeDesktopWithAvailability,
  isSynaraDesktop,
} from '../../utils/desktop';
import { useSetting } from '../../state/hooks/settings';
import { settingsAtom } from '../../state/settings';
import * as htmlCss from './nativeTimelineHtml.css';
import {
  fetchDeviceLinkPreview,
  linkPreviewImagePath,
  timelinePreviewUrl,
  type NativeLinkPreview,
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
  preview: NativeLinkPreview;
  onDismiss?: () => void;
}) {
  const [imageFailed, setImageFailed] = useState(false);
  const thumbnailSrc =
    preview.imageHandleId && !imageFailed
      ? convertDesktopFileSrc(linkPreviewImagePath(preview.imageHandleId), 'synara-media')
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
        {thumbnailSrc ? (
          <img
            className={htmlCss.UnfurlThumb}
            src={thumbnailSrc}
            alt=""
            onError={() => setImageFailed(true)}
          />
        ) : null}
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

/**
 * The preview card for a message's first link, fetched from this device when
 * Settings > General > Link previews is on, in encrypted rooms too. Nothing
 * is fetched for media messages or when `skip` says another card (an X post,
 * an inline GIF, an approval or agent card) already shows the link.
 */
export function NativeTimelineLinkUnfurl({
  body,
  formattedBody,
  messageType,
  hasMedia,
  skip,
}: {
  body: string;
  formattedBody?: string;
  messageType?: string;
  hasMedia?: boolean;
  skip?: boolean;
}) {
  const [linkPreviews] = useSetting(settingsAtom, 'linkPreviews');
  const url = useMemo(
    () =>
      timelinePreviewUrl({
        enabled: linkPreviews,
        body,
        formattedBody,
        messageType,
        hasMedia,
        skip,
      }),
    [body, formattedBody, hasMedia, linkPreviews, messageType, skip]
  );
  const [preview, setPreview] = useState<NativeLinkPreview | null>(null);

  useEffect(() => {
    if (!url) {
      setPreview(null);
      return undefined;
    }
    let cancelled = false;
    void fetchDeviceLinkPreview({
      url,
      enabled: linkPreviews,
      desktopAvailable: isSynaraDesktop(),
      invoke: invokeDesktopWithAvailability,
    }).then((next) => {
      if (!cancelled) setPreview(next);
    });
    return () => {
      cancelled = true;
    };
  }, [linkPreviews, url]);

  if (!url || !preview || preview.url !== url) return null;
  return <NativeLinkUnfurlCard preview={preview} />;
}
