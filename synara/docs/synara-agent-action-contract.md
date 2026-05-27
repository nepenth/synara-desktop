# Synara Agent Action Contract

Reviewed: 2026-05-25

Status: initial shared contract with runtime normalization in
`src/app/agents/agentActions.ts` and desktop shell validation in
`src-tauri/src/desktop.rs`. The canonical writer schema and fixtures live under
`docs/contracts/`.

## Purpose

Agent actions are structured controls rendered from explicit agent-card payloads
such as `in.synara.agent`, `org.hermes.agent`, and configured deployment keys.
They must be bounded, typed, and safe to expose on macOS, Linux, and future iOS
surfaces. They are not local command execution.

## Payload Model

Machine-readable artifacts:

- [synara-agent-action.schema.json](./contracts/synara-agent-action.schema.json)
- [synara-agent-action.json fixtures](./contracts/fixtures/synara-agent-action.json)

The JSON Schema defines the canonical bounded writer payload. Runtime readers
still apply the URL safety and normalization behavior documented here.

```ts
type AgentActionPayload = {
  id: string;
  title: string;
  kind?: AgentActionKind;
  prompt?: string;
  url?: string;
  markdown?: string;
};

type AgentActionKind =
  | 'agent'
  | 'copy'
  | 'continue'
  | 'export'
  | 'prompt'
  | 'regenerate'
  | 'run'
  | 'approve'
  | 'reject'
  | 'open'
  | 'open_url';
```

`id` and `title` are required. At least one runnable payload field is required:
`url`, `prompt`, or `markdown`.

## Limits

| Field      |             Limit | Notes                                                                      |
| ---------- | ----------------: | -------------------------------------------------------------------------- |
| `id`       |  1,024 characters | Trimmed. Empty or oversized values are rejected.                           |
| `title`    |  1,024 characters | Trimmed. Empty or oversized values are rejected.                           |
| `kind`     |  1,024 characters | Lowercased and matched against the allow-list. Unknown kinds are rejected. |
| `prompt`   |  1,024 characters | Trimmed. Empty values are omitted.                                         |
| `url`      |  2,048 characters | Must be safe public HTTPS.                                                 |
| `markdown` | 16,384 characters | Trimmed. Empty values are omitted.                                         |

## URL Policy

Action URLs must pass the same remote-content safety policy used for agent
artifacts:

- HTTPS only.
- No credentials in the URL.
- No localhost, private IPv4, private IPv6, link-local, or local network host.
- No local hostname suffixes such as `.local`, `.lan`, or `.internal`.

Unsafe URLs reject the whole action.

## Kind Behavior

- `copy`: platform may copy `markdown`, then `prompt`, then `title`.
- `open` and `open_url`: platform may open a safe HTTPS `url`.
- `agent`, `continue`, `export`, `prompt`, `regenerate`, and `run`: allowed
  typed actions for higher-level agent workflows. They are forwarded or handled
  only by code that explicitly supports the kind.
- `approve` and `reject`: platform may submit a controlled
  `in.synara.agent.action` Matrix event in the same room after explicit user
  activation. Platforms must not execute the underlying requested work locally.
- Missing `kind` with a safe `url` is treated as an open-url action for current
  desktop compatibility.
- Unknown `kind` values are rejected. Platforms must not infer behavior from
  unknown strings.

## Approval Result Event

Approve/reject actions persist as room events, not local-only client state. The
iOS MVP sends an authenticated `m.room.message` event with `msgtype: m.notice`
and a bounded `in.synara.agent.action` object:

```json
{
  "msgtype": "m.notice",
  "body": "Approved agent action: Deploy",
  "in.synara.agent.action": {
    "version": 1,
    "action_id": "deploy",
    "action_title": "Deploy",
    "decision": "approve",
    "source_event_id": "$source:example.org",
    "created_at": 1770000000000
  }
}
```

`decision` is either `approve` or `reject`. Readers must ignore unsupported
versions and must not treat the event as authorization to run arbitrary local
commands.

## Security Rules

- Never execute arbitrary local shell commands from an agent action.
- Never infer actions by scraping rendered HTML or plaintext.
- Never open non-HTTPS, credentialed, localhost, or private network URLs.
- Reject actions missing `id`, `title`, or a runnable payload.
- Treat rejected actions as unhandled; UI may fall back to copying safe prompt
  text when available.
- Do not include access tokens, device tokens, APNs tokens, recovery keys, or
  decrypted private message content in action payloads.

## Fixtures

### Safe Copy Action

Input:

```json
{
  "id": "export",
  "title": "Export Thread",
  "kind": "COPY",
  "prompt": "Copy this prompt",
  "markdown": "# Thread"
}
```

Expected normalized action:

```json
{
  "id": "export",
  "title": "Export Thread",
  "kind": "copy",
  "prompt": "Copy this prompt",
  "markdown": "# Thread"
}
```

### Safe URL Action Without Kind

Input:

```json
{
  "id": "artifact",
  "title": "Open artifact",
  "url": "https://artifacts.example.org/report.html"
}
```

Expected normalized action keeps the URL and omits `kind`.

### Malicious Examples

These must be rejected:

```json
{ "id": "", "title": "Missing id", "prompt": "Continue" }
```

```json
{ "id": "unsafe-url", "title": "Open private", "url": "https://127.0.0.1/private" }
```

```json
{ "id": "unknown-kind", "title": "Execute", "kind": "shell", "prompt": "rm -rf /" }
```

```json
{ "id": "no-payload", "title": "No payload" }
```

## iOS Notes

- Swift should implement the same allow-list, limits, URL policy, and
  fail-closed behavior before wiring actions to native UI.
- Notification actions for approve/reject require a separate threat model and
  authentication policy; they should not reuse arbitrary agent action payloads
  blindly.
- App Intents may expose only explicitly supported safe actions, not the raw
  action payload channel.

## Acceptance Criteria

- Runtime tests cover safe actions, safe URL actions, unsupported kinds, unsafe
  URLs, missing fields, missing runnable payloads, and oversized fields.
- Desktop shell still validates the payload independently before acting.
- Future iOS tests can use the fixtures above as conformance examples.
