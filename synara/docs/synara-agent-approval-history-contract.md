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

The complete encoded account-data object is capped at 256 KiB. Oversized reads
and writes fail closed instead of parsing or publishing a partial replacement.

Writers append after a successful `matrix_agent_approval_decide` reaction send.
A history-write failure must not fail the decision. Duplicate `(roomId, eventId)`
rows keep the newest `decidedAt`. Items older than 30 days are dropped. The
list is newest-first and capped at 200 items.

`summary` is at most 240 Unicode scalar values: the first useful fenced/Code
command line, else the `Reason:` text, else the first non-heading line.
Whitespace is collapsed. Writers must not store the full command body, access
tokens, or other secrets.

Core serializes read-modify-write mutations within one running process and
fetches the current server value before each mutation; it does not rely on a
possibly stale `/sync` account-data cache after a write. Read-only snapshots
use the SDK's synchronized local store. After a successful local append, the
timeline owner projects the returned items for at most 30 seconds so a stale
SDK cache cannot erase the acknowledged change before `/sync` catches up.

Desktop surfaces Recent as the union of this history and in-memory inbox
items whose status is not pending. Account-data records win on
`(roomId, eventId)`. History items render as decided, not expired.

Schema and fixtures:

- `docs/contracts/synara-agent-approval-history-content.schema.json`
- `docs/contracts/fixtures/synara-agent-approval-history-content.json`
