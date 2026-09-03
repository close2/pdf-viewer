# 907 — A pair of rasters inside a mask, a `Y` over four axes, and 41 pages that move away from every reference

Date: 2026-09-04.
ADRs: [0856](../adr/0856-the-colorimetric-branch-has-no-component-count-in-it.md),
[0857](../adr/0857-a-pair-of-rasters-inside-a-mask-and-a-y-over-four-axes.md).
Touched: `crates/pdf-render/src/soft_mask.rs`, `crates/pdf-render/src/repeat.rs`,
`crates/pdf-render/src/group_cost.rs`, `crates/pdf-render/src/lib.rs`,
`crates/pdf-model/src/colour.rs`, `crates/pdf-model/src/soft_mask.rs`,
`crates/pdf-model/src/content/transparency.rs`, `crates/pdf-model/src/content/image.rs`,
`crates/pdf-model/src/content/pattern.rs`, `crates/pdf-model/tests/transparency_groups.rs`,
`crates/pdf-model/examples/press_census.rs`,
`crates/pdf-model/examples/luminosity_mask_census.rs`, `crates/render-cpu/src/lib.rs` and three
of its tests, `crates/render-gpu/src/soft_mask.rs`, `crates/render-quorra/src/scene.rs`,
`crates/test-scenes/src/lib.rs`, `crates/viewer-confined/src/protocol/display_list.rs`,
`doc/conformance/ledger.toml` (§11.5.3), `doc/checks/fixed-documents.toml`,
`doc/todo/23-transparency-departures.md`, `doc/state-of-play.md`, `doc/QUORRA_FEEDBACK.md` §43,
two ADRs, this file. **41 pages of the crawl change what they draw; no page of either submodule
corpus does.**

## The reading, before any of it was built

Round 904 named the last shape §11.5.3 had left and measured it — a `/Luminosity` mask group
whose `/CS` is a four-component `ICCBased` space, **3417 groups in 181 crawl documents**, none in
`doc/pdf.js` — and wrote the five pieces it needed into `doc/todo/23`. All five were right. What
the reading added was a word.

§11.5.3's colorimetric branch has **no component count in it**: "For CIE-based spaces, convert to
the CIE 1931 XYZ space and use the Y component as the luminosity", with EXAMPLE 1 generalising its
`CalRGB` formula to "other CIE-based colour spaces"; §11.3.4 lists "ICCBased bi-directional
'GRAY', 'RGB ', and 'CMYK' colour spaces" in one sentence and §8.6.5.1 makes all three CIE-based.
What four components change is the *carrier*, and three carriers were priced (ADR 0856):

- **The pair of rasters.** Chosen. §11.3.4 composites per component, a rasteriser has three
  channels, so §11.4.7's construction — one content stream interpreted twice — is what a
  four-component space needs, here inside a mask.
- **Evaluating the luminosity during compositing.** Refused by the clause rather than by the
  price: §11.5.3 takes the luminosity of "the resulting colour", and the luminosity of a composite
  is not the composite of luminosities unless `Y` is affine in the components, which a press
  profile is not.
- **The `Y` of the resolved device colour.** ADR 0851's rejection re-derived, because it is the
  one that looks affordable: eight bits, clamped to sRGB's gamut, so a saturated press colour's
  `Y` would be wrong by whatever the clamp removed — and the error lands in an alpha, where
  nothing downstream can show that it happened.

**The word the list did not have is *unconditional*.** `group_commands` skips its own black half
where nothing inside the group composites, on the argument that a group's four components are
converted to the device at the end and an opaque `Normal` mark carries its colour through whatever
space it was carried in. That argument does not survive the move into a mask: a mask's four
components become **one number** that reads all four, and `0 0 0 1 k` paints nothing at all into
the chromatic raster while the clause still asks for the `Y` of full black.

## What was built

