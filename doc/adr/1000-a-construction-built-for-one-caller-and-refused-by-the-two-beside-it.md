# 1000 — A construction built for one caller, and refused by the two beside it

Session 979. Status: **accepted**. Clause 11 round; the topic was transparency, and the map was
the ledger's `partial` rows of that clause read against the code, by pixel consequence.

## 1. What was chosen, and what was not

Five candidates were on the brief. Four of them — §11.4.4's blend mode at the `Do`, §11.3.7.2's
shape channel on every command, §11.7.5.3's stated black generation, §11.7.2's conversion between
two presses — are each *reported* today, each has **zero** corpus witnesses by `tests/corpus.rs`'s
own listing, and each is priced in `doc/todo/23` as a debt or as a file that departs. The fifth,
§11.7.4.4's implicit knockout group, is the one `partial` row of the family with corpus witnesses
that are **drawn wrong rather than reported** — two documents in the corpus's incomplete list under
`CompositedInParts`, whose report says a `B` was "pushed flat": its stroke composited over its own
fill, which is exactly the double border §11.7.4.4's NOTE 2 exists to prevent. A page drawn wrong
outranks a page refused, so that is the row.

## 2. What the clause says

§11.7.4.4, second bullet — the only one this device can reach, since §8.6.7 keeps overprinting off:

> In all other cases, a non-isolated knockout group shall be established. Within the group, the
> fill and stroke shall be performed with their respective prevailing alpha constants and the
> prevailing blend mode. The group results shall then be composited with the backdrop, using an
> alpha value of 1.0 and the Normal blend mode.

§9.3.8 says the same thing of a text object under `Tk`: "the behaviour shall be equivalent to
treating the entire text object as if it were a non-isolated knockout transparency group", and
"the group results shall be composited with the backdrop, using the Normal blend mode and alpha
and soft mask values of 1.0". And §11.4.6 says what such a group computes, per element: "a)
Composite the source object with the group's initial backdrop, disregarding the object's shape and
using a source shape value of 1.0 everywhere", then "b) Compute a weighted average of this result
with the object's immediate backdrop, using the source shape as the weighting factor."

So three things are fixed by the clauses and one varies. The group's `Do` carries alpha 1.0, no
mask and the Normal blend mode; what varies is what the *parts* carry — their own constants, a
soft mask in force, and the prevailing blend mode — and each of those is a different construction
for a rasteriser that holds one alpha per pixel.

## 3. What the code did

`transparency::knockout_group_elements` was the one function both callers asked, and it answered
`None` — the parts drawn flat and named — for two of the three cases:

- **A part under a soft mask** (`/AIS false`). Its shape is its geometry and the mask is opacity,
  so the shape has to be *stated* beside the object: `Command::Shaped`, which ADR 0234 built and
  `knockout_elements` had derived for a form `XObject`'s knockout group since then, in the same
  file, forty lines above. `knockout_group_elements` never asked it. §11.6.2's ledger row recorded
  this as deliberate — "`Command::Shaped` is deliberately not used here … this one has no corpus
  witness to size the second construction against" — which is a decision not to use a
  construction that already exists, exact, in the function beside it.
- **Any part that blends.** Refused outright, on the argument that a group drawn on transparency
  gives a blending element nothing to blend against. True, and the corpus's two witnesses are both
  this: `issue17215.pdf`'s `b` under `/BM /Difference` and `issue14438.pdf`'s `B` under
  `/BM /Multiply`, both with fill and stroke in one colour.

This is `doc/habits/the-ledger-and-claims-about-this-tree.md`'s third shape — a capability that
arrived and announced nothing — at its smallest scale yet: not between crates or sessions but
between two functions of one file, where the one that could state the shape was written for the
form-group caller and the two implicit-group callers kept asking the older one.

## 4. The decision: three constructions, each exact, in `implicit_knockout_group`

`transparency::implicit_knockout_group(parts, alpha, inside_knockout)` answers with
`ImplicitKnockout { elements, blend, isolated }` — the three fields of `Command::Group` the clauses
leave to the parts — and tries, in order:

