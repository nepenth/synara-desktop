# Client reliability CI coverage

This coverage-only branch starts from integration `39bbcea2`. It adds no product
or test implementations and depends on the pending navigation/live-read branches
being merged before running the combined CI jobs.

Frontend CI now runs `npm run test:browser:native-timeline` immediately after the
existing timeline browser command, in the same `synara` working directory and
using the existing Chromium installation. It is a required step with ordinary
failure propagation. The command comes from the navigation branch; its dedicated
Playwright configuration enumerates the full native presenter suite. A light
`--list` check on navigation head `7a348762` found 45 Chromium cases (including
the 33 cases present at the initially reviewed `ddda4f3f`). No browser execution
or native build was performed for this wiring change.

The main iOS UI CI lane already invokes `ci-build.sh` with `IOS_TEST_SUITE=ui`,
which selects the whole `SynaraUITests` suite and therefore includes the new UI
methods after merge. It does not invoke the separate `run-ui-tests.sh` shard
runner. The UI selector additions repair that local/selective runner's omissions;
the missing CI execution step in this change is the native browser command.

The local/selective `timeline-and-composer` UI shard now includes unread-position
restoration, missing-marker recovery, send-from-history navigation, and all four
stable-viewport navigation cases. The `live-and-visual` shard includes both
synthetic live-read cleanup regressions beside the existing opt-in live-read
proof. These two cleanup tests require no live credentials. The existing live
proof remains opt-in. No shard was removed or renamed.

Light verification completed:

- Parsed the workflow YAML and checked that the required native browser step
  follows Chromium installation and the existing timeline command, with the
  correct working directory and no continue-on-error setting.
- All 16 existing CI-scope regressions passed; the existing change gates retain
  their behavior.
- Shell syntax and diff whitespace checks passed.
- Executed the actual shard runner with a disposable `xcodebuild` command-capture
  stub, so no build or simulator launched. `all` and each individual shard
  produced the exact selectors declared by the arrays: 11 auth/room, 17
  timeline/composer, 11 settings/workflow, and 16 live/visual selectors, totaling
  55 without duplicates.
- Compared every selector against the combined test methods in integration
  `39bbcea2`, navigation `ddda4f3f`, and live-read `13578624`. All nine newly wired
  selectors exist; both newly added navigation methods and both cleanup methods
  are covered. Five already-present navigation methods omitted from the old
  arrays are also included.

This establishes command and selector coverage only. The pending branches own
their runtime evidence; the final combined integration still needs its CI run.
