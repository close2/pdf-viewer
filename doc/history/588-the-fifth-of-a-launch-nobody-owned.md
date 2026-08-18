# 588 — The fifth of a launch nobody owned

**Finding:** the owner's ten-second document now appears in one and a half seconds, and the largest
part of it that is *not* interpretation had never been divided: `FrameCost::scene` is one number
covering this host's walk of a display list and every resource it hands across quorra's boundary.
Divided, four fifths of a cold frame's scene is inside `upload_outline`, and six sevenths of that is
quorra converting the page's cubics to quadratics — a representation only its GPU coverage lane reads
and no launch of this viewer takes.

Date: 2026-08-18. ADR: [0423](../adr/0423-the-fifth-of-a-launch-nobody-owned.md).

Touched: `crates/render-quorra/src/cache.rs`, `crates/render-quorra/src/scene.rs`,
`crates/render-quorra/src/present.rs`, `crates/render-quorra/src/lib.rs`,
`crates/render-quorra/examples/zoom_frame.rs`,
`crates/viewer-ui/src/bin/pdf-viewer/timing.rs`, `crates/viewer-ui/src/bin/pdf-viewer/surface.rs`,
`doc/conformance/ledger.toml`, `doc/QUORRA_FEEDBACK.md`, `doc/performance.md`,
`doc/running-the-viewer.md`, `doc/todo/44-a-draft-that-takes-ten-seconds.md`,
`doc/todo/45-where-a-frame-goes.md`, `doc/todo/README.md`.

## The round was sent at a hole that had already been closed

`doc/todo/README.md`'s line for item 44 named "the launch-table hole `--trace` must learn to name",
and item 44's own file has said since session 497 that the hole was closed. The round's first hour
went on establishing that, and it was not wasted: the *index* line was the last place in the tree
still claiming ten seconds, the owner's own later traces (`tmp/trace2`, `tmp/trace3`) had been
sitting beside the first one saying 1.6 and 1.7 seconds, and nobody had read them back.

**A one-line index that restates a file's header is a second copy to keep in sync** — which is what
`doc/todo/README.md` says about itself, in the paragraph above its own index. This is the second
time that has cost a round; the first is recorded in the same paragraph.

## Where the round went instead

The table had no gap, so the question became what is *inside* a row. Two rows of it are one frame —
`first scene built` and `first present` — and between them they are half of this launch. Neither
could be divided: `scene` had never been divided at all, and the device's three phases were printed
only in the summary at exit, as medians over a whole run, which cannot answer a question about the
one frame a person waited for.

Both are closed. The measurement then said something nobody had asked: an *upload* is not a copy.

## What is worth remembering, beyond this document

- **A share is about a population, and "nobody has needed to divide it" is a claim about which
  frames were measured.** `doc/todo/45` §5 recorded `scene` at 2.5% of a zoom frame and concluded
  nobody had needed to divide it. True of the frames that file measures — page turns and zoom steps
  — and false of the frame a person judges the program by, where it is a fifth. The sentence that
  replaces it names its population.
- **Two instruments, one answer.** The wall clock said 83% and callgrind said 82%, and the second
  then went one level down to the function. Either alone would have been an argument; together they
  are an attribution, and the ask that went upstream carries both tables.
- **A new clock owes an A/B against the code it is measuring.** `ResourceCaches`' own comment had
  said, years of sessions ago, that a timer at each upload site "would cost a clock read" where a
  counter costs an increment. Answered rather than ignored: 116 058 clock reads on the frame that
  pays most are below this machine's run-to-run spread, and the *on* arm holds the lowest of the four
  samples.

## The spec-driven half

§10.8.3 — separation simulation — was `reported` with no `code` and no `test`, and its note said the
simulation is one "a processor performs when asked to, and nothing in a PDF asks for it". Table 275's
`SeparationSimulation` requirement type asks for exactly that, and `pdf_model::requirements` has
carried the type by name for as long as it has existed, **citing §10.8.3 in its own doc comment**.
The row now names the code and the two tests that hold the report, and the reading sharpens what is
owed: §10.8.3 imposes nothing — its verb is a permission and its steps are a `should` conditional on
performing one — so the debt is owed to Table 275, and what is missing is the control.

## Gates

Every gate this round could reach was run after the last edit: `fmt`, `clippy --workspace
--all-targets` silent, `nextest --workspace`, `--workspace --doc`, `conformance`, the `pdf-model`
corpus gate and the `render-quorra` corpus gate — the last being the one this round's code can
actually move, and its ratchets held. Four of `doc/todo/01`'s sweeps were run and flagged nothing
this round wrote. The oracle was not re-run: nothing here touches `render-cpu`, which is what it
rasterises with. `tools/state.sh` prints every number.