1. **Nothing blends.** The elements are drawn on transparency, `blend: Normal`, `isolated: true`.
   §11.4.4's NOTE 3 makes the isolated group stand in for the non-isolated one exactly wherever
   every element paints Normal: the backdrop is composited in and removed again, so it cancels
   (ADR 0237's argument, unchanged by knockout — ADR 0307). Under `/AIS false` an element whose
   shape is its coverage draws bare and every other states it (`knockout_elements`); under
   `/AIS true` every element states the pair (`stated_elements`, ADR 0415). **This is the masked
   case, and it is the case the old function refused.**

2. **Every element blends under one mode `M`, and `M` commutes with §11.4.6's weighted average.**
   The mode moves from the elements to the group's `Do`, and the elements are drawn Normal on
   transparency. The derivation is in `blend_at_the_do`'s comment and is repeated here because
   it is the decision. For two elements over a backdrop `B` (alpha `α_b`, colour `C_b`), with
   `w₁ = (1 − f₂) × α₁` and `w₂ = f₂ × q₂` the weights stage b) leaves each, the clause's
   accumulation is, premultiplied,

   ```text
   P = (1 − w₁ − w₂) × α_b × C_b + (1 − α_b) × (w₁ C₁ + w₂ C₂) + α_b × (w₁ M(C_b, C₁) + w₂ M(C_b, C₂))
   ```

   Drawing the elements Normal on transparency accumulates `K = w₁ C₁ + w₂ C₂` at alpha
   `w₁ + w₂`, and compositing that onto `B` under `M` by §11.3.3 gives the same first two terms
   and `α_b × (w₁ + w₂) × M(C_b, K ⁄ (w₁ + w₂))` for the third. They are equal exactly when `M` is
   affine in its source argument for a fixed backdrop — per component, `Multiply` is `c_b × c_s`,
   `Screen` is `c_b + c_s − c_b × c_s`, `Exclusion` is `c_b + c_s − 2 × c_b × c_s`, and `Overlay`
   is `HardLight(c_s, c_b)`, whose branch is on the backdrop and whose arms are the first two —
   **or** when `C₁ = C₂`, in which case `K ⁄ (w₁ + w₂)` is that colour whatever `M` does to it.
   At every *point* the shapes are 0 or 1 (§11.6.4.2) and one weight vanishes, so the two agree
   everywhere the standard defines a value; the condition is about the fractional pixel a
   rasteriser makes of an edge, and it is held exactly rather than allowed as anti-aliasing
   residue. Both witnesses take this route — `Multiply` is affine, and `issue17215.pdf`'s
   `Difference` is not but its two parts are one yellow — and **every backend draws it**: an
   isolated knockout group with a blend mode at its `Do` is an ordinary group to all three.

3. **Otherwise, §11.4.6's own backdrop.** Every element states its shape, `blend: Normal`,
   `isolated: false` — ADR 0327's construction, which retains the initial backdrop beside the
   accumulation and is drawn by the oracle alone; `render-gpu` and `render-raster` refuse it by
   name. Not inside a knockout group: NOTE 6 gives a nested group "the same [initial backdrop] as
   that of the outer group; it is not the immediate backdrop of the inner group", and the
   construction seeds from the immediate one, so there it stays a report.

`knockout_group_elements` keeps its signature and is now this function with the two answers its
callers cannot state — a blend mode at the `Do`, the group's own backdrop — filtered off. So the
two callers take route 1 today with no change to their lines, and routes 2 and 3 the moment they
ask the new function.

## 5. The callers, and why they are not switched in this commit

Route 2 needs the caller to put `group.blend` on the `Command::Group` it builds, and route 3
needs `group.isolated`. The callers are `content/path.rs` (`B`, `B*`, `b`, `b*`) and
`content/text.rs` (§9.3.8's text object and §11.7.4.4's mode-2/6 glyphs), and this round was
given neither: `text.rs` is the font round's and `path.rs` is unassigned, and the rule for a
parallel round is that a file it was not given is not its to change. The change is the same three
lines at each of the three sites, and the `path.rs` form of it — 45 lines of diff, the import
included — was applied **temporarily** here to measure, and reverted:

