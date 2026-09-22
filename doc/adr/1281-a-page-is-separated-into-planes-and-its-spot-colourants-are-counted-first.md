# 1281 — A page is separated into planes, and its spot colourants are counted before the first mark

Status: accepted. Session 1222.
Context: ISO 32000-2 §10.8.1, §10.8.2, §10.8.3, §8.6.6.4, §8.6.6.5 (Tables 70, 71), §11.3.4,
§11.4.7, §11.7.3, §11.7.4.3, §12.5.5; traps 5, 11, 38, 41; ADRs 0262, 1228, 1229.
Code: `crates/pdf-model/src/colourants.rs` (new), `crates/pdf-colour/src/colour.rs` (`Plane`),
`crates/pdf-model/src/content.rs` (`in_planes`), and the `Half` → `Plane` sites in
`soft_mask.rs`, `content/transparency.rs`, `content/overprint.rs`, `tests/image_reuse.rs`.
Tests: `crates/pdf-model/tests/spot_colourants.rs`.
Documents: §10.8.3's ledger row, `doc/todo/23` ("A plane per spot ink"), `doc/todo/65`.

## 1. What this is: stage one of §10.8.3's spot plane, and its acceptance test

`doc/todo/23` priced the plane per spot ink at `ceil(S / 3)` rasters beside the chromatic and black
ones, one interpretation per plane, with the colourants enumerated from the page's resources before
the first mark. This is the first stage: **the enumeration, and the page's interpretation written
as one run per plane rather than as a hard-coded pair.** `S` is zero on every path that draws, so
the acceptance test is that nothing moved — `raster_golden` and `render-raster`'s corpus gate are
unchanged row for row.

## 2. The enumeration

`colourants::spot_colourants` walks the page's resources — `/ColorSpace`, `/Pattern`, `/Shading`,
`/XObject`, `/Font` — and beneath them a form's, a tiling pattern's and a Type 3 font's own
resources, a shading pattern's shading, an image's `/ColorSpace`, an `Indexed` base and a `Pattern`
space's underlying space, and each annotation's normal appearance, which §12.5.5 draws onto the same
page. It admits §8.6.6.4's `Separation` name and §8.6.6.5's `DeviceN` names less four kinds:

- `None`, which "shall not produce any visible output";
- `All`, which "shall refer collectively to all colourants available on an output device" — every
  plane rather than one — and which a `DeviceN` may not state at all;
- `Cyan`, `Magenta`, `Yellow` and `Black`, "reserved to name the process colourants of a CMYK
  device", which the simulated device of §10.8.3 step a) is;
- an `NChannel` space's Table 71 `/Components`, because "[a]ny component not specified in the
  process dictionary shall be considered to be a spot colourant" draws the line by that
  dictionary and not by the name. A plain `DeviceN`'s process dictionary folds nothing, since the
  clause gives it meaning under the subtype alone.

A `/Colorants` entry the `names` array does not use is not the page's: the dictionary "may also
include additional colourants not used by this colour space". **A soft mask's group is not
walked**, on §11.7.3's sentence that spot colours "shall not be available in a transparency group
XObject that is used to define a soft mask", so `/ExtGState` is not a road in.

The walk over-reads rather than parsing the content stream, because the answer is needed before the
stream runs and an unused colourant costs a plane nothing draws on where a missed one costs a
colourant its separation. It is depth-first in the order each object states its entries, so the
planes come out in the same order on every run; each indirect object is entered once per role,
which ends a cycle; and it runs from an explicit stack, so a chain of forms each naming the next
costs heap rather than the thread's stack. **No bound on `S`**: trap 38's question has the answer
*the standard states none* — names "subject to implementation limits", a `DeviceN` of "an arbitrary
number of colour components" — and the bound belongs where planes are made, in section 5.

## 3. The planes, and what stage two adds to them

