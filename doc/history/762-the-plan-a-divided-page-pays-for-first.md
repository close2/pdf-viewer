# 762 — The plan a divided page pays for before it draws anything

General-improvement round, chosen rather than assigned. Subject: **the *composition* of page 101's
rasterisation**, whose total this project has re-taken seven times and whose breakdown was last
taken in the hundred-and-sixty-third session. Taken: **−7.08% of the page, byte-identically**, and
the serial prologue in front of the divided render down **4.8×**.

ADR **0687**. 0688 and 0689 unused.

## Why this subject

The briefing's rule — find a number this project wrote down and has not re-run, prefer a
composition to a total, and prefer one that costs nothing to set up. Three siblings were on the
errata-ranked ledger rows, the oracle's *we are alone* list and the native hosts' launch path, so
those were out.

`doc/performance.md` carries two long-lived compositions. `callgrind_interpret`'s is the older
(session 58) but its subject has been rebuilt repeatedly since. `callgrind_rasterise`'s is the
*larger* number by four times, is the biggest standing figure in the tree, and its total has been
re-taken in sessions 162, 163, 175, 185, 195 and again in ADR 0677's table four rounds ago — while
its breakdown has not moved since the hundred-and-sixty-third. That is 757's rule in its sharpest
form, and the instrument needed no edit: one `valgrind`, one `callgrind_annotate`.

The first run said two of the three items the old composition names are gone and the third is
unchanged, and that a function no document in this tree had ever named was third in the profile —
`pdf_render::strips::segments`, 373.7 M self, **448.3 M inclusive, 8.28% of the page**.

## What was found

`unsplittable_rows` names the rows a horizontal cut may not fall on (ADRs 0138, 0139) by walking
every fill's, every clip's and every soft mask's path: **76 991 `mark` calls per render** on page
101. `mark` only ever *sets* a row, and a dense text page is thousands of glyph fills over a few
hundred rows — so nine of those ten marks changed nothing. A path whose whole device extent lies in
rows already forbidden is skipped, exactly rather than conservatively, because `Path::bounds` is the
control hull and `oblique_spans` reports y ranges of those same control points.

**Nine per cent of the total understated it.** The prologue is serial and the 4.9 G under it is not.
`examples/strip_spans` says this page is granted eleven strips whose slowest holds 10.6% of the
estimated cost, so the drawing's contribution to the critical path was about 24.7 M — which was the
planner's own figure. On this page the plan cost as much as the drawing did.

## What moved

Both arms in one sitting; the before arm re-derived on this tree rather than quoted, and run three
times to the after arm's two because a parallel render carries rayon's scheduling into its
instruction count (repeats differ by up to 0.3%).

| | before | after | |
|---|---:|---:|---|
| ISO 32000-2 p. 101 ×20 | 5 412 167 781 / 5 428 689 008 / 5 426 590 074 | 5 046 315 493 / 5 030 698 863 | **−7.08%** |
| `tracemonkey.pdf` p. 1 ×10 | 2 370 062 770 | 2 232 441 225 | **−5.81%** |
| ISO 32000-2 p. 6 ×20 | 3 641 516 664 | 3 525 317 024 | **−3.19%** |
| `bug1721218_reduced.pdf` p. 1 ×2 | 3 661 400 384 | 3 622 806 372 | −1.05% |
| `issue12841_reduced.pdf` p. 1 ×5 | 8 717 134 836 | 8 698 566 957 | −0.21% |

Every page falls, none rises, and the ink sum the example prints is identical in every pair.
`strips::segments` goes 448 348 680 → 56 526 340 inclusive and the whole prologue 494 039 652 →
102 319 839.

## Trap 13, and why the raster proves nothing

The strips are exact by construction, so a planner that forbade *too few* cuts would still draw the
identical picture here — byte-identity of the raster cannot calibrate this change at all. A
temporary switch and a temporary example compared the guarded row vector against the unguarded one:
**0 disagreements over 3412 page-scales of `doc/` and the pdf.js corpus**, and **0 over about
18 000 crawled documents and about 119 000 page-scales** before the run was stopped for the gates.
Three planted defects were caught first — `any` for `all` (1833 page-scales), the range shrunk by a
row at each end (928), the transform forgotten (1176). Neither the switch nor the example is
committed; the discriminating case is, as a unit test that fails at row 20 against the planted
`any`.

**Two process notes from the differential.** `xargs` under the harness ended up running twice over
the same list, which is why the crawl's distinct population is half the log's line count; and
`pgrep -f tmp_settled_diff` matched *this round's own shell*, which is `doc/environment.md`'s
process-table warning arriving through a pattern that was not a path.

## Gates and load

The full §2 sequence, `pdf-render` being in the everything row: fmt clean, `RUSTFLAGS="-D warnings"
clippy --workspace --all-targets` clean (only the known cold-build gcc lines from `viewer-qt`'s
generated bridge), 2678 tests, doctests, `fuzz/ --bins` checks, corpus, oracle, text extraction ×3,
both censuses, dates, xmp, JPEG 2000, quorra, fixed documents (40 checked, 0 absent), conformance
192 — all green, no ratchet moved. `PDFREF_CACHE` was set to
`/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache` throughout, so nothing was rebuilt beside the
shared copy; the text and selection lines report 958 and 954 cache hits and 0 misses.

The machine carried other rounds; load averages ran between 3 and 22 over the session. Nothing here
is a wall clock — every figure is a callgrind instruction count, and the one latency claim is
arithmetic over two counted numbers rather than a measurement of a clock.

§5's binaries were **not** rebuilt: this is not a fifth round, and the measurement is callgrind over
a purpose-built `--release` example, which names none of §5's six binaries. `tools/round.sh` reports
`target/` empty in this worktree, which is the worktree's state rather than this round's.

## Files

`crates/pdf-render/src/strips.rs`, `doc/conformance/ledger.toml` (§10.7.4 — its code list did not
name the strip planner), `doc/performance.md` (the spent composition marked as spent, and the new
finding beside the planner's other half), `doc/habits.md` (one measuring habit: a share of a total
is not a share of a critical path), `doc/adr/0687-…`.
