# Round 2169 — five leftovers of the audit queue

A fix round in the `gwg` worktree for the leftovers of rounds 2141, 2147, 2163 and 2165.

## What changed

- ADR 3377: `lockdown::apply_for_decoder_within` off Linux answers `apply_for(Profile::Decoder)`,
  so `unnecessary_wraps` has nothing to fire on; `tests/confinement.rs` states its two Linux-only
  expectations under `cfg_attr(target_os = "linux", ...)`. CI's Windows job runs `cargo check`
  (`tools/platform-check.sh`), not clippy, so neither was CI's.
- ADR 3378: `quorra-transform-worker` and `pdf-vfs-worker` call `keep_only_the_standard_descriptors`
  first and forbid `unsafe`; `callers.rs` names seven helpers. The vfs worker's host loads beside
  GLib; the transform broker's own inherited descriptors pass through its spawn.
- ADR 3379: `Td`/`TD`/`Tm` operands and `"`'s `aw`/`ac` that are not numbers are
  `StreamSyntaxFault::TextOperandNotANumber`; `TD` sets the leading from a numeric `ty` alone, by its
  stated `-ty TL tx ty Td`.
- ADR 3380: `MAX_OPERANDS` still counts objects (an array counted as one would need a second
  bound on its elements), raised from 8 192 to 65 536; a 10 000-element `TJ` draws whole. The
  report's name, which ADR 3267's archive check reads, is unchanged.
- ADR 3290's `Damage::Unframed` has its own sentence in `viewer-core`'s `report.rs`, with a test.

## Ledger and gates

**Ledger.** No status moves: 9.4.2 and 9.4.3 stay `implemented`, notes extended and three tests
added to their lists. `crate-map.md` and `state-of-play.md` name the two new callers.

**Gates.** One `-p` set — `pdf-sandbox inherited-descriptors pdf-model viewer-core viewer-qt
pdf-transform pdf-vfs`: fmt 0; clippy `-D warnings --all-targets` 0; `pdf-sandbox --bins` 0;
nextest `--lib --bins --tests` (`xvfb-run -a`) 5091 passed, 58 skipped (`audit-diff.py`: failed 0,
expected 0). Windows gnu and msvc: clippy `-D warnings --all-targets -p pdf-sandbox -p
inherited-descriptors` 0, `check -p pdf-transform --bins` 0. rustdoc `-D warnings
--document-private-items` of six crates 0. Fuzz `check --bins` 0 (`StreamSyntaxFault` gained a
variant). Planted back, the 8 192 cap fails the 10 000-element test and the old `TD` the leading
test. `cargo test -p conformance --no-fail-fast` 0.

## Habit

A lint expectation on a crate that builds for several targets is fulfilled on each or stated
under `cfg_attr` for the one that uses it; `-D warnings` turns an unfulfilled one into an error.
