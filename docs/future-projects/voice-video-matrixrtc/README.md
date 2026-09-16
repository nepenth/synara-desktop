# Voice and Video Calls (MatrixRTC) — Research Memo

Status: exploratory research memo, written 2026-09-14. Nothing in this document
authorizes product changes. It answers the question: _what would it take for
Synara (desktop macOS/Linux and iOS) to offer voice and video calls between
users, and later with AI agents, using the current Matrix ecosystem?_

## 1. How calling works in Matrix today

There are two generations of Matrix calling.

| Generation      | Spec                                                                                                                                                                                                                                                                                                                            | Media path                                                                                                                                                                                                                     | Who uses it                                                  | Status                                                                                    |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------ | ----------------------------------------------------------------------------------------- |
| Legacy 1:1 VoIP | `m.call.invite` / `m.call.answer` / `m.call.candidates` (client-server spec, v1.x)                                                                                                                                                                                                                                              | Peer-to-peer WebRTC, optional TURN from the homeserver                                                                                                                                                                         | Element Web/Desktop (1:1 only), older clients                | Frozen; not supported by Element X; `matrix-rust-sdk` has no calling state machine for it |
| MatrixRTC       | [MSC4143](https://github.com/matrix-org/matrix-spec-proposals/pull/4143) (RTC sessions), [MSC4195](https://github.com/matrix-org/matrix-spec-proposals/pull/4195) (LiveKit transport), [MSC4075](https://github.com/matrix-org/matrix-spec-proposals/pull/4075) (ringing notifications), MSC4519 (transport discovery endpoint) | Matrix carries **signalling only** (`org.matrix.msc3401.call.member` state events per participant, plus encrypted to-device key exchange for media E2EE); media flows through a **LiveKit SFU** deployed beside the homeserver | Element Call, Element Web/Desktop, Element X, Cinny (recent) | The only path with active investment; this is what "Matrix 2.0" calls means               |

Key properties of MatrixRTC that answer the room-type question directly:

- **No special room type is required.** A call is an _RTC session_ attached to
  any room. Participants publish a `org.matrix.msc3401.call.member` state event
  (state key = their user/device) that names the session and the transport
  (focus) they are using. Anyone reading room state can see who is in the
  call. Leaving is clearing that state event. Element uses ordinary rooms and
  DMs; "video rooms" in Element are just rooms with a room-type hint
  (`org.matrix.msc3417.call`) so the UI opens straight into the call, not a
  server-side requirement.
- **Encryption is per-sender media keys** distributed over Matrix encrypted
  to-device messages (`io.element.call.encryption_keys`), so E2EE rooms give
  E2EE calls. Unencrypted rooms give SFU-terminated (transport-encrypted only)
  calls.
- **Scale**: because the SFU fans out media, group calls scale well past what
  peer-to-peer mesh allows; a 1:1 call is simply a two-participant session.

## 2. Server side (Synapse, latest)

Synapse itself does not carry media. A MatrixRTC deployment needs three
pieces next to the homeserver:

1. **LiveKit SFU** (`livekit/livekit-server`) — WebRTC media server.
2. **MatrixRTC Authorization Service** (`element-hq/lk-jwt-service`) — exchanges
   a Matrix OpenID token for a LiveKit JWT so only authenticated Matrix users
   (optionally only from allow-listed homeservers) can join rooms on the SFU.
   Requires Synapse to expose a `federation` or `openid` listener because the
   service calls `/_matrix/federation/v1/openid/userinfo` to validate tokens.
3. **Discovery**: clients find the transport through, in order of preference,
   the authenticated endpoint `GET /_matrix/client/v1/rtc/transports`
   (MSC4519; Synapse serves it when `msc4143_enabled: true` and
   `matrix_rtc.transports` is configured) and the legacy
   `.well-known/matrix/client` key `org.matrix.msc4143.rtc_foci`. Serve
   well-known as `application/json` with permissive CORS; missing CORS is the
   most common self-hosting failure (Element X reports
   `MISSING_MATRIX_RTC_TRANSPORT`).

Minimal `homeserver.yaml` fragment (Synapse ≥ 1.13x; MSC3266 room summary is
stabilised in recent releases and no longer needs a flag):

```yaml
experimental_features:
  msc4222_enabled: true # sync v2 state_after, needed for correct call state tracking
  msc4143_enabled: true # advertises MatrixRTC + serves the transports endpoint
matrix_rtc:
  transports:
    - type: livekit
      livekit_service_url: https://rtc.example.org/livekit/jwt
```

Deployment guides: Element Call `docs/self_hosting.md`, and Element Server
Suite's "Matrix RTC" configuration reference. Element's hosted LiveKit is no
longer a courtesy backend for third-party homeservers (since April 2025), so a
self-hosted Synapse needs its own SFU + JWT service for calls to work.

## 3. Client side: how Element does it and what `matrix-rust-sdk` offers

### Element Web / Element Desktop

Element Web embeds **Element Call** (a React app built on `matrix-js-sdk`'s
MatrixRTC session module + the LiveKit client SDK) as a **widget** in an
iframe. The host client owns the Matrix account, encryption, and the widget
`postMessage` API; the widget owns WebRTC, device pickers, layout and call UI.

### Element X (iOS/Android, built on `matrix-rust-sdk`)

Element X does not implement call UI natively. It:

1. Creates a Rust `WidgetDriver` for the room using
   `matrix_sdk::widget::{WidgetSettings, WidgetDriver}` with the
   `experimental-widgets` feature.
2. Builds the Element Call URL with
   `WidgetSettings::new_virtual_element_call_widget(props, config)` +
   `settings.generate_webview_url(room, client_props)` (there are typed
   `VirtualElementCallWidgetProperties` / `VirtualElementCallWidgetConfig` with
   `intent`, `skip_lobby`, `header`, `controlled_audio_devices`, etc.).
3. Loads that URL in a platform **WebView** and bridges `postMessage` traffic
   both ways through `WidgetDriverHandle::{recv, send}`.
4. The driver implements capability negotiation, room state/timeline reads,
   event sends, and encrypted to-device delivery on the widget's behalf — this
   is how Element Call discovers participants, publishes membership, and
   exchanges media keys without ever holding the user's Matrix keys.
5. Ringing/incoming-call UX uses MSC4075 `m.rtc.notification` events and push.

### What is available in our pinned SDK (`matrix-sdk = 0.19.0`)

Verified against the published crate source:

- `matrix_sdk::widget` module exists behind the **`experimental-widgets`**
  cargo feature (we do not enable it today): `WidgetSettings`,
  `WidgetDriver`, `WidgetDriverHandle`, `Capabilities`/`CapabilitiesProvider`,
  Element Call virtual-widget helpers (`widget/settings/element_call.rs`),
  to-device capability support for `io.element.call.encryption_keys`.
- `Client::rtc_transports()` queries the authenticated MSC4519
  `GET /_matrix/client/v1/rtc/transports` API (with cache helpers).
  `Client::rtc_foci()` remains as a deprecated well-known fallback.
- There is **no** native (non-widget) MatrixRTC session state machine in the
  Rust SDK and no LiveKit client in Rust that Element ships. Element's native
  clients all delegate WebRTC to the Element Call web app.

## 4. Options for Synara

| Option                                                                | What it is                                                                                                                                                                                                                                                                                                                                                                                      | Effort                                                                                                                                                                                                                                                                                                                                          | Fit with our architecture                                                                                                                            |
| --------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| **A. Embed Element Call via the SDK widget driver** (Element X model) | Enable `experimental-widgets`; Core exposes a typed "call widget" owner (create driver for room, generate URL, pump messages); desktop hosts the URL in a second Tauri webview or an iframe inside the app webview; iOS hosts it in `WKWebView`. Element Call is loaded from a configurable URL (self-host `element-call` static build alongside the homeserver, or `https://call.element.io`). | Medium. Most of the hard parts (signalling, E2EE keys, roster) come from Element Call + the Rust driver. Work is: driver plumbing in Core, a `postMessage` bridge in each shell, incoming-call detection from `m.rtc.notification` / member state, room header call buttons, lobby/permissions (mic/camera prompts in WebKitGTK and WKWebView). | Good: one shared Core, two thin shells, matches ADR 0004 (platform APIs own the webview). Calls work in **any** room, encrypted or not, DM or group. |
| B. Native call UI with a LiveKit client per platform                  | Use LiveKit's Swift SDK on iOS and the LiveKit JS client inside our React app on desktop, with Core publishing `call.member` state and handling media-key to-device exchange ourselves.                                                                                                                                                                                                         | High. We would re-implement MatrixRTC session semantics and E2EE key rotation that Element Call already encodes; two media stacks to maintain.                                                                                                                                                                                                  | Poor for now; revisit only if the widget approach is limiting.                                                                                       |
| C. Legacy `m.call.*` 1:1 VoIP                                         | Peer-to-peer WebRTC via the app webview.                                                                                                                                                                                                                                                                                                                                                        | Medium, but dead-end.                                                                                                                                                                                                                                                                                                                           | No group calls, no Element X interop, no SDK support. Reject.                                                                                        |

**Recommendation: Option A.** It is what every other Rust-SDK client does, and
it is the only path that interoperates with Element X / Element Call users on
the same homeserver.

### Desktop-specific notes (Tauri 2)

- WebKitGTK (Linux) and WKWebView (macOS) both support WebRTC; camera and
  microphone permission prompts must be handled by the shell
  (`WebViewBuilder` permission handler on macOS; WebKitGTK permission request
  signal on Linux). This is the main platform-integration cost.
- Hosting Element Call in a **separate Tauri window/webview** (not an iframe in
  the main app webview) avoids iframe permission delegation problems and lets a
  call persist while the user navigates rooms. Element X uses a dedicated
  screen for the same reason.
- `parent_url` for the driver should be the call webview's own URL when there
  is no iframe parent (documented in `VirtualElementCallWidgetProperties`).

### iOS notes

- `WKWebView` + `WKUIDelegate` media capture permission; CallKit/PushKit
  integration for ringing is optional and can come later. Element X ships
  background audio via the web view's audio session.

## 5. Agents in calls (including locally hosted agents)

MatrixRTC makes an agent a first-class call participant with **no client
changes** beyond what Option A delivers:

- The agent is a Matrix user in the room. It watches for
  `org.matrix.msc3401.call.member` state, exchanges its OpenID token for a
  LiveKit JWT at the same `lk-jwt-service`, joins the LiveKit room, publishes
  its own member state (so it appears in the roster), and, in encrypted
  rooms, participates in the per-sender key exchange over encrypted to-device
  messages.
- The **LiveKit Agents framework** (Python/Node) provides the realtime audio
  pipeline: VAD, turn detection, STT → LLM → TTS or a speech-to-speech model,
  interruption handling. It is provider-agnostic: STT/LLM/TTS can be
  OpenAI-compatible **localhost** endpoints (whisper.cpp / faster-whisper,
  llama.cpp / Ollama / vLLM, Piper / Kokoro), which satisfies the
  "locally hosted agent" goal.
- A working reference implementation already exists: the open-source
  **MindRoom** project's `matrix_rtc` module joins Element Call rooms as an
  agent, supports a fully local cascaded STT/LLM/TTS configuration, and
  documents the exact MatrixRTC join/keys flow (`docs/voice-calls.md`,
  PR "Add provider-independent MatrixRTC voice calls"). Hermes (the agent
  framework Synara already integrates for approvals) could adopt the same
  pattern, or a small sidecar could bridge Hermes to a LiveKit agent session.
- Practical rule from that implementation: allow at most one calls-enabled
  agent per room, and have the agent only join calls whose founding member
  advertises the trusted local focus, to avoid accidentally streaming to a
  remote SFU.

Because the agent is "just another participant", the same room, the same
call button, and the same encryption apply whether the other party is a human
or an agent. Ad-hoc private DM rooms with one agent are the natural UX for a
voice conversation.

## 6. Proposed phased plan (for a later ADR / implementation plan)

1. **Infra spike (no product code)**: deploy LiveKit + `lk-jwt-service` beside
   the development Synapse; enable `msc4143_enabled`/`msc4222_enabled` and
   `matrix_rtc.transports`; verify Element Web/X can call each other on that
   homeserver. Record the exact config in `integration/synapse/` as an
   optional profile.
2. **Core**: enable `experimental-widgets`; add a typed call-widget owner
   (`matrix_call_widget_open/close`, message pump events) that follows the
   existing owner pattern; add `rtc_transports()` (bump SDK or typed request);
   surface "call in progress" from room state and `m.rtc.notification`
   ringing into the notification contract.
3. **Desktop**: dedicated call window hosting Element Call, `postMessage`
   bridge, WebKitGTK/WKWebView media permission handling, room header
   voice/video buttons, in-call indicator on the room list.
4. **iOS**: `WKWebView` call screen with the same driver pump; ringing via
   the existing APNs path.
5. **Agents**: document the agent join contract (member state, JWT exchange,
   key exchange) and provide a reference LiveKit-Agents sidecar config for a
   fully local STT/LLM/TTS stack; validate 1:1 voice with a local model.
6. Later: CallKit, screen share policy, moderation (room admins can end
   sessions via state), telemetry-free diagnostics.

Open questions for the decision: which Element Call URL to ship by default
(self-hosted build strongly preferred for a privacy-first client), whether
to require E2EE rooms for calls, and how to present agents versus humans in
the roster.

## References

- MSC4143 MatrixRTC, MSC4195 LiveKit transport, MSC4075 call notifications,
  MSC4519 transport discovery endpoint.
- Element blog: "Exploring MatrixRTC: Real time communication in rooms";
  "Element X: Ignition" (Element Call embedding).
- Element Call `docs/self_hosting.md`; `element-hq/lk-jwt-service`.
- `matrix-rust-sdk` `matrix_sdk::widget` docs (`WidgetDriver`,
  `VirtualElementCallWidgetProperties`, `VirtualElementCallWidgetConfig`);
  PR #6789 "Support for RTC transports discovery".
- LiveKit Agents framework documentation; MindRoom `docs/voice-calls.md`.
