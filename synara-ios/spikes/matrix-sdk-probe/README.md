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
