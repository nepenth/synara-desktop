# ROE-11: Media Metadata and Cache Policy

Hypothesis: media identity, metadata, cache eligibility, limits, eviction, and
integrity policy should be shared, while byte transport and platform file
handoff retain their dedicated native channels.

Investigate:

- MXC identity, encryption metadata, thumbnails, MIME claims, dimensions,
  duration, size limits, integrity, and retry state;
- cache keys, quotas, LRU/priority, pinning, lifecycle, logout wipe, and
  corruption recovery;
- desktop and iOS filesystem constraints plus NSE memory/storage constraints;
- privacy leakage through filenames, metadata, previews, and diagnostics;
- which decisions can cross typed DTOs without moving large bytes or paths
  through the generic Core envelope.

Minimum proof: eviction/property tests, corrupt/truncated/adversarial media,
quota and disk-pressure tests, encrypted-media Synapse proof, logout/wipe
proof, desktop/iOS cache integration, and performance/memory budgets.
