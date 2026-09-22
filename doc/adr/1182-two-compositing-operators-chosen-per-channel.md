# 1182 — Two compositing operators, chosen per channel, and neither scene vocabulary can choose

Status: accepted. Session 1172.
Prices: ADR 1178's finding that `render-raster`'s by-name refusal of the special overprinting
blend mode is the largest coverage loss either scene backend carries — 1788 crawled documents,
2.7% of the crawl that opens once the verdict is settled against the commands (ADR 1181).
Builds on: ADR 1157 (the mode), ADR 1158 section 1 (the two refusals), ADR 1169 (the empty kept
set), ADR 1170 (the two implicit groups).
Context: `crates/render-cpu/src/blend.rs`, `crates/render-raster/src/lib.rs`,
`crates/render-raster/src/scene.rs`, `crates/render-gpu/src/lib.rs`,
`crates/render-gpu/src/scene.rs`, `doc/QUORRA_FEEDBACK.md` section 49.
Clauses: ISO 32000-2 §11.3.5, §11.3.6, §11.7.4.3 (Table 146).

## 1. What the mode is, in the vocabulary a rasteriser already has

§11.3.6's compositing formula, in the premultiplied form `render-cpu::blend::composite`
computes, is `cr = (1 − αs)·cb + (1 − αb)·cs + αs·αb·B(Cb, Cs)` with `αr = αb + αs − αb·αs`,
where the straight `cb` and `cs` are each scaled by their own alpha. §11.7.4.3's first bullet
gives `B` two values and no others:

> If the overprint mode is 1 (nonzero overprint mode) and the current colour space and group
> colour space are both DeviceCMYK , then process colour components with nonzero values shall
> replace the corresponding component values of the backdrop; components with zero values leave
> the existing backdrop value unchanged.

Substituting each of them collapses the formula, and this is the finding:

- a **kept** component has `B = Cb`, so `cr = αb·Cb·[(1 − αs) + αs] + (1 − αb)·αs·Cs`, which is
  `cb + (1 − αb)·cs` — **Porter-Duff destination-over**;
- a component it does not keep has `B = Cs`, so `cr = (1 − αs)·cb + αs·Cs·[(1 − αb) + αb]`, which
  is `cs + (1 − αs)·cb` — **Porter-Duff source-over**, the arithmetic of Normal.

The alpha is one union either way. So the mode is not a seventeenth *blend function* at all: it is
**two compositing operators every rasteriser already has, chosen per channel by three bits the
command carries**. `render-cpu`'s
`blend::tests::the_special_mode_is_destination_over_in_the_channels_it_keeps` is the identity
held against `composite` itself, on an input calibrated so that the two operators differ by more
than eight-bit rounding in every channel.

## 2. Why that does not let either backend draw it today

Both refusing backends submit a *scene* and neither can vary an operator within one mark.

- **`render-raster`.** `raster_scene::Compose` has `SrcOver`, `Src`, `DestOut` and `Plus`, and no
  destination-over. The staged `DestOut` + `Plus` pair §11.4.6 needs is `P' = (1 − f)·P + S` and
  cannot produce `P + (1 − αb)·S`: the missing factor is the *destination's* alpha per pixel,
  which no scene-side value carries. So the all-kept case is out of reach as well as the mixed
  one.
- **`render-gpu`.** `peniko::Compose::DestOver` exists, so a mark whose kept set is all three
  channels is expressible as a layer under `(Mix::Normal, Compose::DestOver)` — and a mark whose
  set is empty is Normal exactly. A **proper subset** is not: Vello composites a layer whole, and
  two layers cannot each be confined to a different channel.

Neither library offers a channel write mask, which is the other shape that would do it. Both
refusals therefore stand, and this ADR is what a round that wants to lift one has to read first.

**One thing worth having straight about `render-gpu`, because ADR 1178 counted it with
`render-raster`**: that backend refuses every page *and* every group compositing in four
components before it reaches the overprint test (`GpuRasterizer::rasterize`,
`scene::refuse_untranslatable_group`), and the special mode is chosen only where four-component
compositing is in force. What the overprint refusal actually takes off it is the one position
left over — a §11.6.5.1 soft-mask group whose own blending space has four components on a page
whose space is the device's, which that backend does draw (`soft_mask.rs`). The 1788 documents are
`render-raster`'s loss, not both backends'.

## 3. The ask, stated as what it now is

`doc/QUORRA_FEEDBACK.md` section 49 asked for "one variant carrying three bits". Section 1 above
makes it smaller and more familiar, and the ask is restated there in those terms: a `Compose` a
mark may carry **per channel**, which for the kept channels is destination-over and for the rest
is source-over. A vocabulary that cannot hold a per-channel operator can still take the two
uniform cases with `Compose::DestOver` alone, and the census says how much of the population that
is. Over the same 65 944 crawled documents, of **27 435 261 marks under the mode: 13 772 602 keep
all three channels, 13 650 173 keep none, and 12 486 keep a proper subset** — 0.046% of them. Per
page and per document it is starker, because a page reaches the mode in both halves of §11.4.7's
pair at once: **9734 of 9863 pages and 1711 of 1788 documents state no proper subset at all.** So
`Compose::DestOver` by itself — one operator, no per-channel anything — is 95.7% of the documents
this refusal costs, and the per-channel choice is 77 documents and 129 pages
(`examples/overprint_ink_group_census`, the kept-set column added this session).

**Nothing of this is built inside `raster/`.** That is a separate project with its own brief and
its own team, reached from here through `QUORRA_FEEDBACK.md`; its own documents do not mention
overprinting at all. Whether this tree may build the mode there instead of asking for it is
`doc/questions/Q76`.

## 4. What is not decided here

Whether `render-gpu` should take the two uniform cases for the soft-mask position alone. It is a
real narrowing and an exact one, and it is left undone deliberately: the flag two backends read is
one boolean over a whole list, so taking it would mean the list carrying which shapes of kept set
are on it, for a population nothing has measured and that this round's census does not separate.
A round that measures that position first can build it in an afternoon.
