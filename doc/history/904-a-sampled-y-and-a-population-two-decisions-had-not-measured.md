# 904 — A sampled `Y` for a table profile's mask, and the population two decisions had not measured

Date: 2026-09-03.
ADRs: [0850](../adr/0850-the-clause-that-decided-half-of-the-route-into-grey.md),
[0851](../adr/0851-a-sampled-y-for-a-table-profiles-mask-and-a-population-two-decisions-had-not-measured.md).
Touched: `crates/pdf-render/src/soft_mask.rs`, `crates/pdf-model/src/colour.rs`,
`crates/pdf-model/src/soft_mask.rs`, `crates/pdf-model/src/content/ext_gstate.rs`,
`crates/pdf-model/tests/transparency_groups.rs`,
`crates/pdf-model/examples/luminosity_mask_census.rs`,
`crates/viewer-confined/src/protocol/display_list.rs`, `crates/render-quorra/src/scene.rs`,
`doc/conformance/ledger.toml` (§11.5.3, §11.3.4), `doc/checks/fixed-documents.toml`,
`doc/todo/23-transparency-departures.md`, `doc/state-of-play.md`, `doc/QUORRA_FEEDBACK.md` §43,
two ADRs, this file. **No page of either corpus changes what it draws.**

## The measurement came first, and it inverted the two items

Round 879 left §11.5.3 `partial` on two shapes and this round was told to measure before deciding
either. `examples/luminosity_mask_census` was extended to name the four-component shapes the way
ADR 0797 had named the three-component ones, and run over `doc/pdf.js/test/pdfs` (964 of 974 open)
and over all 145 archives of `CC-MAIN-2021-31` one at a time under
`tools/bounded.sh --tree 12 --data 12` (65 720 of 65 944 open, 3343 with a `/Luminosity` mask, 77 s
warm, peak 0.29 GiB):

| mask group `/CS` | `doc/pdf.js` | the crawl |
|---|---|---|
| `/DeviceGray` | 37 | 35 141 |
| three-component `ICCBased`, matrix profile | 3 | 28 972 |
| `/DeviceCMYK` | 39 | 21 834 |
| `/DeviceRGB` | 2 | 6 597 |
| **four-component `ICCBased`, bi-directional** | **0** | **3 417**, in 181 documents |
| one-component `ICCBased` | 8 | 228 |
| `CalRGB` | 1 | 0 |
| **three-component `ICCBased`, table profile** | **0** | **0** |

So the first item — a three-component table profile as a mask group's `/CS` — has **no member
anywhere**, and the second — a four-component profile — has **3417 groups in 181 documents**, every
one of them a profile this crate reads and every one bi-directional. ADR 0797 and `doc/todo/23`
both recorded the second as having "no corpus member"; that was true of `doc/pdf.js` and neither
said so, which is the `undenominated` sweep's own defect in two decisions and a todo file.

## Item 1: drawn, because what stood in its place was a silence rather than a choice

§11.5.3 branches on the *kind* of the space — "For CIE-based spaces, convert to the CIE 1931 XYZ
space and use the Y component as the luminosity" — and EXAMPLE 1 adds that "[a]n analogous
computation applies to other CIE-based colour spaces". An `ICCBased` space is CIE-based (§8.6.5.1)
whether its profile carries a matrix and three curves or a lookup table, so there was never a
choice between two readings here: the device branch was the wrong branch, and it was taken with no
report at all. `pdf_render::Luminance` is now two shapes behind one type — three 256-sample curves
summed, or `side³` samples of `Y` interpolated trilinearly — and `RgbRoute::luminance` samples the
profile's own `A2B` at 33 points an axis, the same grid `profile_stages` already builds for the
conversion *out* of that profile, without §8.6.5.9's compensation as the separable shapes are.
`RgbRoute::luminance_is_separable` is gone: every three-component route states a `Y` now.

**`soft_mask::entry` takes the interpretation's `Presses`**, which is not a micro-optimisation: a
table profile's route is 36 000 profile evaluations for the cube and as many again for the `Y`, and
a page can name one mask per `gs` — `6081357.pdf` names 912.

## Item 2: named, with the construction it needs written down

A four-component profile mask group's `Y` is a function of four composited components, §11.4.7's
construction for four components is a **pair** of rasters, and `pdf_render::SoftMask` carries one
group and derives one value per pixel. Taking the `Y` of the *resolved device colour* instead was
considered and rejected in ADR 0851: that colour is eight bits and clamped to sRGB's gamut, so a
saturated press colour's `Y` would be wrong by whatever the clamp removed, which §11.5.3 does not
admit. So it is reported by name, and `doc/todo/23` carries the five pieces the next round needs.

**The report's condition is the clause's own branch and not a list of space names** (trap 11):
`luminosity_departure` fires on a CIE-based `/CS` for which the colorimetric route was not taken,
whatever the reason. That turned two further silences into reports at no cost — a profile with no
"from CIE" half, which §11.3.4 requires of a blending space, and a one-component curve with no
inverse — and it means a space that stops having a route stops being silent by construction.

## Item 3: §11.5.3 has already decided half of it

`doc/todo/23` framed ADR 0790's route-into-grey as one decision over two places, because "a mask and
a blending space are one sentence of §11.6.6 and may not take two conversions". §11.5.3 is a second
sentence: its device branch converts the composited colour to `DeviceGray` "with no compensation for
gamma or other colour calibration", and §10.3's route — the colour's sRGB taken to linear light and
its `Y` read — *is* a gamma compensation. So for a `/Luminosity` mask group in a device space the
classic weights are what the clause asks for, and `mupdf` and `ghostscript` depart from it there.
Agreement is evidence; this is a clause read.

