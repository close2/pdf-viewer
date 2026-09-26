# 1329 — A spot colour passes through a group of one or three components, and §10.8.3's row is decided

Status: accepted. Session 1246.
Context: ISO 32000-2 §10.8.3, §11.3.4, §11.4.7, §11.6.6, §11.7.2, §11.7.3, §11.7.4.2, §11.7.4.3
(Table 146), §8.6.6.4, §8.6.6.5, §12.11 Table 275; traps 5, 11, 13, 41, 50, 51; ADRs 1228, 1229,
1281, 1311, 1317; `doc/questions/A100`.
Code: `crates/pdf-colour/src/colour.rs` (`Compositing::paint_beside`, `DeviceSpots::paint_process`,
`overprint_beside`, `outside_native`, `SpotSet::native`, `Conversion::beside`);
`crates/pdf-model/src/content.rs` (`ProcessPlanes`, `NotSeparated`, `process_planes`, `separate`,
`planes_of`, `separate_drawn`, `Interpreter::on_the_simulated_device`, `spots_beside`),
`colourants.rs` (`SpotColourants::readable`), `content/colour.rs`
(`spot_group_compositing`), `content/overprint.rs` (`spot_overprint_beside`),
`content/transparency.rs` (`Departure::simulated_press`, the mask compositing pair),
`content/report.rs` (`Unsupported::SeparationGivenUp`); `crates/pdf-render/src/`
`separation.rs` (`matte`, `resolve_over_device`), `display_list.rs` (`set_spot_planes`,
`shape_digest`); `crates/render-cpu/src/lib.rs`, `crates/render-raster/src/lib.rs`;
`crates/viewer-confined/src/protocol/display_list.rs`; `crates/viewer-core/src/report.rs`;
one-line hooks in `content/image.rs` and `content/pattern.rs`; `examples/spot_depth.rs`.
Tests: `crates/pdf-model/tests/spot_press.rs`, `spot_planes.rs`, `colour_paths.rs`;
`crates/render-raster/tests/separation.rs`; `pdf-render`'s `separation::tests`; `viewer-confined`'s
`a_separated_page_round_trips_to_an_equal_list`.

## 1. The reading: what a group that is not a press does to a spot colour

§11.7.3 separates the two kinds of colour before it says anything about groups. Of process colours:
"Whatever means is used to specify them, process colours may be subject to conversion to and from the
group's colour space." Of a spot colour, where the device has the colourant:

> When an object is painted transparently with a spot colour component that is available in the
> output device, that colour shall be composited with the corresponding spot colour component of the
> backdrop, independently of the compositing that is performed for process colours. A spot colour
> retains its own identity; it shall not be subject to conversion to or from the colour space of the
> enclosing transparency group or page.

The sentence names "the enclosing transparency group or page" without a condition on that group's
space, so **the space a group states decides nothing about its spot colours**. The first bullet is the
treatment this tree takes (ADR 1311): "The group shall maintain a separate colour value for each spot
colour component, independently of the group's colour space. In effect, the spot colour passes
directly through the group hierarchy to the device, with no colour conversions performed." Its words
"independently of the group's colour space" are the whole answer for a `/DeviceRGB`, `/DeviceGray`,
`CalGray`, `CalRGB` or `ICCBased` group of one or three components: the spot plane passes through it.

**What does revert, and when.** Three sentences, and none of them is the group's component count:

- the second bullet, "The spot colour shall be converted to its alternate colour space", is the other
  of the two treatments a group "should" take, and this tree takes the first everywhere but a mask;
- the mask: "spot colours shall not be available in a transparency group XObject that is used to
  define a soft mask; the alternate colour space shall always be substituted in that case";
- a **process** name in a `Separation` or `DeviceN`: "within a transparency group, this should be done
  only if the group inherits the native colour space of the output device … If any other colour space
  has been specified for the group, the Separation or DeviceN colour space shall be converted to its
  alternate colour space." NOTE 1 says why: "the device's process colour components are not
  accessible within the group".

