# Q875 — May a toolchain bump change only the lint form of your audit tests?

Source: round 2047, which prepared moving the toolchain pin from 1.97.1 to 1.100.0 (ADR 2910). Round
2000's rule stands beside it: the owner's audit tests are not edited to make a change pass, and a fix that
touches one is a question.

## What the new compiler finds in them

Clippy 1.100 adds two lints that fire across the whole workspace and changes two others:

- `chunks_exact_to_as_chunks` (default set): `chunks_exact(4)` with a constant length becomes
  `as_chunks::<4>().0`, which yields arrays instead of slices.
- `assert_is_empty` (pedantic): `assert!(v.is_empty())` becomes `assert_eq!(v, [] as [T; 0])`, so a failure
  prints what was there.
- `map_unwrap_or` now also names `.map(f).unwrap_or_default()`, written `.map_or_default(f)`.
- `float_cmp` no longer fires inside `assert_eq!`, so an `#[expect(clippy::float_cmp)]` above such an
  assertion is now unfulfilled, and `-D warnings` makes that an error.

Twenty-seven files that the owner's audit commits added (the `*_audit.rs` tests and the files beside them)
trip these lints. The edit to each is in its lint form only. The values compared and the inputs built stay
the same: `chunks_exact(4)` becomes `as_chunks::<4>().0` with one `*` where an array meets an array, an
emptiness assertion becomes the `assert_eq!` form clippy writes, and an unfulfilled expectation is removed,
with its reason shortened to the lints that remain. The edits are written and checked, 523 lines held
outside the tree as `~/scratch/agent/r2047/audit-tests-lint.patch` (`git apply` from the checkout's root).

## The question

May a toolchain bump make lint-only edits like these to your audit tests, where the assertion's operands and
outcome stay the same?

- **(a) Recommended: yes, for lint form only.** Each such edit is listed in its round's record. Any edit that
  changes what an audit test builds, reads or expects stays a question, as round 2000's rule says.
- **(b) No.** The toolchain stays at 1.97.1 until you, or a round you name, rewrite each audit file.

## What the tree does meanwhile

It stays on 1.97.1. The bump waits for a second reason as well: Cargo 1.100 moves test executables out of
`deps/`, and the test programs here find their workers from that path (ADR 2910). The rest of the bump's lint
fixes are held as `~/scratch/agent/r2047/toolchain-1.100.patch`. Nothing in the tree depends on the answer.
The round that makes the lookups layout-independent applies both patches once you have answered.
