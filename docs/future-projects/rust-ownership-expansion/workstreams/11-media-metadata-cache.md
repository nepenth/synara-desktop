# ROE-11: Media Metadata and Cache Policy

Prior: **metadata only, subordinate to ADR 0005**.

ADR 0005 already establishes opaque media handles and dedicated native byte
channels. This workstream cannot put local paths or media/attachment bytes on
the generic Core command envelope, or replace required timeline handles with
raw `mxc://` presentation sources. A bounded Matrix content URI may still be a
legitimate control-plane identifier on a deliberately typed operation; it is
not a byte channel or a presenter decryption path.

## Bounded research question

Is shared authority missing for MXC identity, encryption metadata, thumbnails,
MIME claims, dimensions, duration, size/integrity limits, cache eligibility,
quota/eviction, retry state, corruption recovery, logout wipe, or diagnostics?
Separate those decisions from desktop/iOS filesystem paths, file handoff, and
NSE storage/lifecycle constraints.

Any proposal must use typed metadata or existing handles and preserve the
accepted channel. Evidence should cover corrupt/truncated/adversarial media,
quota and disk pressure, encrypted Synapse media, logout/wipe, privacy-safe
diagnostics, and platform performance/memory budgets.

Paths and bytes on the generic envelope remain prohibited even if a metadata
policy gap is found.
