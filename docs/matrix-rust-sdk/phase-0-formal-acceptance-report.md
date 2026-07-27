# Phase 0 formal acceptance report (R0.8)

| Field | Value |
|---|---|
| Report ID | `phase-0-formal-acceptance-report` |
| Twin | [`phase-0-formal-acceptance-report.json`](phase-0-formal-acceptance-report.json) |
| Issued by | **R0.8 only** (plan §R0.2/R0.8 authority) |
| Subject tip | `e7d9cb992b8f7e648a232b8d006f7f3e5f3c77d2` |
| Product evidence | `3a2fa6f400762f95b4851d02206d5be8ee550c66` |
| **Verdict** | **`not_accepted`** |
| Phase 0 gate | **`open`** |
| P0.1–P0.7 strict accepted | **none** |

## Decision

Phase 0 is **not** accepted and the Phase 0 gate is **not** closed.

R0.8 is the only authority that may later flip this report to `accepted` and
close Phase 0. This revision exists so the program has an explicit, residual-
honest formal report rather than an implied “phases complete” claim.

## Blocking residuals

1. **R0.2 open** — E1 PR [#82](https://github.com/nepenth/synara-desktop/pull/82)
   parked on identical `v2 exceeded 512 MiB` residual; Phase 0 evidence readiness
   not accepted.
2. **Phase 0 evidence manifest** — rows remain open/stale/blocked; only
   `accepted` evidence satisfies gates
   ([`phase0-evidence-manifest.md`](phase0-evidence-manifest.md)).
3. **No independent accept attestation** bound to this subject for Phase 0 close.

## Evidence / CI context

See [`r0.8-phase-gate-readiness-inventory.md`](r0.8-phase-gate-readiness-inventory.md).
Green Quality gate on product evidence does **not** clear Phase 0 residuals.

## Non-claims

- Does not accept P0.1–P0.7.
- Does not approve bounded deferrals in this revision.
- Does not authorize dual-backend or cutover.
