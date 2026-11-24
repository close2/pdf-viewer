# 3574 — The census counts every default, and holds each file's silent ones to a ratchet

Session 2218. Status: **accepted**. Context: ADR 3477 (the grep census), ADR 3573 (the mapping
gate), review 2194's census (section 1), ISO 32000-2 clause 7.

## Context

ADR 3477's fourth check keeps "every `unwrap_or*`, `.ok()` and `.clamp(` on a value read from the
file in the three crates either beside a class or under a `// not a repair:` comment naming the
clause". A grep cannot tell a file's value from a count or an index, so the rule has to say what it
counts and what answers a site.

## Decision

- **Population**: every line of non-test code in `pdf-syntax`, `pdf-model` and `pdf-font` that
  calls one of the three shapes. Test code is a `…_tests.rs`, `…_audit.rs`, `tests.rs` or
  `fixture.rs` file, a `tests/` directory, and a file's code from its first top-level `#[cfg(test)]`
  module on. The writers are excluded by path (`serialize.rs`, `write.rs`, `linearize.rs`, the
  font embedders, `view/writing.rs`): what this program emits is not a file it was given. Every
  module of `pdf-model` is in, not only those on the validator's path, because a hand list of
  modules is trap 25.
- **Answers**: a class item or a recording call (`record_repair`, `record_repairs`, `note_repair`,
  `noted`) within twelve lines either side; or a `// not a repair: <reason>` comment within six
  lines above, or trailing the site. The reason is a clause that makes the default the
  specification's, or what the value is when it is not the file's. A marker reaches six lines and
  not twelve because, at twelve, 76 sites were answered by a marker written over another default.
- **A per-file ratchet** (`HELD` in `tools/conformance/tests/repair_census.rs`): a file with more
  silent sites than held fails (a new default arrived silent), and one with fewer fails until the
  bound is lowered with the change. Every site was read before it was marked; a site read as a
  repair is left counted, because marking it would make the census lie, and logging its class is
  a fix round's work.
- The markers were written by six Sonnet helpers under one brief that states this rule, each
  reading the site; the round checked the ones that cite a clause and removed one with no clause
  behind it.

## Consequences

- The gate holds the count at what it is and lets it only fall; the test prints the count on every
  run.
- Known limits, stated: a default in another shape (`match`, `if let … else`, `filter_map`) is
  outside the grep; a class named within the window need not be the one this default makes; code
  after a test module is not counted (the round's plant after one was not seen, before one was).