The other half — §11.6.6's conversion into a `DeviceGray` or `CalGray` blending space for a page or
an isolated group — is still open, and the ranking that would settle it points less firmly than it
looks: §10.3's own subject is "CIE-Based colour to device colour" and this conversion has a device
source, §10.3.2 conditions its remapping on device spaces that "do not match that of the raster
output device" while this processor's `DeviceRGB` is that device's sRGB, and the only grey round
trip the standard writes out (§11.3.5.3's, for the non-separable modes) is §10.4.2.2's pair, under
which grey to RGB and back is the identity. Not moved here; halved, and named.

## Item 4: three of the six things the two rows fire on are not debts

`doc/todo/23` gains a section that answers this clause by clause. A `Lab` blending space (§11.3.4
forbids it outright), a one-component special space (§11.6.6 excludes `Indexed`, `Separation`,
`DeviceN` and `Pattern` by name), and a profile with no PCS-to-device half (§11.3.4 requires one)
are conditions the **document** fails; each stays reported for good, and none is work anybody owes.
What is owed is the four-component mask and the group-scoped conversion between two spaces at a
`Do`.

## Witnesses

No page of `doc/pdf.js` states either shape, which is what the census says and what the three raster
gates then confirm: corpus **64 incomplete**, oracle **61 contradicted, 47 not comparable**, quorra
**929 agree, 22 differ, 7 refused** — round 879's figures exactly, with the ambiguous bucket one
larger from the twenty-four rounds in between.

**The witness for the new report is a crawl document and the `fixed_documents` gate found it**,
which is that gate doing precisely the job ADR 0458 built it for.
`corpus-cache/safedocs/cc-main-2021-31/4359/4359750.pdf` p1 — seeded in session 631 for a `/Lab`
`DCTDecode` image — failed on `unexpected report ... four-component ICCBased /CS`, **with its ink
band `39.547 .. 41.547` untouched**. That pairing is the evidence that this round added a report and
moved no pixel; the row now lists the substring and says why.

`doc/todo/00` step 7 was re-run over the oracle's own artefacts (835 ambiguous pages, 772 with our
raster and a non-blank reference, 13 s). The head is the standing set — `issue12418_reduced.pdf`
at −19.45, `issue4722.pdf` at −13.81, `issue15977_reduced.pdf` at −12.93, all of them pages we draw
blank and the corpus gate already reports — and the tail is `bug1552113.pdf` at +47.81. Nothing in
either end is this round's; nothing could be, with the population at zero.

## Gates

The whole of `doc/todo/02` §2 in the worktree, in order, each walking line under `tools/bounded.sh`
(`--tree 8` for a build, `--data 12 --tree 12` for a walk) one at a time after checking `ps` for a
neighbour's gate binary. Every line exit 0 on its last run:
`Summary [69.361s] 3176 tests run: 3176 passed (1 slow), 26 skipped`; corpus **974 documents in
18.3s — 0 unopenable, 9 locked, 1 encrypted beyond us, 5 pageless, 64 incomplete, 0 slow**; oracle
**1945 pages in 96.7s (1841 complete, 104 incomplete)**, 61 contradicted, 836 ambiguous, 47 not
comparable, with `our_rendering_agrees_with_the_reference_consensus_across_the_corpus ... ok`; text
extraction **11 094/11 131 matched words in bounds (99.67%), 493 of 503 documents fully in**;
selection census **1000/1011 words (98.91%) over 453 documents**; accessibility census green over
**102 853 elements**, 57 116 a caret can move through; dates **1514 of 1545 (97.99%)**; XMP **318 of
319 read**; JPEG 2000 green; quorra **958 pages compared in 30.2s: 929 agree, 22 differ, 7 refused,
16 not comparable**; fixed documents **69 checked, 0 absent, 69 rows**; the transform gate **148.4
pages/s over a floor of 40**; the four transform walks and the foreign readback green; conformance
**875 subclauses, 13 756 citations, 1237 quotations verbatim**.

Two lines failed once each and both were the round's own doing rather than a regression. `clippy`
under `-D warnings` found an `unnecessary_to_owned` in the new fixture. `fixed_documents` failed on
`4359750.pdf`, above. **And one failure was neither**: `viewer-confined`'s
`the_two_deferred_producers_reach_the_raster_arm_by_name` failed on `issue19517.pdf` before
`cargo build --profile gates -p pdf-sandbox --bins` had ever run in this worktree — trap 10 in a
fresh checkout, where a JPEG 2000 scan that cannot be decoded encodes as an 82-byte list instead of
reaching the raster arm.

`--bin undenominated` was run because this round widened a population claim; its one hit in §11.5.3's
row is a pre-existing sentence about the blend-mode residue that the same note denominates two
sentences later. §5's binaries are not owed (`tools/round.sh`: not a fifth round, and the round's
own measurement was a census built in this worktree, not a launch number).

## What is left

- **§11.5.3 stays `partial` on one shape**: a four-component `ICCBased` mask group, 3417 groups in
  181 crawl documents, now loud. `doc/todo/23` has the five pieces — a second command list under
  `Compositing::Subtractive(Half::Black, press)` and its backdrop, a four-axis `Luminance`, a second
  buffer in `render-cpu`'s `build_soft_mask`, a second scene in `render-gpu`'s `evaluate`, and the
  protocol's second list. `render-quorra` needs nothing; it refuses every mask carrying a
  `Luminance` already.
- **§11.3.4 stays `partial` on half of what it did**: §11.6.6's conversion into a grey blending
  space. The mask half is decided by §11.5.3 and is not a choice any more.
- **`doc/QUORRA_FEEDBACK.md` §43's mask ask is wider**: the field beside `MaskKind::Luminosity`'s
  backdrop wants curves *or* a grid, which is the same "curves on either side of an N-axis grid"
  vocabulary the group ask in that section already wants.
