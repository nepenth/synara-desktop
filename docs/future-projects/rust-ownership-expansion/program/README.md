# Residual-census research record

Status: closed docs-only research run, conducted 2026-09-01, followed by a
separately human-authorized remediation pass on 2026-09-02. The research
record is not standing approval for further product implementation, a new
shared-Core phase, a scoreboard, or a release gate.

This folder preserves the provenance and results of the completed
`feature/rust-ownership-residual-census` research run. It is historical
evidence, not a live multi-agent operating protocol. Current work must follow
the accepted ADRs, current source, and current goal graph.

The run answered two different questions that must not be collapsed:

1. **Ownership verdict:** is a behavior owned by Core, a platform observer, or
   a platform renderer, and is harmful duplicate authority present?
2. **Residual action:** did the census reveal a product defect, evidence gap,
   transport/parity gap, security proof requirement, or future design choice?

An ownership verdict can be closed while a residual action remains open.

| File | Durable role |
| --- | --- |
| [ACTIONS.md](ACTIONS.md) | Authoritative residual action, remediation, and proof-gate register |
| [STATE.md](STATE.md) | Reconciled outcome of the research and authorized remediation |
| [TRACKING.md](TRACKING.md) | Memo provenance, ownership verdicts, remediation, and residual status |
| [DECISIONS.md](DECISIONS.md) | Historical run decisions plus later corrections |
| [CENSUS.md](CENSUS.md) | Dated starting source snapshot; never live inventory |
| [OPERATING.md](OPERATING.md) | Archived protocol used during the 2026-09-01 run |

No document here provides standing permission for future product, Core,
UniFFI, ADR, playbook, goal-graph, or release-state changes. The dated
2026-09-02 remediation was separately authorized and still required the normal
architecture, test, independent-review, and promotion gates.
