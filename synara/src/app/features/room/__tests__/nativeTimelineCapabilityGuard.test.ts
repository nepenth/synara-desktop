import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

/**
 * Presenter capability-boundary guard (follow-on thinning slice).
 *
 * Both timeline presenters grew ~700-800 lines in the ownership follow-ons.
 * They must stay capability-gated renderers: action availability comes only
 * from Core `TimelineViewRow` capabilities, forward transport only from the
 * Core `text|media` route, media only from opaque handles. No policy inference
 * from event text, MIME types, paths, media bytes, or raw MXC URIs.
 *
 * `nativeTimelinePresentationProjection.ts` remains the single shared
 * presentation-projection boundary for formatted bodies.
 */

const presenter = readFileSync(
  `${process.cwd()}/src/app/features/room/NativeTimelinePresenter.tsx`,
  'utf8'
);
const viewContract = readFileSync(
  `${process.cwd()}/src/app/features/room/nativeTimelineView.ts`,
  'utf8'
);
const projection = readFileSync(
  `${process.cwd()}/src/app/features/room/nativeTimelinePresentationProjection.ts`,
  'utf8'
);

test('desktop presenter renders only from Core capabilities', () => {
  // Capability reads exist across row kinds and surfaces.
  assert.match(presenter, /capabilities\??\./);
  assert.match(presenter, /capabilities\??\.react/);
  assert.match(presenter, /capabilities\??\.vote/);
  assert.match(presenter, /capabilities\??\.declineCall/);
  assert.match(presenter, /capabilities\??\.forward/);
  // Closed forward-transport check, never inferred from media handles.
  assert.match(presenter, /isNativeTimelineForwardTransport/);
  // The presenter never touches raw MXC URIs; media flows via opaque handles.
  assert.doesNotMatch(presenter, /mxc:\/\//);
});

test('desktop presenter consumes the shared projection boundary', () => {
  // The presenter renders formatted bodies only through NativeFormattedBody,
  // which projects through the single shared projection module.
  assert.match(presenter, /NativeFormattedBody/);
  const formattedBody = readFileSync(
    `${process.cwd()}/src/app/features/room/nativeTimelineFormattedBody.tsx`,
    'utf8'
  );
  assert.match(formattedBody, /projectNativeFormattedBody/);
  assert.match(formattedBody, /nativeTimelinePresentationProjection/);
  assert.match(projection, /projectNativeFormattedBody/);
  // The view contract keeps the prohibition explicit at the source.
  assert.match(viewContract, /Presenters must not infer/);
  // Capabilities are read back from Core opens, never inferred locally.
  assert.match(viewContract, /never inferred/);
});

test('desktop view contract keeps capability and transport vocabularies closed', () => {
  assert.match(viewContract, /NativeTimelineRowCapabilities/);
  assert.match(viewContract, /NativeTimelineForwardTransport/);
  assert.match(viewContract, /declineCall/);
  // No local policy tables mapping MIME types or message text to actions.
  assert.doesNotMatch(viewContract, /capabilit\w*\s*=\s*[^;]*mimeType/i);
  assert.doesNotMatch(viewContract, /capabilit\w*\s*=\s*[^;]*body\.includes/i);
});
