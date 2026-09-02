# ROE-12: Shared Validation, Sanitization, and Security Rules

Hypothesis: protocol validation, resource bounds, URL/event safety policy, and
security-sensitive normalization should be shared where the threat model is
identical, while renderer-specific DOM and attributed-text sanitization may
need to remain with each presenter.

Investigate:

- untrusted Matrix event fields, formatted bodies, links, mentions, filenames,
  MIME metadata, agent actions, widget data, and account data;
- canonical protocol validation versus output-context-specific escaping;
- URL schemes, Unicode/confusables, nesting/depth, size, recursion, regex and
  parser denial-of-service, and unknown future fields;
- consistent fail-closed errors and redacted diagnostics;
- whether a shared safe semantic model can reduce duplicated sanitizer policy
  without creating unsafe HTML or platform UI instructions in Core.

Minimum proof: threat model, fuzz/property tests, malicious corpus, resource
limit benchmarks, cross-language contract tests, DOM/SwiftUI output-context
tests, security review, and regression fixtures for every accepted finding.
