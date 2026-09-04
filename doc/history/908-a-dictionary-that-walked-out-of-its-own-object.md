# 908 — A dictionary that walked out of its own object, and a ranking whose light tail was its own instrument

Date: 2026-09-04.
ADRs: [0858](../adr/0858-a-dictionary-body-that-walked-through-endobj-and-took-the-next-objects-entries.md),
[0859](../adr/0859-batch5-cairo-and-a-ranking-whose-light-tail-was-its-own-instrument.md).
Touched: `crates/pdf-syntax/src/parser.rs`, `crates/pdf-syntax/tests/robustness.rs`,
`doc/conformance/ledger.toml` (§7.3.7, §7.3.10), `doc/checks/fixed-documents.toml`,
`doc/oracle-and-corpus.md` §3d, `doc/traps/instruments-and-reports.md` trap 10,
`doc/todo/03-more-corpora.md` §47, two ADRs, this file; and the merge commit before this round's
own. **No page of `doc/pdf.js` changes what it draws**, which is why every raster gate's figures
are round 904's exactly.

## The merge

**`round-904` (082fbbc3) is on `main` as `b3cf561a`**, `--no-ff`, on top of round 905. It is round
904: a three-component table profile's `/Luminosity` mask group drawn through the profile's own
`A2B` rather than down the device branch, a four-component mask group refused **by name on the
clause's own branch** instead of by a list of space names — which turned two further silences into
reports at no cost — and the correction of a claim that had stood in ADRs 0790 and 0797 and in
`doc/todo/23` that such masks have no corpus member. The real population is **3417 groups in 181
documents of `CC-MAIN-2021-31`**, and the claim was true of `doc/pdf.js` alone, which is the
`undenominated` sweep's own defect standing in two decisions and a planning file.

**Git found no conflict at all, in any of the three files where one was expected**, and the reason
is worth the sentence rather than the relief. `doc/todo/23-transparency-departures.md`,
`doc/checks/fixed-documents.toml` and `doc/state-of-play.md` conflicted with nothing because
`main` has not touched one of them since the branch left — each resolves to the only side that
moved it, which was checked by hashing all three files on `base`, on `main`, on the branch and on
the merged tree rather than by reading a diff. `doc/conformance/ledger.toml` auto-merged: `main`'s
six changed rows sit in clause 7 and the branch's two (§11.3.4, §11.5.3) in clause 11, hundreds of
lines apart, and **each of the eight was compared against the side that wrote it**, byte for byte,
because this file is one table per clause and a per-line diff is the wrong instrument for it.

**`r904` stays open**: round 907 branched from it and is building the four-component construction
`doc/todo/23` now names piece by piece.

**The whole `doc/todo/02` §2 sequence ran on the merged result before this round wrote a line** —
every one of its twenty-six lines exit 0, the walking lines under `tools/bounded.sh` one at a
time. Its figures agreed with round 904's everywhere. The figures recorded below are the *final*
run's, on §2's own rule that a number belongs to the round that ran the gate last.

## The walk: `batch5/cairo`

Surveyed whole under the four rules of 2026-09-02 — twelve rayon threads, `--data 8 --tree 12`,
0.5 s, 0.04 GiB peak: **166 documents, 0 unopenable, 0 locked, 0 encrypted beyond us, 2 pageless,
18 incomplete, 0 slow**. 10.84% incomplete is the highest rate of any tracker walked so far, and
the shape is the tracker's rather than the world's — a cairo issue attachment is a file somebody
filed *because* a program choked on it.

**The ranking's first run was wrong at the light end and it was the instrument.** Five rows came
back at −8.7 to −22.8 levels, every one a page this tree drew blank while both references drew
ink, and all five were JBIG2. `examples/render_at` runs from `<target>/release/examples/`,
`pdf_sandbox::worker_program` searches beside the running executable **first**, and that directory
held a `pdf-sandbox-worker` some earlier round had copied there — ten hours behind this tree, and
no `cargo build` line and no §5 invocation touches it. `open_one` says so in one sentence, naming
the stale path and both build hashes, which is trap 5 earning its keep: a decoder that had failed
*quietly* would have made five findings out of nothing. Refreshed, all five agree with both
references to within 0.13. That is trap 10's third copy and `doc/oracle-and-corpus.md` §3d's
fourth way to be wrong with this ranking.