`pdf_render::SoftMask::black` is a `BlackHalf` — the second command list and the half of
§11.6.5.1's four-component `/BC` it composites onto. `Interpreter::mask_halves` takes the second
run under `Half::Black`, with `readback_mark`/`rewind_readback` around it and `paired` after it,
and gives the pair up on two conditions with a report and a re-run on the device: a group inside
that introduced a space of its own, and §11.7.5.3's black generation. `Luminance` grew a fourth
axis and `trilinear` became `multilinear` over `2^axes` corners. `Press::luminance` samples
`PRESS_SIDE⁴` values of the profile's own `A2B` at the very points `sample_press` samples for the
conversion out, without §8.6.5.9's compensation, behind a `OnceLock`.

`render-cpu` draws the black half into a second buffer over the same rows (`mask_values` is the
value pass, split out); `render-gpu` builds a second scene and calls the oracle's own
`SoftMask::paired_value` on the readback; `render-quorra` refuses by name, and
`doc/QUORRA_FEEDBACK.md` §43 now asks for a **second body** beside the curves-or-grid field it
already wanted. The confined protocol tags the four-axis grid apart from the three-axis one.

## What it cost

`examples/press_census` gained `--luminance` and was run over all 145 archives of
`CC-MAIN-2021-31`, which name **287 profile presses**. For each it builds the grid and compares
interpolating it against evaluating the profile over 20 000 ink quadruples:

| | median | p90 | worst |
|---|---|---|---|
| the sampled `Y`, in levels of 255 | **0.17** | **0.98** | **0.98** |
| the sampled *device colour* on the identical grid (ADR 0272) | 5.99 | 11.02 | 14.52 |

Under one level everywhere, against six for the colour — and for ADR 0272's own reason: that grid
is uniform in ink and read out through sRGB's transfer function, fifteen levels steep at black,
where a luminosity is smooth in linear light. **11.8 ms in the median to sample and 58.9 ms at
worst**, over 203 cold samplings, and 334 KB a press, once per press. It is behind a `OnceLock`
rather than inside `sample_press` because 187 of those presses are a page group's and 94 an output
intent's, and none of them carries a mask.

## The witness, and the thing this round has to say plainly

All 181 witnesses were rendered at scale 1.0 with the branch on and with it off, in one sitting.
**41 move**, and the three references were run over those 41:

| | before | after | |
|---|---|---|---|
| mean \|ours − poppler\|, 40 pages | 18.728 | 19.502 | **+0.774** |
| mean \|ours − mupdf\|, 41 pages | 17.195 | 18.038 | **+0.843** |
| mean \|ours − ghostscript\|, 17 pages | 14.367 | 15.837 | **+1.470** |

**38 of the 41 move away from the reference consensus and 3 toward it.** That is ADR 0797's
`issue21346.pdf` disagreement one component count over, and it is the same sentence of the same
clause: the clause's `Y` of an encoded mid-grey is its *linear* luminance, and every reference
takes the device branch. `4605565.pdf` p1 is pinned in `doc/checks/fixed-documents.toml` and shows
both directions for one arithmetic — its "Soup of the Day" panel goes *to* poppler's, mupdf's and
ghostscript's bright yellow, its salad panel's gradient fades further from all three — ink 57.235
before and 51.779 after, 241 642 pixels, worst channel 56.

Refusing four components while drawing one and three would be an accepted decision applied to two
of its three cases, so it is drawn and the number is stated here rather than discovered later. The
ranking is the owner's, and it is now one ranking for one, three and four components rather than
three.

Round 904's witness `4359750.pdf` p1 goes back to `reports = []` with its ink band untouched for
the third round running — the evidence that this page's mask is not what its picture rests on.

## Two measurement notes worth keeping

- **A stale `--release` example binary nearly produced a false negative.** The first before/after
  pass reported *zero* of 181 moved: `cargo build --example X` had been run for three other
  examples in between, so `render_at` on disk was still the branch-off binary. Trap 10b's shape,
  outside a module.
- **The null was run against the defect twice** (trap 13). Inverting the sampled `Y` moves 41
  pages by up to 252 of 255 — which proves the instrument sees these masks at all — and a bulge
  of at most 16 of 255 confined to the **middle** of the `Y`, zero at both ends, moves the same 41
  by up to 101, which is what rules out the only innocent reading of the first pass's zero: that
  the masks are binary and both routes agree at the ends.

