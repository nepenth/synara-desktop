# Synara Media And External URL Policy

Reviewed: 2026-05-25

Status: initial shared contract with runtime URL safety in
`src/app/utils/remoteContent.ts` and schema fixtures under `docs/contracts/`.

## Purpose

Synara renders Matrix media, GIF/provider metadata, agent artifacts, and
external links across desktop and future iOS surfaces. Platform code must not
fetch or open local/private network targets through untrusted payloads.

## Machine-Readable Artifacts

- [synara-safe-remote-url.schema.json](./contracts/synara-safe-remote-url.schema.json)
- [synara-safe-remote-url.json fixtures](./contracts/fixtures/synara-safe-remote-url.json)

## Safe Remote URL Rules

Safe remote URLs must:

- Use `https:`.
- Omit username/password credentials.
- Target a public host, not localhost.
- Avoid private, loopback, link-local, carrier-grade NAT, or unspecified IPv4.
- Avoid loopback, unique-local, link-local, unspecified, or IPv4-mapped IPv6.
- Avoid local host suffixes such as `.localhost`, `.local`, `.localdomain`,
  `.internal`, `.lan`, and `.home.arpa`.

Unsafe URLs must fail closed. Platform code should render a disabled or
unavailable state rather than silently opening a rejected URL.

## Matrix Media

- Matrix media should prefer `mxc://` identifiers and SDK/media-repository APIs
  rather than long-lived authenticated HTTP URLs.
- Authenticated media URLs must not be stored in Synara account data,
  notifications, agent action payloads, or durable settings.
- iOS media downloads should use SDK-provided authenticated media handling
  where available.

## iOS Notes

- Swift URL validation should mirror the same allow-list and local/private
  network rejection rules before opening external links or downloading agent
  artifacts.
- Rich notification extensions must not fetch arbitrary remote media without a
  separate threat model and explicit App Store/privacy review.

## Acceptance Criteria

- Fixtures cover public HTTPS acceptance and non-HTTPS, credentialed, localhost,
  private IPv4, private IPv6, and local suffix rejection.
- Runtime tests call `safeRemoteContentUrl` for the same fixtures.
