# Client reliability remediation and review plan

Status: isolated workstreams are integrated; combined client PR and remaining live proofs are pending.
Read-frontier, edit policy, window/depth, notification context, verification, last-read navigation, live-read cleanup, NSE production-feature CI guard, and CI coverage wiring are on `cursor/client-reliability-integration-d7fa`. Combined client acceptance and Grok review of the combined PR remain pending.
Base: `0695da32c77bc1a56ca22c9ec383f8fd006e6a24` (v2.1.28).
Integration branch: `feature/client-reliability-2026-09-06` (local tip plus remaining CI/NSE commits on `cursor/client-reliability-integration-d7fa`).

## Requested behavior

1. Viewing the newest messages in an active, focused room clears its unread state automatically on both clients.
2. Desktop has an obvious usable window drag area without sacrificing its controls.
3. Room entry restores the user's last-read event when available. If loading cannot resolve it, preserve the current viewport and offer Jump to Last Read. Later background loading must not unexpectedly move the viewport. A focused room at the newest bottom follows new messages; Jump to Latest is visible only away from that bottom; sending a message returns to the newest bottom.
4. iOS alerts notify new messages, not replacement edits to existing messages.
5. Notification taps open the actual room and target event in surrounding history. Encrypted preview, timeline loading/decryption, and verified-session discovery failures require separate causal investigation; a shared cause must be demonstrated.
6. Reduce distracting depth in the iOS room list and add restrained, consistent depth to desktop Settings.

The explicit requested last-read behavior supersedes the prior timeline contract's unavailable-marker fallback to live. Implementing agents must update that contract along with its owner implementation.

## Isolated work and review gates

Implementation agents diagnose, create their own branches/worktrees, implement and validate bounded changes. A fresh review agent checks each completed branch and returns required fixes to its author. After review acceptance, the orchestrator requests a read-only Grok 4.6 High review against the exact branch commit. Accepted branches merge into the integration branch. Integration conflicts and material changes receive renewed review. The final combined PR targets main and receives another Grok 4.6 High review. Do not treat an agent's completion message or model approval as runtime proof.

| Workstream | Implementation | Fresh review | Grok review | Integration |
| --- | --- | --- | --- | --- |
| Visible-tail read acknowledgement | `e56f333a`; test follow-up `13578624` | Accepted | Accepted, including cleanup correction | Merged |
| Last-read navigation and sparse history | `ddda4f3f`; pagination follow-up `7a348762` | Accepted after stream-adoption and stale-response corrections | Accepted after pagination consistency correction | Merged |
| Edit push policy | `32efeb4e` | Accepted | Accepted | Merged; branch CI passed |
| Verification observation and diagnosis | `8f6e90df` | Accepted after authority/cleanup/refresh corrections | Accepted | Merged |
| iOS notification context and preview diagnosis | Production `cb745886`; proof record `76ab6cdb` | Accepted | Accepted | Merged |
| Window drag and visual depth | `9e69a7d8` | Accepted | Accepted after maximize synchronization correction | Merged; branch CI passed |
| NSE production-feature CI guard | `e440c353`; archive/wiring P2s on this PR | Accepted after Apple target coverage correction | Accepted; P2 archive/wiring/fixtures applied | Merged |
| Combined CI coverage wiring | `fe206671` | Coverage-only | Not required | Merged |

Branch review records are in this directory. Grok verdicts are source reviews;
reported limits remain gates for the corresponding runtime claims. An ACCEPT
heading does not waive an actionable finding in its body.

## Operating paths and acceptance

Each path uses an ordinary client user as actor, an authenticated dedicated test session as starting state, and the normal UI action as its first action. Tests may create disposable rooms/messages with the configured test accounts. Personal accounts, production credential changes, key resets and server reconfiguration are outside this diagnostic authorization.

### Room reading and scrolling

Owner route: client room navigation and viewport observation → typed platform adapter → shared Core room/timeline and read owner → Matrix server → room-list projection. Transitions: entering/resolving last-read context → stable anchored history or confirmed live bottom → visible new tail → server acknowledgement → cleared unread state. Side effects are fixture events and read markers on the test room. Completion requires UI position/control state plus authoritative server read-marker/receipt and unread-count readback. Disqualifiers include manual Mark as Read, synthetic receipt writes by the fixture harness, offscreen/background read advancement, lost history, unwanted scroll jumps, or a success label without server evidence.

### Desktop window movement

Owner route: pointer press/drag on visible window chrome → Tauri native drag permission/API → macOS window server. Transition: stationary normal window → same-size window at a changed origin. Side effect is local test-window position. Completion is native frame readback with working nearby controls. CSS presence alone, resize, or dragging from an unrelated OS shortcut is not proof.

### Notifications and room context

Owner route: new or edited Matrix test event → server push rules and gateway → APNs/NSE policy and decryption → iOS notification center → notification tap routing → Core room/context → timeline UI. Side effects are test messages/edits and notifications. Completion requires delivery counts for new-versus-edit events, eligible decrypted preview under declared key/lock conditions, and the real room identity plus target event and surrounding history after tap. Simulator payload injection can prove routing but does not prove APNs delivery or NSE execution. Missing device evidence must remain explicitly unconfirmed.

### Crypto and session eligibility

Owner route: normal Security/verification UI action or room open → platform adapter → Core crypto/session owner → Matrix device/key API and local encrypted store → typed result → UI. Transitions distinguish loading, eligible sessions, no eligible sessions, unverified state, and a concrete service error. Completion requires authoritative returned device/key state and rendered result, plus accessible room history where keys exist. Side effects must stay within test sessions. No key reset, unsafe trust bypass, or replacement credentials may manufacture success.

### Visual depth

Owner route: ordinary room-list/Settings navigation → platform design tokens and surface rendering. Completion requires before/after rendered inspection, legible hierarchy and controls, including relevant accessibility/reduced-motion settings. These are local presentation changes with no remote side effects.

## Validation constraints

Native simulator and large Rust/Xcode builds are serialized by the orchestrator. Reuse existing build caches and preserve prior worktrees. Never commit credentials, raw private event payloads, private server URLs, or encryption material. Every proof records its exact source commit and whether it is Confirmed, Failed, or Not confirmed; unit and mocked tests preserve behavior but do not establish live delivery or physical-device claims.