And past the plane bound, a colourant the device does not have reverts by §8.6.6.4 (ADR 1311 section 2).

## 2. What was built

- **The process separation of such a page is the page in its own space.** §11.7.3's "additive value
  of 1.0" for every component an object does not specify is white in every one of the five spaces,
  so on the process run a colour whose space paints spot colourants alone paints the group's
  components white and keeps its alpha; an `NChannel` space paints Table 71's process space into the
  group; everything else is the space's ordinary paint (`DeviceSpots::paint_process`). The spot
  colourants ride on the run, not on the compositing (`Interpreter::spots_beside`), so they pass
  through every group whatever it composites in and every colour the run resolves reads them —
  a fill through `Compositing::paint_beside`, an image, a shading and a mesh through `Conversion`.
  A soft mask's group takes them away (`enter_mask_compositing`).
- **No process name is the device's on such a page** (`DeviceSpots::new(.., None)`), and a spot plane
  entering a group that states another space is told the same (`outside_native`), so a `DeviceN`
  naming `Cyan` beside a spot colourant reverts on every plane or on none — without that the spot
  plane painted the spot component directly while the process plane reverted the whole space.
- **Overprint beside the planes**: §11.7.4.3's second bullet on the group's components
  (`overprint_beside`): a mark specifying no process component keeps all three (Table 146's
  "Separation or DeviceN" row), any other keeps none; its `None`-ness is the spot planes', so the
  planes are one structure.
- **The spot planes are drawn on step a)'s press**, the intent's or the assumed inks'
  (`content::simulated_press`, which no longer refuses a page stating a `/CS`), and step b)'s matte
  goes under the process list **in its own components**, before its curve or cube
  (`separation::matte`); after them each spot plane is multiplied into the device colour through the
  same `process_to_flat` a press's pair takes (`resolve_over_device`). `DisplayList::set_spot_planes`
  states it and the codec carries it as three more blending shapes.
- **Two causes of giving a page up are gone.** A group drawn in a space of its own converts its
  process components and not its spot ones, so the process plane's command carries a
  `GroupBlending` and the spot plane's does not: the spot planes are held to the process plane by
  `DisplayList::shape_digest`, which is the geometry digest without group conversions — §11.7.3's
  "single shape value and opacity value" is about shape, and a conversion is colour. And on a page
  stating no `/CS` the run in the simulated press takes that press's four components as the space in
  force (`Departure::simulated_press`), because §11.4.7's initial space "is inherited from the
  native colour space of the actual, assumed or simulated output device": a group inside stating
  `/DeviceCMYK` inherits it rather than changing the space, and the plane is drawable.
- **A separation given up is on the report** (`Unsupported::SeparationGivenUp`, naming the plane or
  the colourant), because the page then takes the four steps per painting operation (ADR 1229), a
  different press from the one asked for, and a fallback nobody hears about is trap 5.

## 3. The fixtures, worked by hand

`tests/spot_press.rs`, through the assumed inks and the sRGB/D50 matrices of `colour.rs`: LogoGreen at
0.5 on a `/DeviceRGB` and a `/DeviceGray` page group is (134, 198, 183), its own colour; over an RGB
yellow under overprint (64, 146, 0); over an RGB yellow without overprint the spot is erased,
(255, 255, 0), and kept under it; on a linear-sRGB `ICCBased` group a `DeviceRGB` grey of 0.5 is
half of D50 by §11.7.2 and times LogoGreen at 1 is (21, 108, 97); a grey of 0.5 on a `/DeviceGray`
group times LogoGreen at 0.5 is (64, 98, 90); LogoGreen at half opacity inside an isolated
`/DeviceRGB` group, drawn in its own space, composited under Multiply onto process yellow on a
`/DeviceCMYK` page is (145, 187, 0). With `paint_beside` planted to the plain paint four of these
fail, and with `ProcessPlanes::Own` planted away four fail; `spot_planes.rs` holds the one process
plane and the `DeviceN` reverting on every plane, and `render-raster` draws the RGB page within two
levels of the oracle.