## The head, and what it was

`cairo-85141-0.zip-3.pdf` page 1: ours **4.6304** against `poppler` 1.7573 and `mutool` 1.6622,
2.87 outside the interval where the next row in either direction is 0.50, reporting nothing at
all. The ladder does not converge (4.630 / 4.588 / 4.586 / 4.645 at 72, 144, 288 and 576 dpi), and
the page shows why: this tree draws a paragraph and a ten-item numbered list that neither reference
draws.

**§7.3.7's `read_dictionary_body` was walking out of its own object.** Its last arm skipped every
token that is not a name where a key belongs, on a comment about "a stray value between entries" —
so on object 76, a Type 3 `/CharProcs` whose bytes stop at `/a112 57` under another stream's data,
it skipped the damage, then `endstream`, then `endobj`, then `78 0 obj <<`, took object 78's two
entries, met **its** `>>`, and returned `Ok`. `Document::get(76)` answered a **stream** made of
object 76's forty surviving entries, object 78's `/Length` and `/Filter`, and object 78's data.
Forty glyph procedures drew out of an object no producer wrote and `interpret` said
`unsupported []`.

That is trap 28 exactly — a recovery's guard is a claim and the comment above it is a different
claim — and it is the one outcome `parse_dictionary_body`'s own doc comment forbids in writing,
stated at the top of the function and broken twelve lines below it. §7.3.10 gives `obj` and
`endobj` their meaning and §7.3.8.1 gives `stream` and `endstream` theirs, so none of the four can
stand where a key belongs; the body stops at them now. The page draws **1.70076**, inside both
references, and reports `font /F16 is a Type 3 font with no /CharProcs dictionary`.

## The population, measured over two corpora and twice each

`tools/safedocs survey --dir` with the binary built before the change and with the one built after,
each with its own `pdf-sandbox-worker` beside it, verdict lines diffed:

| corpus | documents | verdicts that change |
|---|---|---|
| `CC-MAIN-2021-31` | 65 944 | **0** |
| `corpus-cache/tika-issue-tracker` + `doc/pdf.js` + the four `doc/corpora/` submodules | 24 324 | **8** |

Six of the eight gain or sharpen a report and two lose one they should never have had; ADR 0858
lists them. The sharpest is `batch1/PDFBOX/PDFBOX-4351-0.pdf`, refused for its whole life with
*unsupported encryption: /Filter /FlateDecode is not the standard security handler (§7.6.4)* — a
security handler no file has ever stated, read out of a stream's `/Filter` two objects further
down.

**A crawl of the open web states this nowhere and an issue tracker states it eight times.** That is
`doc/todo/03` §1's argument about which corpus is worth a round arriving from a direction it had
not arrived from: not *diagnostic versus large* but **what a file had to survive to be in the
corpus at all**.

## Held, each with the ladder run first

`cairo-48349-6.pdf` at −3.71 **converges** — the references fall to 12.56 and 12.31 at 576 dpi
against our flat 11.86 — so it is §10.7.4's anti-aliasing departure read from the light side.
`cairo-55799-0.pdf` at −2.52 and `cairo-54950-0.pdf` at −0.91 converge to within 0.16 and 0.15.
`cairo-31878-2.pdf` at −1.76 does **not** converge and is `doc/todo/21`'s standing population: a
code the font has no glyph for, where `poppler` draws a hollow box and this tree draws nothing.
And §29's claim about pageless files needs a qualification here — `cairo-101530-0.pdf` and
`cairo-101531-0.pdf` open onto no `/Root` while `pdfinfo` reports 5 and 1 pages, but `pdftoppm`
and `mutool draw` produce no raster for either, so the disagreement is about what a reconstruction
may claim rather than about a page.

