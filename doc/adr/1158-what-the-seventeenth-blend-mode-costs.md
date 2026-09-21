# 1158 — What the seventeenth blend mode costs

Status: accepted. Session 1160.
Prices: ADR 1157's construction — the two backends that refuse it, the two constructions around it
that are reported rather than built, and what the default path pays.
Context: `crates/render-gpu/src/lib.rs`, `crates/render-raster/src/lib.rs`,
`crates/pdf-render/src/display_list.rs`, `crates/pdf-model/src/content/overprint.rs`,
`crates/viewer-confined/src/protocol/display_list.rs`.
Clauses: ISO 32000-2 §11.3.5.2, §11.4.4, §11.7.4.3, §11.7.4.4.

## 1. Two backends refuse it, out loud

`BlendMode::Overprint` is not one of Table 134 and Table 135's sixteen. Vello's `Mix` has sixteen
arms and `raster_scene::BlendMode` has sixteen arms, and the value §11.7.4.3's first bullet gives
that is not the source colour is not among them. Mapping it to Normal would draw the page the
document asked *not* to be drawn — the erasure the producer turned off — with nothing saying so,
against the backend `CLAUDE.md` keeps as the correctness oracle. So both refuse the list by name,
before anything is drawn, and the frame falls back to `render-cpu`.

`render-gpu` already refused every page compositing in four components, so nothing there is newly
refused. `render-raster` *draws* such a page (ADR 0262's second raster, `QUORRA_FEEDBACK.md`
section 17), so this is a real narrowing of its coverage: a page that composites in ink **and**
overprints with a zero tint is now its refusal rather than its picture. That is the smaller cost of
the two available, and it is the one the cross-backend comparison can see.

**The refusal reads one bool rather than walking.** `DisplayList::overprints` is set by the single
place that chooses the mode, and a page is interpreted into one list — a soft mask's group, a
tiling pattern's cell and a form's content are all built on it and split back out — so no walk is
needed to find out and no route can hide one. `set_blending` folds the black half's flag into the
chromatic half's; the confined protocol writes the flag beside the page size, so the renderer
process refuses what the host would have.

## 2. Two constructions are reported rather than built

Both are the clause's *grouping* around the mode rather than the mode itself, and in both the marks
are painted, in the right place, under the document's own blend mode. `Unsupported::Overprint` says
which.

**§11.7.4.3's last paragraph**, the implicit non-isolated, non-knockout group for an object painted
while the current blend mode is not Normal. Its NOTE 3 makes the group unnecessary when the mode
*is* Normal — "simply substituting the special blend mode while painting the object produces
equivalent results" — which is what is built; the other case is named.

**§11.7.4.4's first bullet**, for a combined fill and stroke. Two derivations narrow what is owed
to almost nothing, and both are worth stating because each removes a case that looks owed:

- Where the special mode's value is `C_s` in every component — every case but §11.7.4.3's first
  bullet with a zero tint — the first bullet and the second composite each part against the same
  backdrop under the same alpha and the same mode. They are one picture, so the choice between them
  is unobservable and `combined_overprint` returns without reporting.
- Where the pair's two alpha constants are 1.0 and the blend mode is Normal, the first bullet's
  group *is* the two commands drawn one after the other: a non-isolated group composited onto its
  own backdrop at an alpha of 1.0 under Normal returns its elements unchanged, which is §11.4.4's
  own cancellation and the identity `render-cpu`'s `interpolate` rests on. `end_path` and the text
  object already draw them that way, so the bullet is satisfied with nothing built.

What is left is a pair whose own alpha or mode is not the identity while a part keeps a component
of the backdrop. That is reported. §11.6.2's group is not built for it either, because the second
bullet's knockout is the construction the first bullet replaces.

**And one condition widened.** A part that keeps a component of the backdrop composites with what
is under it however opaque it is, so `paint_composites`'s question — can the portions of an object
differ? — now has the special mode as a third answer beside a non-unit alpha and a non-Normal mode.
It is asked at the call site rather than in `GraphicsState`, because the answer needs the
compositing target and a `GraphicsState` cannot see one; asking it there also keeps an ordinary RGB
page that sets `/OP true` out of the knockout construction entirely, which is where it belongs.

## 3. What the default path pays

**0.032% of one page's interpretation, measured rather than argued.** Page 101 of ISO 32000-2
interpreted fifty times under callgrind, against the same build with the added calls planted away
(`doc/habits/measuring.md`'s A/B in one sitting, attributing by removing the suspect):

| arm | instructions | over the planted-away build |
|---|---|---|
| the feature planted away | 1 246 995 423 | — |
| `overprint_blend` as one function | 1 250 749 913 | +3 754 490, 0.30% |
| everything but `overprint_blend` | 1 247 033 954 | +38 531, 0.0031% |
| shipped: `overprint_blend` split hot/cold | 1 247 391 917 | **+396 494, 0.032%** |

The third row is what says where the cost was: two calls per painting operator, not the dictionary
lookups and not the graphics state's forty extra bytes. The fourth is what the split bought, and
`Interpreter::overprint_blend`'s own comment carries both numbers beside the technique.

What was added, for the reader who wants the list rather than the figure:

- **Three dictionary lookups per `gs` operator** (`/OP`, `/op`, `/OPM`), beside the twenty-odd
  Table 57 already reads there. Not per mark.
- **One `matches!` and one `TryFrom<&[f32]>` per colour operator**, in `cmyk_tints`.
- **Two calls to `overprint_blend` per painting operator**, which on every page this renderer
  draws on the device's three components — all but the fraction `examples/press_census` counts —
  is an inlined integer comparison and an inlined discriminant test. Not per pixel.
- **About forty bytes on `GraphicsState`**, which `q` clones. This is the one cost that is paid by
  a page that states nothing: three booleans, an integer and two `Option<[f32; 4]>`.
- **One bool on `DisplayList`**, read once per frame by two backends.
- **Nothing at all in `render-cpu`'s inner loops**: `Computed::of` is the function `NonSeparable::of`
  already was, called once per command, and a command that is not one of the five computed modes
  takes the same path it took before.

And nothing per pixel anywhere: `raster_golden` moved 966 of 974 first pages on the list digest —
one new `DisplayList` field inside a `Debug` rendering — and **not one of them on its raster**.

## 4. What is not decided here

Whether a producer that writes `/OP` inside a `DeviceCMYK` group is common enough for the
`render-raster` narrowing to matter is a *count*, and no command in this tree prints it yet: the
census that would — pages whose group composites in ink *and* whose content enables overprinting
with a zero tint — is `doc/todo/23`'s to add. Until it exists the refusal is the conservative
answer and the report names the page.
