# Retired SDK experiment

Retired: 2026-09-30.

This directory is a provenance record. Its executable sources, package
manifests, and lockfiles have been removed now that the product uses the
project-owned Rust Core and Matrix Rust SDK 0.19.1. It is not an active build,
test, dependency-update, or supported Matrix client path.

The original experiment can be inspected at repository commit `0a3d2ce95622e7ae4326dd8a3be45cb5240a3ea2`:

```sh
git show 0a3d2ce95622e7ae4326dd8a3be45cb5240a3ea2:synara-ios/spikes/matrix-sdk-probe/README.md
git ls-tree -r 0a3d2ce95622e7ae4326dd8a3be45cb5240a3ea2 -- synara-ios/spikes/matrix-sdk-probe
```

Historical measurements and capability reports retain their original version,
source paths, and conclusions; they do not establish current runtime behavior.
Current architecture is [ADR 0003](../../../docs/adr/0003-shared-native-rust-core.md)
and [ADR 0004](../../../docs/adr/0004-rust-language-boundaries.md).

---

## Original experiment record

The following README is retained as historical evidence. Its commands refer
to files at the commit above and cannot be run from this retired directory.

# Matrix SDK Probe

Status: IOS-0006 local package probe plus gated live E2EE probe.

This package verifies that Synara can resolve and import the official
`matrix-org/matrix-rust-components-swift` Swift package, pinned to release
`26.06.06`. Its default mode does
not contact a homeserver. Its gated live mode validates disposable Matrix login
and encrypted-room behavior before those SDK calls are moved into the app
service layer.

Run from this directory:

```sh
swift package resolve
swift build
swift run MatrixSDKProbe
```

Run the live encrypted-room probe only with disposable credentials supplied by
environment variables:

```sh
SYNARA_MATRIX_PROBE=live-e2ee \
SYNARA_E2EE_HOMESERVER=<test homeserver> \
SYNARA_E2EE_USERNAME=<test username> \
SYNARA_E2EE_PASSWORD=<test password> \
SYNARA_E2EE_ROOM=<encrypted room id, alias, or display name> \
SYNARA_E2EE_SEND=1 \
swift run MatrixSDKProbe
```

The live mode prints non-sensitive status only. It must not be wired into CI
with real credentials, and no homeserver password, access token, or refresh
token should be committed to the repository.
