# ROE-12: Validation, Sanitization, and Security Rules

Prior: **share protocol rules and fixtures, not a universal sanitizer crate**.

Core should own protocol validation, resource bounds, canonical identifiers,
eligibility, and security-sensitive normalization when the threat model is
identical. Desktop DOM/React output and Swift attributed-text output require
different context-specific sanitization and remain platform-owned.

## Bounded research question

Across untrusted event fields, formatted bodies, URLs, mentions, filenames,
MIME metadata, agent actions, widget data, and account data, which rules are
truly protocol authority and which are renderer escaping? Start with a shared
malicious/golden corpus covering schemes, Unicode/confusables, depth/size,
recursion, parser/regex denial of service, unknown fields, fail-closed errors,
and redacted diagnostics.

The `TimelineViewRow.formatted_body` contract must be described accurately:
Core projects Matrix formatted HTML, while each presenter still validates and
sanitizes for its output context. Documentation or comments must not imply the
HTML is universally safe merely because Core transported it.

Only a proven identical rule should move to Core. A safe semantic field may be
considered through ROE-04's fixture-first ladder; DOM trees, attributed strings,
or renderer instructions must not cross the boundary.
