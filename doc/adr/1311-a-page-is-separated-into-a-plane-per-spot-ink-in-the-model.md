# 1311 — A page is separated into a plane per spot ink, in the model, and sixteen planes is the bound

Status: accepted. Session 1237.
Context: ISO 32000-2 §8.6.6.4, §8.6.6.5, §10.8.2, §10.8.3, §11.3.4, §11.7.3, §11.7.4.2, §11.7.4.3,
§14.11.5, Table 275; traps 6, 11, 38, 41; ADRs 0262, 1158, 1228, 1229, 1254, 1281;
`doc/questions/A100`.
Code: `crates/pdf-colour/src/colour.rs` (`Plane::Spot`, `DeviceSpots`, `PROCESS_COLOURANTS`, the
names on `ColourSpace::Separation` and `::Simulated`), `crates/pdf-model/src/colourants.rs`
(`MAX_SPOT_PLANES`, `Separation`), `crates/pdf-model/src/content.rs` (`separate`,
`simulated_press`), `content/colour.rs`, `content/overprint.rs`, `content/report.rs`,
`soft_mask.rs`, `requirements.rs`; one-line hooks in `content/transparency.rs`, `path.rs`,
`text.rs`, `image.rs`; `examples/spot_depth.rs`.
Tests: `crates/pdf-model/tests/spot_planes.rs`.
Documents: §10.8.3's, §8.6.6.4's and §8.6.6.5's ledger rows, `doc/todo/23`, `doc/todo/65`,
`doc/state-of-play.md`.

## 1. What this is: stage two of ADR 1281's plane per spot ink

ADR 1281 enumerated a page's spot colourants and wrote the page's interpretation as a sequence of
planes. This is the model: under the reader's simulation (ADR 1228; a requirement executed under a
host control is executed, `A100`) a page naming a spot colourant is processed, in §10.8.3's step
a)'s words, "as if separations were to be created for a simulated device that supports subtractive
process colourants and possibly spot colours" — once per plane of that device — and the planes are
held on `Interpretation::separation`. **No backend draws them**, and that is the stage's shape
rather than a gap in it: the page a backend draws is the one it drew before, so every gate is
unmoved by construction, and the planes are tested where they are made.

## 2. The census, and the bound

`pdf-model --example spot_depth` over the crawl (`find -L corpus-cache -name '*.pdf'`, the first
ten pages of each document, one walk behind the lock, 83 s): 88 890 documents opened, 374 238 pages
read. 8 517 pages in 2 931 documents name a spot colourant: 5 612 name one, 1 773 two, 629 three,
223 four, 123 five, and the count falls away from there — 18 pages name eight, one names 34, one
(`2268013.pdf`) 49. Past that the only pages are six files of one Ghostscript bug report
(`GHOSTSCRIPT-701369-0.zip-*.pdf`) naming 1090 apiece.

The standard states no bound (trap 38): §8.6.6.4 allows names "subject to implementation limits"
and §8.6.6.5 a `DeviceN` of "an arbitrary number of colour components". What a plane costs is one
more interpretation of the content stream, and in stage four one more raster, so the bound is a
budget set against what documents do, on `MAX_PRESSES`' shape (ADR 1254): **`MAX_SPOT_PLANES`
is sixteen, forty-eight colourants**, eighteen interpretations with the process pair. It carries
every page of the crawl but the 49-colourant one and the six stress files. A page past it is not
cut silently: its first forty-eight colourants have planes, and a mark in any other reverts to its
alternate (§11.7.3's second bullet) and is named on `Separation::without_a_plane` — per painting
operator, for the parts it paints, never because the page names it (trap 11).

## 3. The model

- **`ColourSpace` carries its colourant names**: `Separation { names }` for a `Separation`'s one
  name and a `DeviceN`'s array, `Simulated { names }` for an `NChannel` space under the
  simulation. §8.6.6.4 has the reader "determine whether the device has an available colourant
  corresponding to the name of the requested space", which cannot be asked without the name.
- **`Plane::Spot(n)`** carries colourants `3n` to `3n + 2` of `DeviceSpots::names`, stored as
  §11.3.4's additive complement like the process planes.
- **`DeviceSpots`** rides in `Compositing::Subtractive(plane, press, spots)`: the colourants with a
  plane, the ones past the bound, and the device's own press. Empty on every run a backend draws,
  and `paint` then takes the path it took before behind one discriminant test.
- **`paint` is §11.7.3's paragraph**: "In effect, every object paints every existing colour
  component, both process and spot. Where no value has been explicitly specified for a given
  component in a given object, an additive value of 1.0 (or a subtractive tint value of 0.0) shall
  be assumed." One classification, `SpotSet::named`, read by the paint and by the overprint
  question alike (trap 6): a `Separation` or `DeviceN` whose colourants are all available paints
  them directly, the tint transform ignored — §8.6.6.4's "the PDF processor shall ignore the
  alternateSpace and tintTransform parameters" — with `None` discarded; `All` paints every
  process component and every spot colourant; an `NChannel` space is evaluated per component; and
  every other colour is its four process components and no spot ink. A process name is available
  only where the run composites in the device's own press, which is §11.7.3's condition: "within a
  transparency group, this should be done only if the group inherits the native colour space of
  the output device".
- **The press.** A page whose group composites in four components is separated in that press; a
  page whose group names no `/CS` is separated on the press step a) says to consult — "A default
  DestOutputProfile , if available for a subtractive device" — which is §14.11.5's intent, or ADR
  0263's assumed inks (`content::simulated_press`).
