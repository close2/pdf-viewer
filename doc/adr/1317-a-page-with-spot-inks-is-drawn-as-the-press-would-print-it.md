# 1317 — A page with spot inks is drawn as the press would print it: the planes on the list, and the four steps on two backends

Status: accepted. Session 1240.
Context: ISO 32000-2 §10.8.2, §10.8.3, §10.3.2, §10.4.2.1, §8.6.6.4, §8.6.6.5 (Table 70),
§11.3.5.2 (Table 136), §11.4.7, §11.7.3, §11.7.4.2, §11.7.4.3, §12.11 Table 275; traps 6, 11, 13,
40, 41, 50; ADRs 1228, 1229, 1281, 1311; `doc/questions/A100`.
Code: `crates/pdf-render/src/separation.rs` (new), `display_list.rs` (`set_separated`,
`separation`, `substitute_blend_modes`, the written-out `Debug`), `paint.rs`
(`BlendMode::on_spot_colourants`), `blending.rs` (`BlendingSpace::convert` inlined, section 9); `crates/pdf-colour/src/colour.rs` (`ColourSpace::flat_ratio`,
`separation_conversions`); `crates/pdf-model/src/colourants.rs` (`Source`, `flat_curves`, `plane`),
`content.rs` (`separate`, `draw_separated`), `content/report.rs`
(`Unsupported::SpotColourantsWithoutAPlane`), `requirements.rs`; `crates/render-cpu/src/lib.rs`,
`crates/render-raster/src/lib.rs`, `crates/render-gpu/src/lib.rs`;
`crates/viewer-confined/src/protocol/display_list.rs`; `crates/viewer-core/src/report.rs`.
Tests: `crates/pdf-model/tests/spot_press.rs` (new), `spot_planes.rs`, `colour_paths.rs`;
`crates/render-raster/tests/separation.rs` (new); `crates/render-gpu/tests/separation_refusal.rs`
(new); `pdf-render`'s `separation::tests`;
`viewer-confined`'s `a_separated_page_round_trips_to_an_equal_list`.

## 1. What this is: stages three and four of ADR 1281

ADR 1311 made the planes and drew none of them. This puts them on the display list and draws them.
**The separated page is the page drawn**: under the reader's simulation the interpretation's list is
the chromatic process plane, carrying the black plane as any four-component page does and the spot
planes beside it (`DisplayList::set_separated`). Step d) is "Convert the result to the actual device
colour space and output it", so a separation held beside the page and never output would be steps
a) to c) of a simulation nobody sees. The record `Interpretation::separation` keeps what the model
says about the planes — which colourants have one, how many planes, which colourant a mark painted
without one — and `colourants::plane` reads a plane off the list.

## 2. Step b), and the three things it leaves to a reader

> b) Convert each separation into "flat XYZ" (no gamma) and using a background matte of all white.

- **Where the matte goes.** A separation is a raster of one colourant's tints and the matte is what
  it is converted against, so the matte is composited under each plane in that plane's own
  components — no ink — before conversion: a stored additive complement `v` at the page's one alpha
  `α` (§11.7.3's "single shape value and opacity value") prints a tint of `α − v`. The result is a
  colour on the matte and is written opaque; a pixel the page did not paint is left for the medium,
  which is the same nominal white (§11.4.7's `W`).
- **Which route a colourant's tint takes to XYZ.** §10.4.2.1 ranks the routes: an ICC-capable
  processor "should always follow the provisions and recommendations provided in 10.3", and §10.3.2
  has it "establish CIE-based colour specifications for device colour spaces". So a `DeviceCMYK`
  alternate reaches XYZ through the page's output intent where it states one and through the
  assumed inks' sRGB definition where it does not (ADRs 0009, 0263) — `ColourSpace::flat_ratio`,
  the same `flat_xyz` ADR 1229's per-operation route uses — and never through §10.4.2.5's formula.
  The process separation takes the same route from the device colour its pair resolves to, through
  `colour::separation_conversions`' first cube, so a page separated into planes and one painting
  operation simulated alone share one conversion (trap 6).
- **Which space speaks for a colourant.** The standard states a colourant's appearance only through
  the alternate of a space that names it, and one page can name a colourant in several. The rank is
  this tree's and says so (`colourants::Source`): a `Separation` naming it, or the `Separation` an
  `NChannel`'s `/Colorants` holds for it (Table 70), first met first; failing both, a `DeviceN`'s
  tint transform with that component alone. The curve is sampled at 256 tints, one per level a plane
  stores, at the initial rendering intent, because a plane is one raster whatever intents its marks
  stated.

## 3. Steps c) and d)

Step c)'s multiply is Table 136's `B(cb, cs) = cb × cs` and its NOTE 3 makes white the unit, so each
separation enters the product as its flat XYZ over D50 — ADR 1229 section 2's arithmetic, per pixel.
Step d) is the second cube: the matrix back to linear sRGB in a grid of side two, which interpolates
a linear map exactly, and the sRGB encoding on its output curve. `pdf_render::resolve_separation` is
the one statement of all three steps, and both drawing backends call it.

