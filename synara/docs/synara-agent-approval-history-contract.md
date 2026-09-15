# Synara Agent Approval History Contract

Reviewed: 2026-09-14

Status: shared contract with Core writers in
`crates/synara-core/src/app/account_data/agent_approval_history.rs` and desktop
projection in `src/app/features/approvals/`. The canonical writer schema and
fixtures live under `docs/contracts/`.

## Purpose

Recent approvals must survive session restarts and sync across Synara
clients. The durable record is global Matrix account data. Account data is
server-readable plaintext, including for encrypted rooms, so writers store a
bounded command-preview **line** and never the full command body.

## Account Data Event

History is stored in global Matrix account data:

```text
in.synara.agent_approval_history
```

## Payload Model

Machine-readable artifacts:

- [synara-agent-approval-history-content.schema.json](./contracts/synara-agent-approval-history-content.schema.json)
- [synara-agent-approval-history-content.json fixtures](./contracts/fixtures/synara-agent-approval-history-content.json)

```ts
type SynaraAgentApprovalHistoryContent = {
  version: 1;
  items: SynaraAgentApprovalHistoryItem[];
};

type SynaraAgentApprovalHistoryDecision = 'approve_once' | 'approve_always' | 'deny';

type SynaraAgentApprovalHistoryItem = {
  roomId: string;
  eventId: string;
  sender: string;
  decision: SynaraAgentApprovalHistoryDecision;
  decidedAt: number;
  originServerTs: number;
  expiresAt: number;
  summary: string;
};
```

Live Core readers and writers fail closed when the event is missing a version
or carries a version other than `1`. They must not normalize a newer payload to
v1 and write it back. Missing content normalizes to `{ "version": 1, "items": [] }`.
Malformed items inside a recognized v1 array may be discarded. A non-array
`items` value fails closed.

The canonical writer schema sets `additionalProperties: false` on the content
object and on each item, matching Later and room-notes writer schemas. Readers
ignore unknown fields; they must not persist a rewritten v1 document that
strips fields they do not understand from an unsupported version.

The complete encoded account-data object is capped at 256 KiB. Oversized reads
and writes fail closed instead of parsing or publishing a partial replacement.
That byte cap is enforced by Core, not by JSON Schema.

Writers append after a successful `matrix_agent_approval_decide` reaction send,
or after an AlreadyDecided tap whose existing **own** terminal reaction matches
the requested action. A mismatched tap must not invent a history row. A
history-write failure must not fail the decision. Duplicate `(roomId, eventId)`
rows keep the newest `decidedAt`. Items older than 30 days (`decidedAt` older
than `now - 30 * 24 * 60 * 60 * 1000` ms) are dropped. The list is newest-first
and capped at 200 items. Retention and the 200-item cap are writer prune
policies; the schema `maxItems: 200` bound is the stored canonical payload.

`summary` is at most 240 Unicode scalar values: the first useful fenced/Code
command line, else the `Reason:` text, else the first non-heading line.
Whitespace is collapsed. Empty summary is allowed when no preview line exists.
Writers must not store the full command body, access tokens, or other secrets.
Core writers also reject control and bidi format characters in `summary`, and
replace path-like arguments (`/tmp/x`, `~/x`, `./x`, `../x`) with `<path>` and
secret-like assignments or flags (`TOKEN=…`, `--token …`) with `<redacted>`.
Bare filenames stay (`rm file`). Pending in-app Command previews are not this
field; they are derived from the live prompt body.

### Item validation (Core codec)

Canonical items use camelCase JSON field names. All three timestamps are finite
milliseconds since Unix epoch and must be `> 0`. `expiresAt` must be greater
than `originServerTs` (JSON Schema cannot compare two properties; Core drops
the item). `decision` is exactly `approve_once`, `approve_always`, or `deny`.

Identity bounds:

- `roomId`: Matrix room ID. Starts with `!`, includes a `:server` suffix Core
  parses as a Ruma server name, no whitespace/control/bidi characters, at most
  255 UTF-8 bytes. Canonical writers emit ASCII IDs, so schema `maxLength: 255`
  matches.
- `eventId`: starts with `$`, no whitespace/control/bidi characters, at most
  255 UTF-8 bytes.
- `sender`: Matrix user ID `@localpart:server`, at most 256 Unicode scalars,
  no whitespace/control/bidi characters.

The desktop snapshot command `matrix_agent_approval_history_snapshot` returns
`{ items }` only (no `version`). That DTO is not this account-data schema.

Core serializes read-modify-write mutations within one running process and
fetches the current server value before each mutation; it does not rely on a
possibly stale `/sync` account-data cache after a write. Matrix global account
data has no `If-Match`. After each PUT, Core fetches again: if the confirmed
document lacks the written `(roomId, eventId)`, it merges into the latest
server value and retries (three attempts). A successful PUT whose confirm
fetch fails returns the locally written items. Read-only snapshots use the
SDK's synchronized local store. After a successful local append, the timeline
owner projects the returned items for at most 30 seconds so a stale SDK cache
cannot erase the acknowledged change before `/sync` catches up.

Desktop surfaces Recent as the union of this history and in-memory inbox
items whose status is not pending. History items render as decided, not
expired. Account data is not proof of a Matrix reaction:

- History-only rows stay visible (another Synara device, or Recent before
  inbox discovery).
- When both exist, an inbox `decision` wins the conflict.
- A decided inbox row without `decision` does not inherit the history
  decision (homeserver-injected well-formed history cannot label that row
  "Approved always").
- An expired inbox row without `decision` may still show the history
  decision (prompt TTL is not the decision; the other device may have
  written history).

Schema and fixtures:

- `docs/contracts/synara-agent-approval-history-content.schema.json`
- `docs/contracts/fixtures/synara-agent-approval-history-content.json`
