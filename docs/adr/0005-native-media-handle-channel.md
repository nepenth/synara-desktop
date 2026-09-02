# ADR 0005: Native Media Handle and Byte Channels

Originally accepted: 2026-08-18 via PR #1001.

Last reviewed: 2026-09-01.

Status: accepted and implemented. This status describes the media boundary,
not completion of every historical shared-Core phase or release gate.

## Context

Timeline events need to identify, authorize, decrypt, download, and display
media without placing raw sources, local paths, or large byte arrays in the
generic Core JSON command envelope. That envelope is capped at 1 MiB and is
appropriate for bounded control/data models, not bulk transport.

Encrypted `mxc://` sources also carry authority and key material that should
not be reconstructed independently by presenters.

## Decision

Core/native media owners expose opaque media handles plus bounded metadata.
The handle identifies a native-owned source without exposing a filesystem path
or requiring the presenter to understand Matrix encryption metadata.

- Timeline rows may expose `media_handle_id` and bounded MIME, size, dimension,
  or duration metadata.
- iOS resolves timeline bytes through the dedicated typed
  `SharedCore.timeline_media_bytes(handle_id)` UniFFI byte method.
- Desktop resolves timeline handles through the `synara-media://` native
  protocol and the narrow `matrix_media_download` shell adapter.
- Send/upload workflows use their dedicated native queues and byte/file
  handoffs rather than `Core::command` JSON.
- Cache, integrity, quota, and retry policy may live in Core when shared;
  filesystem paths, OS file coordination, and display remain platform-owned.

The exact byte caps of dedicated channels are product and platform policy and
may differ by operation. They must be explicit, tested, and fail closed; they
do not inherit permission to use the generic envelope.

## Must not

- Put media/attachment bytes or local filesystem paths in `Core::command`.
- Put raw `mxc://` media sources or encryption material on timeline presentation
  DTOs when an opaque handle is required.
- Let React or SwiftUI independently decrypt timeline media.
- Treat legacy/plain download helpers as an alternate iOS live media engine.
- Download media from the notification service extension.
- Log handles, filenames, source URLs, keys, or byte payloads without the
  redaction and diagnostic policy appropriate to that boundary.

## Current evidence

- `TimelineViewRowDto.media_handle_id` is defined in
  `crates/synara-core/src/shared_core_ffi.rs` and the UniFFI schema.
- `timeline_media_bytes` is the dedicated iOS byte path.
- `src-tauri/src/matrix/media/product_commands.rs` resolves native timeline
  handles for desktop with explicit URI and byte bounds.
- `synara/src/app/matrix/media.ts` recognizes the `synara-media://` protocol.
- Core attachment and media modules define separate bounded send/download
  policies rather than serializing bytes into the generic envelope.

## Rationale

- Opaque handles preserve one source/decryption authority.
- Dedicated byte APIs avoid JSON/base64 expansion and the generic envelope's
  memory and serialization limits.
- Platform paths never become portable identifiers or leak into shared models.
- Native adapters can enforce OS storage, lifecycle, memory, and cleanup
  constraints while Core retains shared policy.

## Consequences

- Media metadata can evolve through versioned typed fields without exposing
  bytes or paths.
- Dedicated native channels require their own size limits, cancellation,
  backpressure, corruption handling, cache policy, and tests.
- Plain/avatar/pack legacy paths may be retired incrementally, but they must not
  become a second encrypted timeline-media implementation.
- Any proposal to place media bytes on the generic envelope requires an
  explicit replacement ADR and must overcome the security and performance
  reasons above.

## Related decisions

- [ADR 0003 — shared native Rust core](0003-shared-native-rust-core.md)
- [ADR 0004 — Rust and platform ownership boundaries](0004-rust-language-boundaries.md)
