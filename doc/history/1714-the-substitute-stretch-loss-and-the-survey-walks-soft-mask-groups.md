# 1714 — The loss `substitute-stretch`, one decision for every loss, and the survey walks soft masks

Slot 3 of batch one hundred and ten, 2026-10-23, an archive round. ADRs 2262, 2263; no question.
**Built.** (1) A566: `Loss::SubstituteStretch` on every route that embeds the shipped face for a
font the viewer drew fitted (ADR 0358) — the described route, the restatement route, and the
unrestated route into a file's descriptor, which embedded over the fit as a statement before (a
fixture proves it). The proof stays exact (ADR 2262). `Decision` now carries `Losses`, a set: the
font row names every loss its fonts incur and proceeds only where all are authorised; report, JSON
and `xmpMM:History` name each font's losses and scale; a site `discard` authorises the row's whole
set. (2) ADR 2263: the survey walks a soft mask's `/G` from the `gs` that sets it, the cross-check
compares those selections, and the converter's default-space walk reaches the mask's group too.
**The eight, drawn** (page 1, 100 and 288 dpi): four convert — three PDF 2.0 samples at part 4
differ by at most 1 level, `me450f12project10_report` by 1 level in its body and up to 225 in a
heading drawn from a machine's Times (`machine-face`, the second loss of the same decision,
looked at); `427669` stops at the content fence; three now fail the second verdict on two font rows
(ADR 2262). veraPDF 1.30.3: the three part-4 outputs compliant; the 2b one fails 6.6.2.3.1 and
6.2.4.2 on an object-level packet and two source ICC profiles, which this validator passes.
**Census** (both profiles, by document, ADR 2262's table): `as-if-printed` refuses no new document
and converts 1 + 1 + 1 + 3 more; `as-shown` newly refuses 8 + 18 at 2b and 7 + 20 at 4, every one by
`substitute-stretch` withheld, scales 0.9928438–0.9999765 (`bug1671312_reduced`, 0.72 %, drawn).
**Hypotheses.** "The eight convert with both losses named": half held. "The widened walk reports
the twelve and refuses them at part 4": the twelve are reported; they were refused already, and the
converter-over-veraPDF counts are unchanged. "No document that converted before is refused": held
under `as-if-printed`; under `as-shown` 26 and 27 were, each a silent loss before. **Premise that
did not hold**: the fitting was refused everywhere — one route lost it unasked.
**Gates.** Worktree: `rustfmt --check` on every file of mine exit 0; `-D warnings` clippy
`--all-targets` on pdf-archive and pdf-transform exit 0. Export of this slot alone: `cargo fmt
--check` exit 0; nextest pdf-transform + pdf-archive 949 passed, 0 failed, 16 skipped. Cross-check
(`--tree 6`) exit 0, 13 560 compared, 0 disagreements; planted (the walk taken out) exit 101,
naming 13 places (trap 13). `tools/state.sh --round 1714 archive` exit 0 in both exports: validator
missed 0, over 0; converter over veraPDF unchanged, re-run after the rewriter change one test at a
time, exit 0. Real-corpora census alone, HEAD 159 s and this slot 125 s, exit 0. `cargo test -p
conformance` in the worktree: every test of these files passes (`evidence`, `adr_status` exit 0);
the run exits 101 on two siblings' files, `viewer-ffi`'s `abi/host.rs` and record 1717.
`t3-foreign_corpus` not run: its header reads back five writers and not `archive`, and no path it
runs reaches a changed file. Arms not compared: neither crate is in `render-raster`'s graph.
**Found, not fixed.** The three second-verdict font failures (a `Differences` name, a symbolic
descriptor's encoding); the portfolio's output fails veraPDF 6.8 on an embedded file; the
unrestated route's machine-face case (`doc/todo/72`). 6560 s.