`pdf_colour::colour::Half` is `Plane` now, with `Plane::PROCESS` the two §11.4.7's four process
components take and `Plane::COLOURANTS` the three a plane carries; `SpotColourants::plane_count` is
`PROCESS.len() + ceil(S / 3)`. `content::in_planes` interprets `Plane::PROCESS` in order, stops at
the first run that cannot be drawn in the blending space, checks every run's geometry digest against
the first and hands the companions to `DisplayList::set_blending` — the pair ADR 0262 built, with
the two written as a sequence. Stage two is then these, each local:

- **`Plane::Spot(index)`**, and a device's spot colourants carried beside the press in
  `Compositing::Subtractive`. `paint` for it is §11.7.3's paragraph as written: "every object paints
  every existing colour component", "the named spot colour shall be painted as specified and all
  other components (both process colours and other spot colours) shall be painted with an additive
  value of 1.0". A mark in a colourant the device has paints 1.0 on the process planes; `All`
  paints its tint on every plane.
- **`ColourSpace` carries its colourant names.** It does not today, and a mark cannot be put on a
  plane without them.
- **Groups pass spot planes through**, §11.7.3's first bullet: the spot run of a group is its
  elements in that plane, with no conversion and no black run. Soft masks never get one.
- **The render side**: `DisplayList` and `GroupBlending::FourComponents` hold their spot lists and,
  per colourant, the separation's curve from tint to flat XYZ against the white matte; `render-cpu`
  resolves the process planes as today and multiplies each spot separation in, which is §10.8.3
  steps b) to d) and `colour::simulate`'s arithmetic (ADR 1229); `render-raster` and `render-gpu`
  refuse a list carrying spot planes by name; `viewer-confined`'s wire carries them.

## 4. Two things this stage deliberately does not do

**No report.** A report naming the colourants that have no plane, raised under the preference, was
built and taken out, on trap 11: "this page names a spot colourant" is far wider than "this page
draws differently for want of a plane". An opaque mark with overprint off knocks every separation
out on a press too, and one painting operation's colourants are already combined by ADR 1229's
route — the fixtures `tests/colour_paths.rs` holds for it would all have read as incomplete. The
precise condition is per mark (a spot colourant composited under overprint, or with an alpha, blend
mode or mask that makes the backdrop show), and stage two has it where it paints.

**No render-side field.** `raster_golden` digests the display list's `Debug`, so a field every
page carries empty moves every row that composites in four components list-only (trap 41), and a
vocabulary no path fills has no user a backend can be tested against. It arrives with its first
user, and that round counts the rows it moves.

## 5. Cost

Measured under callgrind in one sitting, two binaries from one tree — `HEAD`, and `HEAD` with this
round's hunks alone applied — over `examples/callgrind_interpret`. Page 101 of the standard, fifty
interpretations, on the device path, which runs none of the new code: 1 250 037 757 and
1 250 038 551 before, 1 250 082 480 after. A 3000-mark page whose group is `/DeviceCMYK`, through
the two-plane path: 86 772 513 before, 87 132 711 after, **+0.41%**. **Not one function of this
tree executes an instruction more on either page**: the whole difference is glibc's `memmove`,
called from `Document::get_key` 254 110 times in both binaries, at 22.05 instructions a call before
and 23.46 after — the alignment of the copies moving with the frame layout, which is also why the
device page, untouched by the change, moved by the same mechanism. Stage two's number is the
spot-plane path's own: one interpretation per plane, `2 + ceil(S / 3)` of them.

## 6. What is left, and its price

Stage two (the model: `ColourSpace`'s names, `Plane::Spot`, `paint`, groups, overprint one plane
wider), stage three (the render vocabulary and the wire), stage four (`render-cpu`'s resolve and the
two refusals), and a census sizing `S` over the crawl before the bound in stage two is chosen, on
`MAX_PRESSES`' shape. Under the preference a page not composited in four components has no process
planes to multiply with, and does not need them: step c) multiplies separations, and the device
page's own composite converted to flat XYZ is its one process separation. `doc/todo/23` carries the
prices.
