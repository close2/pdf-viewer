# Q61 — Does `pdf-model`'s interpreter grow an observer, or do two state machines stay?

Source: `doc/reviews/984-direction-and-boundaries.md` Finding 5, second half; ADR 1005 §5 part
two, which prices it and says it is a design question rather than a task. Raised by session 1001,
which took part one (`Examination::reaches`, ADR 1021) and left this.

## The fact, measured rather than recalled

`crates/pdf-archive/src/lib.rs` opens by saying the validator "adds no reader of its own, and that
is the design rather than an economy". `crates/pdf-archive/src/survey.rs` is 3 715 lines. It takes
tokens from `pdf_model::content::reader::ContentReader` and re-derives everything above them: its
own `State`, its own `q`/`Q`, its own resource-in-force, its own form, pattern and Type 3
traversal — beside the interpreter's `run_reader` in `crates/pdf-model/src/content/run.rs`. The
consumers of `ContentReader` outside the reader itself are exactly three: `pdf-model/src/page.rs`,
`pdf-model/src/content.rs`, `pdf-archive/src/survey.rs`.

`crates/pdf-render/src/lib.rs` states, one layer up, the principle this is on the wrong side of: a
state machine implemented twice has to be implemented *identically*, or the two disagree.

## Why it needs the owner

It is not a refactor with a right answer, and the cost falls on a crate the validator does not own.

**The case for an observer.** `pdf-model`'s interpreter grows a way to watch the resolved state at
each operator — which resources dictionary is in force, what the graphics state is — without
drawing. The survey then *reads* the interpreter instead of re-deriving it, and the one place the
two must agree stops being a place where they can drift. A validator that judges a resource the
renderer never selects is the false failure `survey.rs`'s own header says the crate is written to
avoid.

**The case against.** An observer is a public surface on the interpreter that every future
operator has to feed, and it is a surface whose *shape* is decided by the validator's needs rather
than by the renderer's. The survey's under-report-rather-than-mis-report rule would have to be
re-derived over the three things it deliberately does not walk — a soft mask, an `SMask` image,
a shading function — because the interpreter does walk them. That is where the expense is, and it
is not in the observer itself.

**A third answer exists and is cheap.** Leave two machines and hold them to each other with a
test over the corpus: every resource the survey reports selected is one the interpreter selected,
and the other way round. That converts a drift risk into a gate, costs one test file, and buys
none of the line count back.

## What the tree does meanwhile

Two machines, and nothing holds them to each other. Every round that touches operator handling in
either place has to remember the other exists; the last three that did, did remember. Nothing in
the gates would catch the round that does not.

## Recommendation

The third answer first, and the observer only if it fails. A cross-check test is a day, names the
drift the moment it appears, and is worth having *whichever* way this question is answered — if an
observer is built later, the same test is what proves the survey reads it correctly. Building the
observer first means paying the expensive half before anything has shown the cheap half to be
insufficient.

If the owner wants the observer regardless, the thing to settle in the same breath is **whose
shape it is**: an observer designed for the survey's questions is a validator's interface living
in `pdf-model`, and this project's layer rule would rather have that decided than discovered.
