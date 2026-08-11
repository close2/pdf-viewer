# ADR 0274 — A refusal that came off, and one that changed its reason

Date: 2026-08-11 (session 438)
Status: accepted

## Context

`doc/QUORRA_FEEDBACK.md` section 16, written in the four-hundredth session, asked the quorra side
for one thing: **a flag on `GroupSpec` saying whether a group's buffer begins transparent or as a
copy of what is under it.** That is Table 145's `/I` and it is the only entry of §11.4.5's group
the scene vocabulary did not carry. Until it arrived, `render-quorra` refused every non-isolated
group by name, and four corpus pages sat on the gate's refused list because of it.

The reply is `doc/QUORRA_NON_ISOLATED_GROUPS.md`, committed here beside the feedback it answers,
and the revision is `89d7dd77` (we pinned `2531f447`).

## What the pin brought, which is more than the flag

Ten commits and seven ADRs. Four of them can move a pixel or a number this tree gates on, so they
are listed rather than summarised:

| quorra ADR | what it is | what it means here |
|---|---|---|
| 0019 | `GroupSpec::isolated`, and the composite back | this decision |
| 0020 | layer textures priced by tree **depth**, not by plan count | a page refused with `FrameBudgetExceeded` may now draw; `Counters::layer_textures` reports the peak |
| 0021 | the scratch sheet is narrowed to the widest shelf and charged for its own bytes | a coverage frame moved 10.54 MB and now moves 0.46 MB; a page can now be refused for a sheet it would really have allocated |
| 0022 | the readback reads the mapped range once and divides never | a dense page's offscreen frame 4.94 → 1.65 ms, of which readback 3.84 → 1.32 |
| 0023 | `Options::instrument_encode`, `host_total()`, and the thread decision | section 13's instrument; quorra spawns no threads and will take a pool rather than make one |
| 0024 | atlas admission is a share of the budget, and the glyph key gained the fill rule | **a correctness fix**: one outline requested under non-zero and even-odd used to get the first tile twice wherever a subpath nests |
| 0025 | `Compose::DestOut` and `Compose::Plus` | section 14's ask, granted |

**Nothing in this tree had to change for any of the six that are not 0019**, and the corpus gate's
`differ` list is identical page for page, which is the evidence that 0024's correctness fix did not
reach our pages — consistent with it being harmless *here*, not with it being harmless.

One thing came without an ADR and it is worth naming because it is the last commit in the range:
`89d7dd77` itself replaces SipHash in the two per-glyph key maps with a twenty-line multiply-xor
hasher, on the argument that the keys are the encoder's own `u32`s from a friendly process and a
per-process random seed is a liability where a frame should be a function of its inputs. It pays
for 0024's extra key field twice over. **We inherit that argument rather than make it**, and it is
worth this tree knowing it exists: a hash whose seed is fixed is a decision about hostile input,
and the reason it is safe there is that quorra never sees a document.

## Decision

**Pass `isolated` through, and re-check nothing.**

```rust
let spec = quorra_scene::GroupSpec {
    alpha: *alpha,
    blend: blend_mode(*blend),
    clip,
    knockout: *knockout,
    mask,
    isolated: *isolated,
};
```

The refusal that stood above it is deleted. What replaces it is **not** a copy of the conditions
under which the flag is safe, and that is the decision this ADR exists for.

### Why not re-check the three conditions

§11.4.4's Result step removes the group backdrop by dividing by Table 140's group alpha, and NOTE 4
advises keeping that alpha in a second set of accumulators because a premultiplied raster has one:

> For shape and alpha, backdrop removal can be accomplished by maintaining two sets of variables to
> hold the accumulated values.

ADR 0237 showed the second set is not needed, because the quantity the removal divides out is
multiplied straight back in when §11.3.3 composites the group's result onto the same backdrop —
and that the step which cancels is the **Normal** blend function. So the identity

```text
result = (1 − w) × B + w × E(B)
```

holds exactly where the group's own blend is Normal, the group is not a knockout group, and no
enclosing group is one. `pdf-model` emits `isolated: false` only there (`Command::Group`'s
`isolated` states it as a guarantee to backends), and quorra accepts exactly there, refusing the
rest at `SceneBuilder::group` as `SceneError::NonIsolatedGroupUnsupported { reason }` with `reason`
naming which of the three broke.

**Three readings of one clause already exist**: `pdf-model`'s emission rule, `render-cpu`'s check in
`initial_backdrop`, and quorra's `check_isolation`. A fourth in the translation layer would be a
fourth place for §11.4.4 to be read, with nothing above it deciding a picture — a condition that can
only ever agree or drift. The refusal it would produce is already produced, typed, and attributed,
and it arrives where this crate already handles scene refusals (`QuorraRasterError::Scene`).

**What makes that safe rather than trusting**: the refusal is held by a test that builds the list
`pdf-model` never emits. `quorra_refuses_a_non_isolated_group_that_blends` hands the backend a
non-isolated group composited under `Multiply` and asserts the refusal names what it cannot do and
why. A silent wrong picture there is a failing test, which is the property the deleted refusal had
and the one worth keeping.

