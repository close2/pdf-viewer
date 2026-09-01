# 871 — A component composited, and a curve out: `CalGray` and `ICCBased` 'GRAY' blending spaces are drawn, and two of sixteen `MAX_FORM_DEPTH` witnesses are not cycles

Date: 2026-09-02.
ADR: [0792](../adr/0792-a-component-composited-and-a-curve-out.md).
Touched: `crates/pdf-render/src/blending.rs`, `crates/pdf-render/src/display_list.rs`,
`crates/pdf-render/src/repeat.rs`, `crates/pdf-render/src/group_cost.rs`, `crates/pdf-render/src/lib.rs`,
`crates/pdf-model/src/colour.rs`, `crates/pdf-model/src/content.rs`,
`crates/pdf-model/src/content/transparency.rs`, `crates/pdf-model/src/image.rs`,
`crates/pdf-model/tests/transparency_groups.rs`, `crates/render-cpu/src/lib.rs`,
`crates/render-cpu/tests/group_constructions.rs`, `crates/render-quorra/src/lib.rs`,
`crates/render-quorra/src/scene.rs`, `crates/render-gpu/src/lib.rs`, `crates/render-gpu/src/scene.rs`,
`crates/viewer-confined/src/protocol/display_list.rs`, `crates/test-scenes/src/lib.rs`,
`doc/conformance/ledger.toml`, `doc/todo/01-ledger-partial-rows.md`,
`doc/todo/03-more-corpora.md`, `doc/todo/23-transparency-departures.md`,
`doc/todo/49-restrictions-worth-re-examining.md`, `doc/state-of-play.md`; and the merge commit
before this round's own.

## The merge

`round-867`'s committed tip (`1e4dedf8`, round 868: a font cache per rayon split in
`pdf-transform`, the transform gate on `doc/todo/02` §2's sequence and in `tools/state.sh
transform`, inline images and `--native`, §12.5.6.15's attachments, ADR 0801, `doc/todo/57`
rewritten) was merged into `main` with `--no-ff` and a body; round 870's worktree on that branch
was not touched. The whole §2 sequence, the transform gate now among its lines, ran on the merged
`main` alone on the machine and every line was green; the figures are in the run's log and the
second run's below is compared against them line by line.

## The spec track: §11.3.4's curved greys

ADR 0790 drew `DeviceGray` blending spaces and named `CalGray` and `ICCBased` 'GRAY' as the debt.
Read against §11.3.4, §8.6.5.2, §8.6.5.5, §11.4.7 and §11.6.6, three things are the clauses' and
one is not: the group composites the space's own *component*; the conversion out is §8.6.5.2's
gamma and white point or the profile's `AToB`, at the end, before the medium; a profile the tree
cannot read substitutes by §8.6.5.5's own rule before any blending question arises; and the
conversion *in* is stated nowhere, so it is the inverse of the conversion out on the greys — the
construction ADR 0263 chose for a press, one dimension down. `Compositing::Calibrated` paints
every colour as its component, `pdf_render::GreyCurve` rides on the display list or on the group
(`GroupBlending` is an enum now), and `resolve_grey` applies it where the four-component pair
resolves. Tests derive 188 of 255 for a half-alpha black over white in a `/Gamma 1` `CalGray`
group from §11.3.6's average of the two components and sRGB's transfer function, against device
grey's 128; `mupdf` and `ghostscript` put it at 188 and 187, so the two references that honour the
space use the same construction, and the difference that stays is ADR 0790's conversion in of a
chromatic colour. `press_census` over all 145 crawl archives, 62 s under `tools/bounded.sh`,
found the crawl's three `ICCBased` 'GRAY' page groups (`1407449.pdf`, `2760152.pdf`,
`6942624.pdf`, all `kTRC`) and no `CalGray` one; the three draw and match the references by eye.

## The demand track: the sixteen witnesses

`doc/todo/03` section 39. The GHOSTSCRIPT directory was surveyed again to name the sixteen (172
s, 13.5 GiB peak under the bound), and ADR 0271's experiment was run on each at 256, then at 32
and 64: **fourteen are cycles and two are not**. `GHOSTSCRIPT-697655-0.pdf` (pdftk over iText)
draws whole at 32 and `GHOSTSCRIPT-695948-0.zip-0.pdf` (Aspose.Pdf 9.3) at 64, and both draw a
blank page at 16 where `mupdf` draws the document. The constant was not moved; `doc/todo/49` now
carries it as a decision owed with the figure a raise has to measure, and ADR 0271's sentence that
every witness is a cycle is marked false there.

## Gates and binaries

The full §2 sequence, twice: on the merged `main` before this round's change and after it, each
alone on the machine; the round's own two walks and the scratch builds ran between them. The
second run's workspace tests failed once, on `render-gpu`'s and `render-quorra`'s refusal tests
asserting the words "four components" of a message this round had generalised — trap 27's shape,
an assertion on a substring — and the fix was to make the refusal name the shape it refuses
(pair or curve) and to give the curve its own scene, CPU test and two refusal tests; the core
lines, the quorra gate and the conformance gate were then run again and were green, every other
line's figures being identical to the merge run's. §5's binaries, `pdf-transform` among them for
the first time, rebuilt and installed after that. Figures are in the runs rather than here.

## For the next round

- `doc/todo/49`'s `MAX_FORM_DEPTH` decision, with two real witnesses and a method.
- `doc/todo/23`'s two remaining rows for §11.3.4: `ICCBased` 'CMYK' (`B2A`), and the route into a
  one-component space, one decision with the masks.
- `doc/todo/03`: `batch4` once it lands, `batch5`'s trackers, and the interpretation-side clip
  cost `40` was handed a witness for.
