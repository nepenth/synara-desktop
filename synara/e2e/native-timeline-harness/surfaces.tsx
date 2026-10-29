// Message text outside the timeline (inbox, pins, search, room topics), drawn
// by the shipped components over the timeline's renderer. Only the permalink
// handler is a fixture: it records what a click asked to open.
import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { RenderMessageContent } from '../../src/app/components/RenderMessageContent';
import { RoomTopicViewer } from '../../src/app/components/room-topic-viewer/RoomTopicViewer';
import { ClientConfigProvider } from '../../src/app/hooks/useClientConfig';
import { makeHighlightRegex } from '../../src/app/utils/highlightText';
import {
  NativeFormattedBody,
  NativePermalinkProvider,
  type NativePermalinkHandler,
} from '../../src/app/features/room/nativeTimelineFormattedBody';
import { NativePlainMessageBody } from '../../src/app/features/room/nativeLinkUnfurlCard';

const INBOX_HTML = [
  '<p>Hi <a href="https://matrix.to/#/@reader0:example.test">Reader Zero</a>, ',
  'see <a href="https://matrix.to/#/!room:example.test/$event?via=example.test">this message</a> ',
  '<img data-mx-emoticon src="mxc://example.test/party" alt=":party:"> ',
  'and <img src="https://tracker.example/pixel.png" alt="tracker">',
  '<script>window.__surfaceScriptRan = true</script> <strong>bold</strong></p>',
].join('');

const content =
  (value: Record<string, unknown>) =>
  <T,>() =>
    value as T;

function Surfaces() {
  const [opened, setOpened] = useState<string>('');
  const [topicOpen, setTopicOpen] = useState(false);
  const onPermalink: NativePermalinkHandler = (target) => setOpened(JSON.stringify(target));
  const highlightRegex = makeHighlightRegex(['needle']);
  return (
    <ClientConfigProvider value={{}}>
      <NativePermalinkProvider value={onPermalink}>
        <section aria-label="Inbox message">
          <RenderMessageContent
            displayName="Reader One"
            msgType="m.text"
            ts={0}
            getContent={content({
              msgtype: 'm.text',
              body: 'Hi Reader Zero, see this message :party: and tracker bold',
              format: 'org.matrix.custom.html',
              formatted_body: INBOX_HTML,
            })}
          />
        </section>
        <section aria-label="Search result">
          <RenderMessageContent
            displayName="Reader One"
            msgType="m.text"
            ts={0}
            highlightRegex={highlightRegex}
            getContent={content({
              msgtype: 'm.text',
              body: 'Find the needle here',
              format: 'org.matrix.custom.html',
              formatted_body: '<p>Find the <em>needle</em> here, not in <code>needle()</code></p>',
            })}
          />
          <RenderMessageContent
            displayName="Reader One"
            msgType="m.text"
            ts={0}
            highlightRegex={highlightRegex}
            getContent={content({
              msgtype: 'm.text',
              body: 'A plain needle at https://example.org',
            })}
          />
        </section>
        <section aria-label="Pinned notice">
          <RenderMessageContent
            displayName="Reader One"
            msgType="m.notice"
            ts={0}
            getContent={content({
              msgtype: 'm.notice',
              body: 'Ping https://matrix.to/#/@reader1:example.test about https://example.org/docs or ftp://files.example',
            })}
          />
        </section>
        <section aria-label="Emote">
          <RenderMessageContent
            displayName="Reader One"
            msgType="m.emote"
            ts={0}
            getContent={content({ msgtype: 'm.emote', body: 'waves' })}
          />
        </section>
        <section aria-label="Plain message body">
          <NativePlainMessageBody body="Unfurl https://example.org/article please" />
        </section>
        <section aria-label="Formatted message body">
          <NativeFormattedBody
            html={
              '<mx-reply><blockquote>Earlier</blockquote></mx-reply><strong>Read</strong> https://example.org/guide and <a href="https://example.org/linked">this one</a>, not <code>https://example.org/code</code>'
            }
            fallbackBody="Read https://example.org/guide"
          />
        </section>
        <output data-testid="opened-permalink">{opened}</output>
        <button type="button" onClick={() => setTopicOpen(true)}>
          Open topic
        </button>
        {topicOpen && (
          <RoomTopicViewer
            name="Topic room"
            topic="Docs at https://example.org/topic and https://matrix.to/#/#lobby:example.test"
            requestClose={() => setTopicOpen(false)}
          />
        )}
      </NativePermalinkProvider>
    </ClientConfigProvider>
  );
}

export const mountRichTextSurfaces = (root: HTMLElement) => {
  createRoot(root).render(<Surfaces />);
};
