# 1001 — The walk that answers for the file, and the exemption it unblocked

Date: 2026-09-12. Branch `batch-999-1004`, worktree `/home/AI/pdf-viewer-rounds`, one of six
rounds run side by side. Topic: the validator's per-object contract —
`doc/reviews/984-direction-and-boundaries.md` Finding 5 and Q2, ADR 1005 §5, and the item that had
been blocked on it, `doc/todo/62`. ADR **1021**; question **Q61**.

## What was done

- **`crates/pdf-archive/src/reach.rs`** (new, 952 lines). `Reach` — one bounded walk from the
  trailer's references and the two populations the file's structure names, recording per object
  the run of entries it was reached through, plus the objects a cross-reference section names and
  the walk does not. `Exempt` — ISO 19005 section 6.2.2's last sentence as a population, computed
  as the difference between that walk and the same walk with the exemption's edges cut.
  `exemption_narrows` — the two carve-outs, read by clause.
- **`Examination::reaches()` and `Examination::exempt()`**, each a `OnceCell` like the five that
  were there, and the first that answers a question about a *relationship* rather than handing
  out an enumeration.
- **`Findings::exempting` and `Findings::named_an_object`**, and `check` re-running a failing,
  narrowed requirement's predicate against the first when the second says a second run could
  change anything.
- **`examples/unreferenced.rs` retired onto the method** — 481 lines to 212, the census
  reproducing figure for figure, with two lines added that the method made printable: how many
  documents hold an object the trailer reaches not at all, and how many stopped a walk at a bound.
- **`doc/todo/62` taken and closed**, its A010 record moved out of `clarification`'s owed list
  with the argument for why that table has no key for it, and `coverage`'s two `Scoping` sentences
  at section 6.2.2 made true.
- **`doc/questions/Q61`** — the observer question, with the third answer this round recommends.
- One `editions::SHIFTS` row, §7.7.2, which is *Document Catalog* in ISO 32000-1:2008 and the
  crate's own test demanded the moment a comment cited the number.

## What it found

Three things, each recorded where the code is:

1. **An over-exemption inherited from the measurement**, in two halves — an object holding a
   `/Resources` key was an owner whatever it was, and an owner with no content was an owner whose
   content references nothing. ADR 1021 §4 has both, with what each moved.
2. **The naive wiring cost 23–26 s on a file the baseline does in 5–6 s**, and both causes were
   measured rather than guessed: a fixpoint re-reading every object per round, and a second
   predicate run for requirements whose places name no object at all. ADR 1021 §5.
3. **Twelve fixtures in `crates/pdf-transform/tests/archive.rs` fail**, every one a document whose
   page declares a resource its empty content stream never names. The validator is right; the
   fixtures do not exercise what they claim. ADR 1021 §7 lists them and the one-line fix, and this
   round did not touch that crate.

## Gates

`cargo fmt --all --check`, `cargo test --workspace --doc`, both `fuzz/` lines, `cargo test -p
conformance` (234 + 2 + 1 + 2 + 1 + 1, all passing), `pdf-archive`'s corpus harness and
`tools/state.sh archive`: `over` 0 on all six targets, `missed` 1 on PDF/A-2b at section 6.6.2.3.3
and pre-existing, the converter conforming in and out on all six. The coverage frontier is empty.
`cargo nextest run --workspace`: 4444 of 4456, the twelve above. `cargo clippy --workspace
--all-targets` could not complete while this round ran — `crates/pdf-render/src/paint.rs:559` is
another round's file mid-edit; `-p pdf-archive --all-targets` is clean on every crate it owns.

The corpus figures were taken **twice**, once with the narrowing switched off and once with it on,
which is the only way to say that an exemption withdrew nothing: the same six columns both times.
