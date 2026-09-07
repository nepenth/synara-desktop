# NSE production isolation check

Goal: let CI build the narrow notification extension while rejecting any
production dependency path that enables the full Core UniFFI surface.

The CI runner starts from a clean checkout, runs the existing scaffold checks,
generates Core and NSE Apple packages, then checks the NSE production feature
graph and archive exports before Xcode. Cargo owns feature resolution and the
Apple archive owns the linked export evidence. Test-only login/sync fixture
capabilities must not be mistaken for production linkage. Production normal or
build dependencies that activate full-uniffi, a failed graph query, or a full
Core export in an NSE archive must fail the proof. Scope is generated local
artifacts and disposable Cargo fixtures; there are no live-account, trust,
notification-preference or release changes.

The failed GitHub run 34073410964 stopped both iOS lanes after successful NSE
Apple generation and before Xcode. Its feature query used `-e features`, which
includes dev dependencies. On this exact source, inverse feature readback shows
`default` and `full-uniffi` activated only through synara-nse-core's dev dependency;
normal/build feature readback contains only `nse-preview`. The workspace uses
resolver 2. This is a guard-input defect rather than evidence of production
full-Core linkage. Actual Apple graph/archive verification remains required.

Cargo documents [edge selection](https://doc.rust-lang.org/cargo/commands/cargo-tree.html)
and [resolver 2 dev-feature separation](https://doc.rust-lang.org/cargo/reference/resolver.html).

Repair: CI now calls a small production-feature checker that reads Cargo's
normal/build feature graph, excluding development edges. It collects the
completed query output and fails on any Cargo error, so an early-consuming
pipeline cannot turn a query failure into success. The existing archive
forbidden-export block is unchanged. The new checker is included in the iOS
change scope so edits to it schedule the applicable iOS gates.

Post-repair verdict: **Confirmed** for the production feature/archive boundary;
the complete remote iOS jobs still require a new CI run.

- Four actual Cargo-workspace regressions passed: dev-only full-uniffi remains
  permitted (the original query reproduces the rejection), normal dependency
  leakage fails, build dependency leakage fails, and a failed query fails.
- The actual repository's host, iOS simulator arm64 and iOS device arm64
  normal/build graphs all contain only the `nse-preview` Core feature.
- The production NSE generator rebuilt this source with Cargo 1.93.1 and the
  `nse-release` profile for `aarch64-apple-ios-sim`, generated Swift and namespaced
  FFI headers, and published the XCFramework. No test feature flag was passed.
  Build evidence: `/tmp/synara-nse-isolation-apple-proof.log`.
- Completed `nm -gU` output for the newly generated archive had exit status zero,
  10 NSE exports and zero `_uniffi_synara_core_` exports. Evidence:
  `/tmp/synara-nse-isolation-archive-proof.log`.
- The exact feature/archive block extracted from the updated CI script ran
  against those generated artifacts and returned zero. Evidence:
  `/tmp/synara-nse-isolation-ci-block.log`.
- The existing scaffold checker and shell syntax passed. All 17 CI-scope
  regressions passed, including an isolated checker change scheduling the two
  iOS gates on main push. An initial scope-test setup used a feature PR, whose
  documented default skips iOS; correcting the trigger to main push preserved
  the actual scheduling contract. Evidence:
  `/tmp/synara-nse-isolation-scope-tests.log` and
  `/tmp/synara-nse-isolation-feature-tests.log`.

No feature definitions, NSE dependency declarations, privacy defaults, trust
state, generated artifact policy, or archive export exclusions were weakened.