## 4. §11.7.4.2's substitution is applied where the planes are made

"If the specified blend mode is not separable or not white-preserving, it shall apply only to
process colour components, and the Normal blend mode shall be substituted for spot colours."
`separate` rewrites every spot plane's modes by `BlendMode::on_spot_colourants` after the planes are
checked against one another, because the rule changes a spot plane's modes and not its structure.
So each list states what its plane composites under and no backend needs to know which plane is a
spot one; soft masks are not entered, since §11.7.3 keeps spot colours out of them.

## 5. `GroupBlending::FourComponents` gains nothing, and why the brief's design changed

`doc/todo/23` priced a spot field on `FourComponents`. On a spot plane a group passes the spot
colours through — §11.7.3's first bullet, `Interpreter::spot_group_compositing` answering inherit —
so no spot plane's group carries a conversion and the separation lives once, on the page's list.

## 6. The backends, and a refusal that was not needed

`render-cpu` draws every plane and resolves them. **`render-raster` draws them too, rather than
refusing as the brief asked** (trap 40): the pair a four-component page travels as is already drawn
there by a second whole render against the same device and resolved over the readback, and a spot
plane is one more of the same — with no geometry cost, because the glyph key has no colour in it
(`doc/QUORRA_FEEDBACK.md` section 17.1). `tests/separation.rs` holds its pixels to two levels of the
oracle's. `render-gpu` refuses by name, ahead of the pair's refusal, and the CPU backend draws the
page. `doc/QUORRA_FEEDBACK.md` section 53 asks whether the planes could be combined on the device.

## 7. Trap 41, the wire, the report

`DisplayList`'s `Debug` is written out field for field as derived and prints `separation` only where
there is one, so a page without one renders byte for byte as before. `raster_golden` moved the same
nine rows on the worktree with this round's hunks and with them reversed — the siblings' in flight —
so this round moved none. The codec carries a separated
page as the fourth blending shape and refuses one in a companion list. A colourant past the sixteen
planes that a mark painted is `Unsupported::SpotColourantsWithoutAPlane` on the page's report, which
`viewer_core` words.

## 8. Two tests of ADR 1229's route were about pages that are now separated

`colour_paths.rs`'s `simulated_fill` pages name spot colourants, so they are separated. Two
assertions were of the per-operation route and are now asserted on a page whose `DeviceRGB` group
the separation does not reach (`unseparated_fill`) — exact, as before — and on the separated page
with its own worked value: a `Separation` alone within one level (steps b) and d)'s round trip), and
the `NChannel` whose `Spot2` has no `/Colorants` entry as blue times red, (116, 0, 37).

## 9. Cost

callgrind, one sitting, two trees exported from this worktree with separate target directories
and different `md5sum`s (trap 50): the worktree as it stood, and the same with this round's hunks
alone reversed, so both carry the siblings' work in flight. Neither page separates, which is the
point — the ordinary path must not pay.

| | before | after |
|---|---|---|
| interpret, page 101 of the standard ×50 | 1 279 245 491 | 1 279 296 944 (+0.004%) |
| interpret, a 3000-mark `DeviceCMYK` page | 49 224 713 | 49 200 842 (−0.05%) |
| rasterise, page 101 ×20 | 5 170 019 730 | 5 174 921 040 (+0.09%) |
| rasterise, the `DeviceCMYK` page ×5 | 2 822 189 323 | 2 819 951 396 (−0.08%) |

The same `before` binary measured 5 166 708 866 and 5 172 653 840 for page 101 in two other
sittings, so the page-101 rasterise row is inside the strips' scheduling noise.

**The first build paid 4.9% on the two-plane path, and the diff per function found why** (habit
45): `BlendingSpace::convert`, inlined into `blending::resolve` while that was its only caller,
became an out-of-line call once `separation::resolve` called it too — 916 870 246 instructions in
`resolve` became 867 464 280 in `convert` plus 104 559 884 in `resolve` over two draws. A plain
`#[inline]` did not move it; `#[inline(always)]`, with the number beside it, did.

## 10. The row

**§10.8.3 stays `partial`, on a measured residue rather than on the control.** `doc/questions/A100`
settles the control: a requirement executed under a control a host supplies is executed, so the
default being off decides nothing. The GPU refusal decides nothing either — §11.6.7's row names the
same shape "a backend refusal and not a gap in the clause". What is left is one shape of page, and
`examples/spot_depth --separate` over the crawl's first ten pages counts it: 8 517 pages name a spot
colourant, 7 566 are separated, and 951 are not — 233 because the page group states `/DeviceRGB`,
642 an `ICCBased` space, 10 a `DeviceCMYK` page given up inside, and 66 with no page `/CS` given up
inside the page. Those pages take the four steps per painting operation (ADR 1229), which differs
from the press wherever two spot inks meet. It is the model's to build: `Compositing::Grey`,
`Calibrated` and `Additive` carry no spot colourant, and ADR 1311 section 6 is the shape. Table
275's `SeparationSimulation` is answered as met, because what it asks for is "support for simulation
separations", which this program has.
