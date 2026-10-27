import React, { useEffect, useState } from 'react';
import { Box, Text } from 'folds';
import { isDesktopPlatform, openPlatformExternalUrl } from '../../platform';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import type { InlineGifLink } from '../../utils/gifLinks';
import { fetchXPost, xStatusId, type XPost, type XPostMedia } from '../../utils/xEmbed';
import * as htmlCss from './nativeTimelineHtml.css';

const openExternal: React.MouseEventHandler<HTMLAnchorElement> = (evt) => {
  if (evt.defaultPrevented || evt.button !== 0 || !isDesktopPlatform()) return;
  const { href } = evt.currentTarget;
  if (!href) return;
  evt.preventDefault();
  void openPlatformExternalUrl(href);
};

/**
 * A Klipy or GIPHY link played in place, loaded straight from the provider
 * as its terms require. If it fails to load, the link shows instead.
 */
export function NativeInlineGif({ gif }: { gif: InlineGifLink }) {
  const [failed, setFailed] = useState(false);
  if (failed) {
    return (
      <a href={gif.url} target="_blank" rel="noreferrer noopener" onClick={openExternal}>
        {gif.url}
      </a>
    );
  }
  return (
    <img
      className={htmlCss.InlineGif}
      src={gif.url}
      alt="GIF"
      loading="lazy"
      referrerPolicy="no-referrer"
      onError={() => setFailed(true)}
      data-testid="inline-gif"
    />
  );
}

const mediaAspect = (media: XPostMedia): string | undefined =>
  media.width && media.height ? `${media.width} / ${media.height}` : undefined;

function XPostMediaView({ media }: { media: XPostMedia }) {
  if (media.kind === 'photo') {
    return (
      <img
        className={htmlCss.XPostMedia}
        style={{ aspectRatio: mediaAspect(media) }}
        src={media.url}
        alt=""
        loading="lazy"
        referrerPolicy="no-referrer"
      />
    );
  }
  // X's "GIFs" are short MP4s: they loop silently like a GIF would.
  const gif = media.kind === 'gif';
  return (
    // eslint-disable-next-line jsx-a11y/media-has-caption -- X provides no caption track.
    <video
      className={htmlCss.XPostMedia}
      style={{ aspectRatio: mediaAspect(media) }}
      src={media.src}
      poster={media.poster}
      controls={!gif}
      autoPlay={gif}
      loop={gif}
      muted={gif}
      playsInline
      preload="metadata"
      data-testid="x-post-video"
    />
  );
}

export function NativeXPostCard({ post }: { post: XPost }) {
  const media = post.media.slice(0, post.media[0]?.kind === 'photo' ? 4 : 1);
  return (
    <div className={htmlCss.XPostCard} data-testid="x-post-card">
      <Box alignItems="Center" gap="200">
        {post.avatarUrl ? (
          <img
            className={htmlCss.XPostAvatar}
            src={post.avatarUrl}
            alt=""
            referrerPolicy="no-referrer"
          />
        ) : null}
        <Box direction="Column" style={{ minWidth: 0 }}>
          <Text size="T300" truncate style={{ fontWeight: 600 }}>
            {post.authorName}
          </Text>
          <Text size="T200" truncate priority="300">
            @{post.authorHandle}
          </Text>
        </Box>
      </Box>
      {post.text ? (
        <Text size="T300" style={{ whiteSpace: 'pre-wrap', overflowWrap: 'anywhere' }}>
          {post.text}
        </Text>
      ) : null}
      {media.length > 0 ? (
        <div className={media.length > 1 ? htmlCss.XPostMediaGrid : undefined}>
          {media.map((item) => (
            <XPostMediaView key={item.kind === 'photo' ? item.url : item.src} media={item} />
          ))}
        </div>
      ) : null}
      <a
        className={htmlCss.XPostSource}
        href={post.url}
        target="_blank"
        rel="noreferrer noopener"
        onClick={openExternal}
      >
        View on X
      </a>
    </div>
  );
}

/** The X card for an x.com or twitter.com status link; nothing while loading or if X has no post. */
export function NativeXPostEmbed({ url }: { url: string }) {
  const [post, setPost] = useState<XPost>();

  useEffect(() => {
    setPost(undefined);
    if (!xStatusId(url) || !isSynaraDesktop()) return undefined;
    let cancelled = false;
    void fetchXPost(url, invokeDesktopWithAvailability).then((next) => {
      if (!cancelled) setPost(next);
    });
    return () => {
      cancelled = true;
    };
  }, [url]);

  return post ? <NativeXPostCard post={post} /> : null;
}
