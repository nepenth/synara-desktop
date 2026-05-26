# Synara Agent Card Contract

Reviewed: 2026-05-26

Synara renders structured agent cards from explicit event-content keys:

- `org.hermes.agent`
- `io.hermes.agent`
- `in.synara.agent`
- deployment-configured keys

Synara must not render arbitrary JSON messages as agent cards. Plain JSON bodies
only become cards when they use the explicit `hermes: true` marker supported by
the runtime parser.

Canonical payloads contain a bounded title and at least one visible content
section: summary, actions, artifacts, logs, code, or diffs. Artifact and action
URLs must pass the shared safe remote URL policy. Code/log/diff content is
bounded and rendered as inert text.

The agent-card payload is distinct from `AgentActionPayload`. Agent cards
describe timeline-rendered agent output; agent actions are sanitized bridge
commands/events derived from explicit user activation.

Schema and fixtures:

- `docs/contracts/synara-agent-card.schema.json`
- `docs/contracts/fixtures/synara-agent-card.json`
