# 523 — The width a file states is a statement about the shape

**Finding.** Table 109's `/Widths` row carries a sentence this tree had read as a rule about
*placement* only — "[t]hese widths shall be consistent with the actual widths given in the font
program" — and it is a statement about the **absent font's shapes** as well. So a substituted face
drawn at its own designer's width inside advances the file states for a condensed one contradicts
the file, and `bug1671312_ArialNarrow.pdf` is what that looks like: our letters collide where four
other renderers have gaps. `metrics::substitute_stretch` derives one horizontal scale per
substituted simple font — the median over the declared codes of the stated width over the chosen
face's own advance — and `build_outline` applies it to x alone. It condenses and never expands,
because §9.2.4 makes a width a *displacement*, which bounds ink from above and says nothing about
it from below.

**The clause came first and it requires nothing here.** Not §9.5's NOTE 5, not §9.6.2.2's "[t]hese
fonts, or their font metrics and suitable substitution fonts, shall be available to the PDF
processor", not §9.8.1, not one row of Table 120. This is decision (b) of the three the round was
given: a construction from entries the *file* states, documented as a choice. **And the pull
request the owner supplied is answered by the standard rather than adopted** — `mozilla/pdf.js#12725`
justifies overriding a standard font's widths by experiment against Acrobat, and Table 109's
`/FontDescriptor` row states it: "specifying them enables a standard font to be overridden".

**Date.** 2026-08-14.
**ADR.** [0358](../adr/0358-the-width-a-file-states-is-a-statement-about-the-shape.md).
**Touched.** `crates/pdf-font/src/metrics.rs` (`stated_widths`, `substitute_stretch`,
`simple_advances`, `program_widths` taking the glyph as well as the code, three tests),
`crates/pdf-font/src/loading.rs` (`LoadedFont::stretch`, the field, `PathPen`'s x, one corpus-wide
test), `crates/pdf-model/tests/substituted_shapes.rs` (new, two tests over the witness),
`crates/pdf-model/examples/substitute_stretch_census.rs` (new),
`doc/conformance/ledger.toml` (§9.6.2.1, §9.6.2.2, §9.8.1), `doc/todo/21`,
`doc/adr/0358-*` (new), this file.

## The witness, on session 518's four instruments

| | before | after | the four references |
|---|---|---|---|
| ink box | x[10, **149**] y[15, 34] | x[10, **147**] y[15, 34] | poppler, mupdf: x[10, 147] y[15, 34] |
| marked pixels in it | **983** | **861** | 844, 825, 812, 702 |
| page ink of 255 | **18.45** | **15.28** | 15.52, 15.32, 14.97, 12.71 |
| modal dark run at 576 dpi | **14 px** | **12 px** | poppler 12 px; `/StemV 66` is 10.56 |

**The stem is the check nothing fitted**: the scale is `/Widths`', and `/StemV` is a different
entry of a different table saying the same thing. And the page was looked at — the `cc` and `ss` of
*Accessory* are one blot before and two letters after.

## What moved on the corpus

30 of the 974's 257 substituted first-page fonts are drawn narrower, 15 of them by more than half a
per cent, over 22 and 11 documents; the other 227 are metric-compatible with the face standing in
for them and do not move at all.
`cargo run --release -p pdf-model --example substitute_stretch_census -- <files>` prints it.

**No verdict moved and seven of 888 per-page lines did.** Five improve — the witness (mean 11.83 →
8.05, ssim 0.7968 → 0.8566), `non-embedded-NuptialScript.pdf` (17.02 → 11.47, 0.6524 → 0.7640),
`issue13916.pdf` (12.16 → 11.36), `XiaoBiaoSong.pdf` (6.63 → 6.26), `issue12295.pdf` (5.55 → 5.54)
— and two move in the fourth decimal, which is the 0.03% condensation the census lists for
`bug847420.pdf` and `issue7580.pdf`. `doc/todo/00` step 7 over all 786: exactly two rows move, both
this round's own and both downward — `issue13916.pdf` −6.980 → −7.368 and `issue12295.pdf` −2.823 →
−2.829 — with twenty at or past −1 and sixteen of them documents this tree calls incomplete, the
head unchanged to the thousandth.

**The asymmetry was measured before it was written down.** With expansion allowed, eleven pages
moved and the three extra were the only ones that got worse: `issue9291.pdf` at 1.0665,
`issue7835.pdf` at 1.1978, `issue7454.pdf`. The argument stands on §9.2.4 without them.

## Gates, verbatim

```text
cargo fmt --all --check                                   clean
cargo clippy --workspace --all-targets                    silent
cargo nextest run --workspace                             1899 tests run: 1899 passed (1 slow), 15 skipped
cargo test --workspace --doc                              1 passed, 0 failed
corpus    974 documents in 11.5s: 0 unopenable, 8 locked, 2 encrypted beyond us,
          6 pageless, 61 incomplete, 0 slow
          codes reaching no glyph in silence 5/2; reaching a blank glyph 57/9;
          §9.10.2 could not name 1228/43
oracle    1794 pages in 67.0s (1694 complete, 100 incomplete)
          agrees 906/863   contradicted 67/66   ambiguous 786/755
          our geometry 1/0   reference geometry 2/2   not comparable 13/8   no render 19/0
text      974 documents in 35.0s: 25 skipped, 58 incomplete and not gated;
          overall 99.3% (24016/24195 words), 22 below 90%
          10969/11163 word boxes in bounds (98.26%), 486 of 508 documents fully in bounds
          PDFBox: doc/corpora/pdfbox is not checked out — skipped, as §2 says it may be
dates / xmp / jpeg2000 / conformance                      ok
quorra    956 pages in 126.2s: 934 agree, 20 differ, 2 refused, 18 not comparable
quorra    gpu lane at 4×: 951 pages, 937 agree, 9 differ, 5 refused, 23 not comparable
```

**What it costs at load, measured**: `callgrind_interpret` on the witness, 29 495 778 instructions
before and 29 692 285 after — 196 507, or 0.67%, for one substituted font with 224 declared codes.
The corpus gate's wall clock is inside its own noise across four runs (6.6 and 7.4 before, 6.9 and
7.1 after, on a machine also building).

**Not done, and why.** `doc/todo/02` §5's release binaries were not rebuilt: this round is a
worktree that has not been merged, and installing its binaries into the main tree's `target/`
would put unmerged rendering in front of a person. The merge round owns that section for the
merged result, as §2 says it owns the gates.

## Two things worth keeping

**An estimator that is better on one page and worse on another is not a finding.** A mode over the
ratios answers `issue20489.pdf` better than the median does (0.928 against 0.688, where a third of
that file's `/Widths` array is filler) and `non-embedded-NuptialScript.pdf` worse (0.666 against
0.799, visibly too thin). Both were implemented and measured; the simpler statistic stands and the
cost is written into `substitute_stretch`'s own documentation, because choosing on which page looks
nicer is the curve-fitting principle 5 forbids. What would settle it is a file, not a preference.

**`git stash` is not safe in this repository and this round lost work to it.** The stash stack is
shared by every worktree of one clone, so a parallel round's `git stash` can land between a
round's own push and pop — which happened here, and the pop applied another round's diff over
mine while mine was taken by theirs. Recovered from `git fsck --unreachable`, which is the only
reason it cost minutes rather than the round. **A before/after A/B takes `git diff > patch`,
`git checkout --`, and `git apply` — never `git stash`.**