- **Groups pass the spot planes through.** §11.7.3 offers two treatments with a "should"; this tree
  takes the first, "the spot colour passes directly through the group hierarchy to the device, with
  no colour conversions performed": on a spot plane a group's `/CS` changes nothing
  (`Interpreter::spot_group_compositing` answers inherit), and on a process plane a nested press
  carries the spot colourants with it. **Soft masks carry none**: "spot colours shall not be
  available in a transparency group XObject that is used to define a soft mask".
- **Overprint, one plane wider.** `DeviceSpots::overprint` is §11.7.4.3's two bullets with the spot
  components the first names — "For spot colour components, the value shall always be 𝐶𝑏" — and
  the second's "𝐶𝑠 for all colour components specified in the current colour space, otherwise 𝐶𝑏",
  so under either overprint mode an unspecified spot component keeps its backdrop. Whether the
  mode is special is decided per mark and never per plane, so every plane of a page is one
  structure.
- **§11.7.3's single shape and opacity** — "Only a single shape value and opacity value shall be
  maintained at each point in the computed group results; they shall apply to both process and
  spot colour components" — is what the geometry digest checks across every plane; a plane that
  drew a different structure gives the separation up.

## 4. Why the planes are not in the display list, and what that does to `raster_golden`

`raster_golden` digests the list's `Debug` (trap 41). Stage two puts nothing on `DisplayList`:
the planes are a model value beside it, so `raster_golden` held 974 and moved 0, with the
simulation off as it runs and with it on for every page naming no spot colourant. Stage three owes
the list field, and it must appear only on a page with a spot plane so that every other row keeps
its digest.

## 5. Cost

callgrind, one sitting, two binaries from `HEAD` exported and `HEAD` with this round's hunks
alone, `examples/callgrind_interpret`. Page 101 of the standard ×50, the device path:
1 250 231 227 → **1 249 666 364, −0.045%**. A 3000-mark page whose group is `/DeviceCMYK`, the
two-plane path with no spot colourant: 46 607 061 → **46 532 370, −0.16%**. Per function the
changes are `show_text` −652 400 on page 101 and `end_path` −66 000 on the CMYK page, with glibc's
`memmove` moving ±263 636 on page 101 by alignment, as in ADR 1281.

**The first build paid, and the diff per function found why** (habit 45). `overprint_blend` had
become "not subtractive, or mode not 1 and no spot plane", asked inline twice per show-text
operator: **+1 244 850, +0.10%**, all of it in `show_text`. The fast test is now the overprint
parameter for the part, false on almost every page and a condition under either mode, and the
mode moved into the cold half. A per-glyph report hook cost a further share and moved to one test
per operator.

## 6. What is left

- **Stage three**, the vocabulary: `DisplayList` and `GroupBlending::FourComponents` carrying spot
  lists and each colourant's tint-to-flat-XYZ curve, and `viewer-confined`'s wire.
- **Stage four**, the backends: `render-cpu` multiplying each spot separation in (steps b) to d)),
  `render-raster` and `render-gpu` refusing by name; §11.7.4.2's rule that only a separable,
  white-preserving blend mode applies to a spot colour, Normal standing in for any other, which is a
  substitution on the spot plane's
  compositor and so the backend's; and `without_a_plane` joining the page's report once the
  separation is what is drawn.
- **Beside them**: a page whose group composites in one or three components, or that holds a group
  doing so, is not separated — those `Compositing` variants carry no spot colourant, and painting a
  group's spot marks in their alternate as well as on their planes would count them twice; a group
  in a press of its own inside a separated page fails the digest, its process plane carrying a
  conversion out the spot plane does not; and an image's samples or a shading's ramp in a colourant
  past the bound revert unnamed.