```rust
if let Some(group) = implicit_knockout_group(
    &parts,
    self.alpha_sources.settled(),
    self.inside_knockout,
) {
    self.draw(Command::Group {
        commands: group.elements,
        alpha: 1.0,
        clip: None,
        mask: None,
        blend: group.blend,
        isolated: group.isolated,
        knockout: true,
        alpha_is_shape: false,
        blending: None,
    });
}
```

with `use super::transparency::{Painted, implicit_knockout_group};` in place of the old import,
and `BlendMode` dropped from `path.rs`'s `pdf_render` imports because nothing there names it
afterwards. `text.rs`'s two sites (`end_text_object`, `push_combined_glyphs`) are the same
substitution.

## 6. What it measures

With the `path.rs` patch applied, in the same sitting:

- `issue17215.pdf` at scale 2: **before**, a yellow shape with a black line inside it — the
  stroke's `Difference` taken against the fill's yellow; **after**, a plain yellow shape, 197
  pixels changed. Both parts are composited with the page's *initial* backdrop, which under
  §11.4.7's isolated page group is transparent where nothing has been painted, so §11.3.6's
  "αs = 0.0 or αb = 0.0 results in no blend mode effect" makes both the source colour. `mupdf`
  and `poppler` draw the shape **blue** — `Difference` against a white backdrop — which is the
  §11.4.7 reading this tree has held since §11.3.6's row was written and is not this round's to
  reopen; what this round changes is that the stroke no longer blends against the fill, which no
  reading of any clause allows (§11.6.2: "[p]ortions of an object shall not be composited with
  one another").
- `issue14438.pdf` at scale 2: 125 pixels changed, all on the rim of one `Square` annotation's
  yellow box under `Multiply`. At a rim pixel: **before** `(255, 255, 116)`, the stroke's half
  inside the fill multiplied twice; **after** `(255, 255, 132)`, which is the annotation's own
  `(1, 1, 0.518)` once over white — and `mutool` draws `(255, 255, 132)` at the same pixel.
- The corpus gate: **63 → 61** incomplete over 974 documents, the two witnesses leaving and
  nothing arriving; `CompositedInParts` **2 → 0**.
- The oracle: see `doc/history/979-*.md` for the run's own lines.

Without the patch — this commit as it stands — neither witness moves, and the four unit tests in
`transparency.rs` hold each route against §11.4.6's arithmetic and against the flat drawing:
route 2 at `(128, 0, 0)`, `(255, 0, 0)` and the half-covered `(191, 0, 0)` where flat gives
`(128, 0, 0)` at the overlap; route 3 at `(0, 0, 0)` and `(178, 76, 0)` where flat gives
`(0, 76, 0)`.

## 7. What is left, and what it costs

- **Route 3 on two backends.** `render-gpu` and `render-raster` refuse `isolated: false` beside
  `knockout: true` by name, and a page taking that route goes to the oracle. **No corpus page
  takes it** — both witnesses are route 2 — so `render-raster`'s `REFUSED_BEFORE_THE_SCENE`, held
  to equality, does not move; a crawl page with two parts under `Difference` in two colours would,
  and that list is the colour round's file this session.
- **A form `XObject`'s non-isolated knockout group** (`run_transparency_group`'s
  `backdrop_composited`) still takes ADR 0327's construction whenever its elements blend. Route
  2's argument extends to it unchanged — with alpha `w_g` and a mask at the `Do` the collapse is
  `(1 − w_g α_K) B + w_g α_K (…)` on both sides — so a form group whose elements share an affine
  mode could be handed to all three backends. Not taken here: its one corpus witness,
  `issue18032.pdf`, is drawn by the oracle already, and moving it changes the quorra gate's lists.
- **The reports' wording.** `CompositedInParts` and `TextKnockout` still say only that the parts
  were pushed flat; naming *which* route declined them is the callers' sentence to write.

## 8. The trap this leaves, for the group file that holds trap 1

**Two functions in one file can be the same capability arriving and announcing nothing.** Every
instance of that shape on record was between a crate and a host, or between a session and a row;
this one was forty lines apart, and the function that refused carried a doc comment explaining the
refusal in terms the function above it had already answered. The sweep that finds it is not a grep
over reasons — the reason named no blocker — but reading a refusing function's *neighbours* for
the construction it says does not exist.