## What it draws, measured

`cpu_and_quorra_agree_on_a_non_isolated_group` replaces `quorra_refuses_a_non_isolated_group` on the
same scene, and it asserts a **pixel** as well as the tolerance, because a tolerance is what two
backends substituting §11.4.5's transparent backdrop passed for four hundred sessions. Opaque green
page, opaque blue element under `Multiply`, group alpha ½:

| | inside the group |
|---|---|
| non-isolated, which the clause states | **`(0, 128, 0)`** — Table 134's `B(cb, cs) = cb × cs` against the page gives black, and `(1 − ½) × green + ½ × black` is half green |
| isolated, which this used to refuse rather than draw | `(0, 128, 128)` — §11.3.6: "[a]n alpha value of αs = 0.0 or αb = 0.0 results in no blend mode effect", so the blue survives whole |

Both backends produce the first, to the byte.

### The corpus gate

| | agree | differ | refused | not comparable |
|---|---|---|---|---|
| at `2531f447` | 911 | 35 | 11 | 17 |
| at `89d7dd77` | **914** | 35 | **8** | 17 |

Three names leave `REFUSED`: `bug1755507.pdf`, `issue13520.pdf` and `issue18032.pdf`. The `differ`
list is identical page for page, and the quorra side measured the same movement independently on
its own copy of this tree — two transcriptions of §11.4.4, in two languages, meeting on real files.

**The pages were looked at, not only counted** (trap 1). `bug1755507.pdf` is a panel of Illustrator
artwork whose rounded box carries a drop shadow through a `/Luminosity` mask; `issue13520.pdf` is a
lozenge with a rimmed edge; `issue18032.pdf` is a gradient-filled rectangle. All three are
indistinguishable from the CPU oracle's render by eye, and their page inks agree to 0.07 of 255
(38.6464/38.6886, 21.6080/21.6694, 1.9671/1.9644).

### The fourth document is the finding

`issue12798_page1_reduced.pdf` was the fourth name section 16 put on the list, and it **stays
refused** — with a different reason:

> a page composited in a four-component blending colour space (§11.4.7)

The §11.4.4 refusal had been standing in front of a second one, on the same page, from a clause
three subclauses along. Nothing counted the difference; the *reason string* did. A refused list
that carried only names would have reported three closed holes here and said nothing about the
fourth having changed what it is waiting for.

## Sections 14 and 17 were answered too, and what they leave is ours

Both were checked against this revision rather than assumed still open, and both are answered:

- **Section 14** asked for `Compose::DestOut` and `Compose::Plus` and got exactly those (quorra ADR
  0025), with `DestOut` weighted by **shape** rather than by the paint's alpha — which is what
  `pdf_render::Command::Shaped`'s second member already is. Two positions refuse a staged mark
  (`StagedComposeUnsupported`), neither of which this tree emits, and `Plus` alone saturates, so the
  pair is the caller's obligation. `render-quorra` still refuses `Shaped`, and **its refusal message
  had to be corrected**: it said "quorra's Compose has no Destination-Out and no Plus", which became
  false the moment the pin moved. It now says the backend has not expanded the command yet, which is
  the truth and is `doc/todo/23`'s work.
- **Section 17** offered to close itself if two `Target::Readback` renders against one device were
  already supported and cheap. They are, and `quorra-gpu/tests/two_rasters.rs` now holds it: both
  rasters come back whole, resources are device-scoped so the second upload is strictly smaller, and
  the second pass pays **0.000 ms** of `encode: geometry` because the glyph key does not carry
  colour. The refusal in `QuorraRasterizer::rasterize` is on this tree's display list and is work
  owed here.

**A section that still reads as a complaint after the complaint was answered is worse than no
document**, which is `QUORRA_FEEDBACK.md`'s own opening rule — so 14, 16 and 17 gained
"what came back" subsections, and the summary table at the top of that file gained **dated columns**.
It had been claiming `914 / 42 / 1` throughout the entire stretch in which sections 14, 16 and 17
were written, and every one of those sections is about pages moving *onto* the refused list. The
instance is corrected; the dated columns are the fix for the shape.

## Consequences

- `render-quorra` draws §11.4.4's non-isolated group. `render-gpu` still refuses it, and that
  refusal keeps its test — a Vello layer begins transparent and a scene cannot read what it has
  drawn.
- `doc/todo/23`'s two remaining backend rows stopped being requests and became work: four corpus
  pages behind §11.4.6's two marks, three behind §11.4.7's two rasters.
- The quorra corpus gate's `REFUSED` ratchet is re-cut at eight names, and the ordinals in its doc
  comment now say so: they are the order names *arrived* in, and eleven arrived where eight remain.
- The three-condition guarantee is now load-bearing in a fourth place — it decides what
  `render-quorra` hands the library — and it is stated once, in `Command::Group`'s `isolated`, which
  is where a change to it has to go.
