# 1848 — A thin rule under `/SA true` is fitted off by default, and the choice is put to the owner

Slot 9 of batch one hundred and twenty-five, 2026-10-28, a "measure" round (Opus), two sessions (the first handed
over at about 230k of context); ADRs 2530 and 2531 used, Q714 used; all numbers of mine used. I worked in
`scratchpad/r1848/` and in my own export built there with its sandbox worker (habit 185).
**Ledger:** §10.7.5 stays `departed` (the owner's answer to Q714 decides it); its note names the build, its default
and the figures, `code` gains `pdf-render/src/grid_fit.rs` and `test` two of its unit tests. `doc/todo/65` has no
§10.7.5 entry and no status moved, so it is untouched; `doc/todo/_scan-conversion.md` and `doc/todo/README.md` say
the fit is built and off by default.
**Built:** `pdf_render::grid_fit` (a stroke under `/SA true`, under 1.5 device pixels, under a similarity transform and
along the device axes, drawn one whole pixel wide across pixel centres; six unit tests), applied at `render-cpu`'s
stroker entry when `CpuRasterizer::with_stroke_adjustment(true)` asks — **off by default**, because the owner's
"10.7.5 let's measure it" makes the measurement the deliverable and the build-or-keep decision the owner's;
`stroke_width.rs`'s ladder has a fitted and an unfitted arm (a fit forced off fails both `/SA` tests, trap 13); the
instrument `pdf-model/examples/stroke_grid_fit_measure.rs`.
**Figures** (ADR 2530; page one of the pdf.js documents at 1×, one `--clock --nice 0` hold of 69 s, load 3.82, no
render-node holder): 16 pages stroke under `/SA true` and 7 draw differently; 778 `/SA` strokes, 450 fitted (441 of
them requested 0.5–1 px, drawn up to 0.5 px heavier, the clause's bound); 347 sampled segments, crisp 4 → 316; 2 seam
pixels opened over 123 shared stretches, 0 closed (a plant reads 80 of 80); 23 693 pixels changed; draw time 333.5 ms
unfitted against 330.9 ms fitted (the 16 pages' least of five, summed). The corpus gate built with the fit on, run on
the 7 pages against HEAD's six arms: raster's digests unchanged, `issue14297.pdf` turns from agreeing to differing at
1× on all three lanes (mean error 0.1233 → 1.6956 levels of 255 on the CPU lane), nothing moves at 4×.
**Decision** (ADR 2531, amending ADR 0848's measurement): the code stays, off by default, because the arms match
HEAD's and it is what either answer starts from; Q714 offers four options and recommends drawing it under `/SA true`
in every backend at once, after a `raster` round fits at the four `resolve_width` sites of `raster-gpu`'s encoder.
**Premise** ("a width and an offset rounded at the stroker's entry, seams where a fill shares the stroke's edge"):
held; the seams are where a rule meets a header fill's top edge, 2 pixels on one page.
**Gates.** In my export: `cargo nextest run -p pdf-render -p render-cpu` 558 passed, exit 0 (fit off by default);
`cargo clippy -D warnings` on render-cpu and pdf-render (all targets) and the example, exit 0; rustfmt `--check` on
my files, exit 0; rustdoc `-D warnings` `-p render-cpu` exit 0; `cargo test -p pdf-model --test raster_golden` 2
passed, exit 0; its ignored walk under the gates profile held 974, moved 0, exit 0 (`raster_golden.tsv` unedited); the
six corpus arms on my export, each exit 0, 968 pages at 1× and 965 at 4× with 0 frame digests and 0 mean errors
differing from HEAD's six arms; the six fit-on arms over the 7 pages, each exit 0 (figures above). In the worktree:
`cargo run -p conformance --bin cited` exit 0, `--bin quotations` exit 0; `cargo test -p conformance --no-fail-fast`
exit 101, one red not mine (`evidence`'s path rule on the 1843 record). Not run: the plain corpus gate with its
ratchets (the six arms match HEAD's by page, so it is HEAD's run) and the turn and launch holds (the default path adds
one boolean per stroke; fitted, the draw time did not rise).
