# 1295 — The special overprinting mode is drawn in `raster/`, from the clause alone

Status: accepted. Session 1229.
Acts on: `doc/questions/A76` (the owner's answer to `Q76`: "I agree with your recommendations").
Builds on: ADR 1157 (the mode), ADR 1182 (what it reduces to), ADR 1181 (the verdict).
Closes: `doc/QUORRA_FEEDBACK.md` section 49's ask.
Clauses: ISO 32000-2 §11.3.3, §11.3.6, §11.3.7.3, §11.7.4.3, §11.7.4.5 (Table 146).

## 1. The independence the answer ratified, and how it was kept

A76's reading: build it "from Table 146 and §11.3.6 alone, do not look at `render-cpu`'s
`Computed::Overprint` while doing so, and let `doc/verify.md`'s cross-backend run be the first time
the two meet." This round did not open, grep or read any file under `crates/render-cpu/`. What it
read about the mode was the clauses in `doc/md/`, ADR 1182's text, `Q76`/`A76` and
`QUORRA_FEEDBACK.md` section 49. The arithmetic in `raster/` and its tests' expectations are derived
from §11.3.3's formula with §11.3.7.3's union and §11.7.4.3's two values of `B`, never from a
number the oracle produced.

## 2. The vocabulary: two operators, not a seventeenth blend function

`raster_scene::Compose` gains `DestOver` and `DestOverIn([bool; 3])`, and
`Compose::keeping(kept)` sends the two uniform selections to `DestOver` and `SrcOver`. With
`cr = (1 − αs)·cb + (1 − αb)·cs + αs·αb·B` premultiplied, `B = Cb` gives destination-over and
`B = Cs` source-over, one union alpha either way; the derivation and the verbatim sentences sit
on `Compose::DestOverIn`. Table 146's closing paragraph ("subtractive components shall be
complemented before and after application of the special blend function") needs nothing, because
a selection commutes with a complement.

The builder refuses the mode in three positions where it would meet a second rule, as
`SceneError::OverprintComposeUnsupported`: beside a blend mode other than Normal (the
interpreter already builds §11.7.4.3's implicit group for that), as an element of a knockout group,
and on a non-isolated group.

## 3. Two derivations inside `raster/`, so the library checks itself before the oracle does

- **`DestOver` is a blend state**, `(ONE_MINUS_DST_ALPHA, ONE)` on colour and alpha, in all
  four fixed lane families and the generated function lane: instanced like over, no layer.
- **`DestOverIn` draws through a layer**, composited by `composite.wgsl` with §11.3.6's formula as
  it stands and `B` selected per channel (`compose == 3`, the kept channels in a new `kept` word).
  Two draws under write masks could state it in fixed function at two pipelines per lane for each
  of six subsets; the census puts the proper subsets at 13 134 of 27 462 715 marks, so the layer is
  the cheaper place for the complexity. A group under either operator goes the same road, which is
  how `render-raster` states the mode for a stroke or an image: an isolated group of one element.

`raster-gpu/tests/overprint_compose.rs` holds both against the closed form over a translucent
backdrop (αb = 0.6, so destination-over is not the backdrop unchanged): worst premultiplied
deviation 0.83 of 255 for `DestOver` and at most 1.30 over the six proper subsets, and the two
derivations meet each other on the two uniform selections.

## 4. What the caller changes

`render-raster`'s by-name refusal is deleted: `scene::compose` maps `BlendMode::Overprint` to
`Compose::keeping`, fills carry it, and strokes and images take `scene::overprinted`'s group of one.
`crates/render-raster/tests/overprint_refusal.rs` is replaced by `tests/overprint.rs`, which holds a
fill and a stroke to the clause's closed form over an opaque backdrop. `render-gpu` keeps its
refusal: Vello's `Compose` is one operator for all channels, and its uniform case would buy no page
because that backend refuses every four-component space first.

A mark under the mode that is a direct element of a knockout group takes the group's knockout
pass unchanged, and a stroke or image there is not wrapped: raster's knockout groups are isolated,
and §11.3.6 says "An alpha value of αs = 0.0 or αb = 0.0 results in no blend mode effect", so onto
that transparent initial backdrop the mode is Normal's arithmetic. A *group* under the mode in that
position is still refused, for `KnockoutElementGroupUnsupported`'s reason.

## 5. The first meeting, and where the two readings stand

- **Hand-built lists** (`render-raster/tests/overprint.rs::the_two_backends_meet_on_the_mode`):
  all eight kept sets, a fill with an oblique edge and a stroke, over a backdrop at alpha 0.6, at
  4×. The two backends agree to at most **2 of 255** in any channel, mean at most 0.0078 under
  the mode — below the 0.135 the same scene shows with nothing kept, which is plain source-over.
- **The pdf.js corpus** (`render-raster --test corpus`): its one page under the mode,
  `issue12798_page1_reduced.pdf`, is still refused, now by the builder: its mark is under
  `/BM /Multiply`, so §11.7.4.3's last paragraph wraps it in a non-isolated group painted under
  Multiply, and `GroupSpec` draws §11.4.4's result step only under Normal. That is a separate ask,
  not this mode, so the name stays in `REFUSED_BEFORE_THE_SCENE` with its reason changed.
- **The crawl**: `overprint_ink_group_census` over `openpreserve` and the first 5000 files of
  `tika-issue-tracker` finds 40 documents under the mode. Comparing each first such page at 100%
  (`examples/zoom_ladder`, first rung), **33 now compare, all of them refused before**: mean 0.001
  to 0.398, worst tile at most 3.25 on 32 of them. The 33rd is `REDHAT-1302121-0.pdf` at 6.42:
  306 pixels differ by more than 40 levels, all on the anti-aliased edges of CMYK disks, and at
  200% the page's worst tile is 1.15. In those pixels the channel the mark keeps agrees to about 3
  levels and only the replaced channels differ. Those channels are source-over weighted by
  coverage, so what separates the two backends there is coverage — §10.7.4's area rule, left to
  each rasteriser — and no sentence of §11.7.4.3 decides between them. The other six are
  refused for reasons that are not this mode: three for §11.4.4 under a non-Normal blend, two for
  a clip made of two fills, and one for a four-component group. `PDFBOX-4095-10.pdf` is among the
  33 only because of section 4's knockout reading. The round's first draft refused it; it now
  compares at mean 0.279, worst tile 3.25.
