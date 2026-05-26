# Matrix SDK Probe

Status: IOS-0006 local package probe.

This package verifies that Synara can resolve and import the official
`matrix-org/matrix-rust-components-swift` Swift package without creating the
real iOS app project yet.

Run from this directory:

```sh
swift package resolve
swift build
swift run MatrixSDKProbe
```

The probe does not log in, does not persist credentials, and does not contact a
homeserver. Real login/session work belongs in the native iOS app after the
architecture ADR is accepted.