## Gates

The whole of `doc/todo/02` §2 on `main`, twice — once on the merge and once on the finished tree —
each walking line under `tools/bounded.sh` (`--tree 8` for a build, `--data 12 --tree 12` for a
walk), one at a time, after checking `ps` for a neighbour's gate binary. Every line exit 0 on its
last run: `Summary [69.110s] 3191 tests run: 3191 passed (1 slow), 27 skipped`; corpus **974
documents in 10.5s — 0 unopenable, 9 locked, 1 encrypted beyond us, 5 pageless, 64 incomplete, 0
slow**; oracle **61 contradicted, 836 ambiguous, 47 not comparable**, with
`our_rendering_agrees_with_the_reference_consensus_across_the_corpus ... ok`; text extraction
**11 094/11 131 matched words in bounds (99.67%), 493 of 503 documents fully in**; selection census
**1000/1011 words (98.91%) over 453 documents**; accessibility census green over **102 853
elements**, 57 116 a caret can move through; dates **1514 of 1545 (97.99%)**; XMP **318 of 319
read**; JPEG 2000 green; quorra **958 pages compared in 31.2s: 929 agree, 22 differ, 7 refused, 16
not comparable**; fixed documents **70 checked, 0 absent, 70 rows**; the transform gate **169.5
pages/s over a floor of 40**; the five transform walks and the foreign readback green; conformance
**875 subclauses, 13 914 citations, 1243 quotations verbatim**.

**Three lines failed once each and all three were this round's own doing.** `cargo fmt` on the new
test. `cargo test -p conformance` on the new test's blockquote, which paraphrased §7.3.10 while
claiming to quote it — the sentence is "followed by the value of the object bracketed between the
keywords obj and endobj", and the quotation gate caught the invention in the tree, in the ledger
and in the ADR at once. And `fixed_documents` on `cairo-85141-3.pdf`, a row session 864 seeded,
whose `/F1` report changed from `is a Type 3 font with no /FontMatrix` to §7.3.10's own sentence
about a reference that resolves to nothing: the `/FontMatrix` it used to be missing was a claim
about an object assembled out of two. The row moved and says why.

**One line failed for a reason that was not this round's**, and it is the shape `doc/todo/02` §2
warns about: `pdf-model::outlines::an_outline_resolves_against_the_page_tree_once` is a *ratio of
wall clocks* and it failed at a load average of 38 with a 12-thread crawl survey of this round's
and two neighbours' gate runs on the machine. It passes quiet, in the workspace run above.

`doc/todo/00` step 7 was re-run over the oracle's own artefacts (835 ambiguous pages, 772 with our
raster and a live reference, 82 s). The head is the standing set — `issue12418_reduced.pdf` at
−19.447, `issue4722.pdf` at −13.810, `issue15977_reduced.pdf` at −12.927, all of them pages we draw
blank and the corpus gate already reports — and nothing in either end is this round's, which is
what a population of zero `doc/pdf.js` documents predicts. `--bin undenominated` was run because
this round wrote counts over two corpora; neither ADR nor §47 is a hit.

§5's binaries are not owed: `tools/round.sh` says this is not a fifth round, and this round's
measurements are a survey and a ranking built in this tree rather than a launch number.

## What is left

- **A consumer for `Document::damaged_dictionary` beyond `Pages`.** `cairo-85141-0.zip-3.pdf`'s
  forty glyph procedures are still there behind the door ADR 0784 built, and the prefix now stops
  at the object's own end. Drawing them **deliberately and with a report** is a different change
  from the splice this round removed, and it is the one this file recommends next for §7.3.7.
- **Two reconstruction cases**, `cairo-101530-0.pdf` and `cairo-101531-0.pdf`, where poppler's
  rebuild reaches a catalogue and ours does not and neither draws.
- **`batch5`'s other seventeen trackers**, `pdfminer.six` (123) and `qpdf` (111) the largest of the
  remainder.