## 4. The crawl

`pdf-model --example spot_depth --separate` over the crawl's first ten pages, one walk behind the
lock: 88 890 documents, 374 238 pages, 8 517 naming a spot colourant — **8 506 separated, 11 given
up**, against ADR 1317's 7 566 and 951. The eleven, by their report: 2 pages whose chromatic plane
could not be drawn in the simulated press (`GHOSTSCRIPT-696877-1.pdf`, `-696890-1.pdf`, blend-mode
test files whose nested `ICCBased` groups the pair cannot convert at their `Do` — the same pages drawn
with a `/DeviceCMYK` page group report §11.6.6's departure), and 9 whose colourant's space does not
parse (`PDFBOX-2191-0.pdf` pages 2–8, `GHOSTSCRIPT-690555-0.pdf`, `GHOSTSCRIPT-687068-0.pdf`: a
`Separation` whose tint transform is `/Identity`, which is not the function §8.6.6.4 requires of a `tintTransform`). Both
were then built rather than left: a device page whose press cannot hold a group takes the device's
components as its process separation (`separate_drawn`, step a)'s "The PDF processor determines what
process colours and possible spot colours the simulated device is to have"), and a colourant whose
space does not parse has no plane (`SpotColourants::readable`) — nothing paints it through that
space, which the interpreter already replaces and reports. Checked on each of the four files after the
change: the two Ghostscript pages and `690555` are separated, `687068` and the `PDFBOX` pages name no
readable spot colourant and are the page itself. The census was not run a second time.

## 5. Cost

callgrind, one sitting behind the lock, `HEAD` and this round exported to separate trees with separate
target directories and different `md5sum`s (trap 50). Neither page names a spot colourant, which is
the point.

| | before | after |
|---|---|---|
| interpret, page 101 of the standard ×50 | 1 271 760 370 | 1 272 005 560 (+0.019%) |
| interpret, a 3000-mark `DeviceCMYK` page | 48 567 507 | 48 597 505 (+0.062%) |
| rasterise, page 101 ×20 | 5 164 989 060 | 5 166 363 440 (+0.027%) |
| rasterise, the `DeviceCMYK` page ×5 | 2 937 113 323 | 2 937 074 768 (−0.001%) |

Per function (habit 45): on the CMYK page the whole difference is `run_reader`, +36 000 — twelve
instructions per colour operator, the second argument every colour now carries (the run's spot set
beside its compositing) and its one emptiness test; `end_path` is −6 000. On page 101 it is
`show_text` +258 100 and `run_reader` +61 250 for the same reason, with the xref sort and object
drops moving ±6.7 M between symbols and cancelling. `#[inline(always)]` on `paint_beside` moved the
CMYK page by 30 instructions, so the cost is not a lost inlining (trap 51) and the attribute was not
kept. The rasteriser does not pay.

## 6. The row's word

**§10.8.3 is `implemented`.** Every requirement of the clause is executed for every page that names
a spot colourant: the control is a reader's and a requirement executed under a host's control is
executed (`A100`); step a) on the page's own press, the intent's or the assumed inks', or beside its
own one or three components; steps b) to d) on both CPU backends; `render-gpu`'s refusal by name is a
backend refusal, §11.6.7's shape; and the sixteen-plane bound is step a)'s own permission to decide
the device's colourants, with every vector mark reverting past it named. What is not `departed`
because it is not a decision: a page whose separation is still given up — none in the crawl after
this round — says so on its report. Left, and not the clause's: an image's samples or a shading's
ramp past the bound revert unnamed, a report gap on pages naming more than forty-eight colourants;
naming them needs the image's and the shading's spaces surfaced from `crate::image` and
`crate::shading` to the interpreter, which was not one small change. Table 275's
`SeparationSimulation` stays met.