## Gates

The whole of `doc/todo/02` §2, in the worktree, after merging `main` (round 905's two commits and
its merge of round 904), each walking line under `tools/bounded.sh` one at a time after checking
`ps` for a neighbour's gate binary. Every line exit 0 on its last run:
`Summary [71.151s] 3193 tests run: 3193 passed (1 slow), 27 skipped`; doctests 0 failed; corpus
**974 documents in 10.3s — 0 unopenable, 9 locked, 1 encrypted beyond us, 5 pageless, 64
incomplete, 0 slow**; oracle **1945 pages in 103.8s (1841 complete, 104 incomplete)**, 979 agrees,
**61 contradicted**, 836 ambiguous, 47 not comparable, with
`our_rendering_agrees_with_the_reference_consensus_across_the_corpus ... ok`; text extraction
**11 094/11 131 matched words in bounds (99.67%), 493 of 503 documents fully in**; selection census
**1000/1011 words (98.91%) over 453 documents**; accessibility census green over **102 853
elements**, 57 116 a caret can move through; dates **1514 of 1545 (97.99%)**; XMP **318 of 319
read**; JPEG 2000 green; quorra **958 pages compared in 88.4s: 929 agree, 22 differ, 7 refused, 16
not comparable**; fixed documents **70 checked, 0 absent, 70 rows**; the transform gate **90.2
pages/s over a floor of 40**; the six transform walks green over 974 documents each (`foreign` 203
of 974 at stride 8); conformance **875 subclauses, 13 996 citations, 1243 quotations verbatim**.
Round 904's raster figures are unmoved, which is what the census said in advance.

Four lines failed once each and all four were this round's own doing. `clippy` under
`-D warnings` found a `too_many_lines` and a `manual_midpoint` in the new `pdf-render` test and,
after the merge, a `doc_markdown` on a verbatim clause list in the new `pdf-model` test — the fix
was to make it the rustdoc blockquote `CLAUDE.md` asks for. The conformance gate found the other
two: a blockquote of §11.5.3's branch attributed to §8.6.5.1 because that was the nearest citation
above it, and a `§43` written after `doc/QUORRA_FEEDBACK.md` where a `§` means ISO 32000-2.

**One failure was not this round's and is worth recording**: `cargo clippy --manifest-path
fuzz/Cargo.toml` would not compile at all, on `tinyvec 1.13.0` — an upstream `cannot find macro
vec` — which the *gitignored* `fuzz/Cargo.lock` had resolved to while the workspace pins 1.12.0.
`cargo update --manifest-path fuzz/Cargo.toml -p tinyvec --precise 1.12.0` fixes it and touches no
tracked file.

`--bin undenominated` was run because this round writes population claims; its one hit in
§11.5.3's row was an "no corpus member" this round had written, and the sentence now carries the
census's own two figures. §5's binaries are not owed (`tools/round.sh`: not a fifth round, and
this round's measurements are censuses and renders built in this worktree rather than launch
numbers).

## What is left

- **§11.5.3 stays `partial` on one thing that is work and three that are not.** The work is
  §11.3.5.2's residue — a non-`Normal` blend mode inside a subtractive group of more than one
  component, 0 of 1126 curated documents and 0 of 65 703 crawl documents — for which the pair
  built here is now the carrier, since a `DeviceCMYK` mask group could composite four components
  and apply §10.4.2.3 at the end instead of compositing one weighted average. The three that are
  not are `Lab` (§11.3.4 forbids it), a profile with no "from CIE" half (§11.3.4 requires one),
  and a page that has spent `colour::MAX_PRESSES`.
- **The disagreement is now four times the size it was and is one decision.** ADR 0797 recorded
  §11.5.3's colorimetric branch parting from every reference on `issue21346.pdf`; this round
  measures the same parting over 41 crawl pages. Whether the clause's `Y` or the references'
  device grey is what this viewer should draw is the owner's to rank, and it is now one question
  rather than three.
- **`render-quorra` refuses every four-component mask**, and the ask in `doc/QUORRA_FEEDBACK.md`
  §43 is wider than it was: a second body, not just a field.
