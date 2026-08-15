# 542 — The geometry phase divides, and the number is the machine's

**Finding.** quorra built the phase this tree asked for in session 533, and the ask came back with a
field whose default is 1 and whose value is the host's to choose. The round is that choice, made by
measurement, plus the two claims that had to be checked rather than accepted.

**The release.** `a64a9084` → `619ef3b4`, thirty-two commits (thirty-three with the merge): the
whole function-paint implementation of quorra's ADR 0053, a thirteen-file split of their device,
their own correction of `Agreement::Exact` to `Bounded`, and — the last two — the divided geometry
phase of their ADR 0054. **Not one line of this tree had to change to compile it**, which is the
fourth release running.

**The number is `std::thread::available_parallelism`, and it is measured rather than assumed.** A
new instrument, `crates/render-quorra/examples/encode_threads.rs`, walks a ladder of thread counts
over a real document with a cold device per sample and takes the minimum of five round-robin
rounds. On the owner's `tmp/Entwurf.pdf` at its fit view, quorra's `encode` falls 467.2 → 150.6 ms
with the machine quiet, 849.8 → 251.8 with it busy, and 1376.0 → 458.7 with more spinners running
than the machine has threads. **Upstream's caution — a round of theirs read 24 threads as worse
than 8 at load 25–33 — did not reproduce here at any load**, which is the reason the value is read
off the machine at run time instead of written into the source as a constant.

**Where the decision lives is the other half of it.** `render_quorra::options()` is the one place
this host's quorra options are chosen, and every constructor and caller spreads it;
`with_options` stays the escape hatch. No flag: there is nothing for a person to type, the one
caller that needs another number is a gate, and what would revive the flag is written down.

**Determinism, reproduced on this tree's own gate.** All four coverage lanes — `cpu` and `gpu`,
scale 1 and scale 4 — were run twice, at 24 threads and at 1, and their judged output compared
character by character: identical in all four, `REFUSED_AT_FOUR` unmoved, and every row also
identical to ADR 0367's at the previous pin. `PDFVIEWER_QUORRA_ENCODE_THREADS` is how the pair is
run again, and it does not turn the ratchets off.

**What it buys, and the ceiling that was stated before it was built.** `tmp/Entwurf.pdf` under
`Xvfb`/llvmpipe, ADR 0368's script, arms alternated A A B B A: the two magnification frames go from
608.2 and 514.6 ms to 295.0 and 274.7, and the return to the fit view from 937.8 to 314.1. The
frame's *structure* is unchanged — `host` 0.0, `scene` 14–22 ms, `settle` 1–2 ms, 40 uploads, the
same cull counts — and the whole difference is inside `device`. **The launch table did not move**:
`graphics device` reads +30.4 and +27.9 ms at one thread against +35.3, +22.8 and +30.9 at
twenty-four, which is upstream's "nothing is built at construction" checked from this side.

**Date.** 2026-08-15.
**ADR.** [0377](../adr/0377-the-geometry-phase-divides-and-the-number-is-the-machines.md).

**Code.** `crates/render-quorra/src/lib.rs` (`options()`, and the two headless constructors through
it), `crates/render-quorra/src/present.rs` (both surface constructors — the window path is the
whole point), `crates/render-quorra/examples/encode_threads.rs` (new: the instrument),
`crates/render-quorra/tests/corpus.rs` (`encode_threads()`, the announce line, the gate's own
options), `crates/render-quorra/tests/real_pages.rs` and three examples (spreading `options()`
rather than quorra's defaults), `Cargo.lock` (two hashes).

**Touched.** `doc/QUORRA_UPGRADE.md` (the release's own section: the range in three groups, the
ladder, the eight lane runs, the frame path), `doc/QUORRA_ENCODE_THREADS.md` (the ask recorded as
answered, built, taken and turned on — body unedited), `doc/QUORRA_FEEDBACK.md` §27 (what went
back: the ladder, the reproduction, the confined-worker correction, and the two censuses still
owed), `doc/todo/49-restrictions-worth-re-examining.md` (why this thread count was decided the
opposite way from the search's, and that it is not a contradiction),
`doc/adr/0377-*` (new), this file.

**Gates.** `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets` silent of
lints; `cargo nextest run --workspace` **1994 tests run: 1994 passed, 15 skipped**;
`cargo test --workspace --doc` green. Corpus: `974 documents in 5.4s: 0 unopenable, 8 locked,
2 encrypted beyond us, 6 pageless, 64 incomplete, 0 slow`. Oracle: `1794 pages in 129.7s (1691 we
call complete, 103 incomplete)` — `agrees 906, contradicted 67, ambiguous 786, our geometry 1,
reference geometry 2, not comparable 13, no render 19`. `text_extraction` 10969/11163 words in
bounds (98.26%) over 486 of 508 documents; `dates` 1514 of 1545 conform (97.99%); `xmp` and
`jpeg2000` green; `conformance` 5 passed. **The quorra gate was run eight times** — both
lanes, both scales, at the chosen thread count and at one: `931 / 23 / 2 / 18`, `929 / 25 / 2 / 18`,
`936 / 10 / 5 / 23`, `937 / 9 / 5 / 23`, each identical at one thread.
