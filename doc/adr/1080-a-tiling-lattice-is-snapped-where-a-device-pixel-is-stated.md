# ADR 1080 — A tiling lattice is snapped where a device pixel has been stated, and nowhere else

## Status

Accepted, 2026-09-15. Session 1066.
Crates: `crates/pdf-render` (`src/lattice.rs`), `crates/pdf-model` (`src/content/pattern.rs`,
`src/content.rs`). Clause: ISO 32000-2 §8.7.3.1, Table 74's `/TilingType`.
`§N` is ISO 32000-2 and nothing else; a section of this document is named in words.

## Context

Table 74 makes `/TilingType` "( Required )" of every Type 1 pattern dictionary and calls it "[a]
code that controls adjustments to the spacing of tiles relative to the device pixel grid". Code 1
wants cells "spaced consistently - that is, by a multiple of a device pixel", achieved if need be
by "making small adjustments to XStep , YStep , and the transformation matrix", with "[t]he amount
of distortion shall not exceed 1 device pixel". Code 2 wants the opposite trade: "[t]he pattern
cell shall not be distorted, but the spacing between pattern cells may vary by as much as 1 device
pixel". Code 3 is code 1 "but with additional distortion permitted to enable a more efficient"
tiling.

ADR 1031 read the entry, counted the population and declined to *report* the departure, on a
measurement: over `doc/pdf.js` **392** dictionaries ask for code 1, **557** for code 3 and **5**
for code 2, so a report would have fired on 99.5% of patterned pages and taken each of them off
the oracle's judged list to say so (trap 39, trap 11). It closed by naming what honouring the
entry would actually take — "a rasteriser that snaps a lattice to the device pixel grid, because
only a backend knows the pixel" — and left the row `partial`.

**The second half of that sentence is what this ADR revises.** A backend is not the only thing in
this tree that knows a device pixel, and it is the wrong one: by the time a display list reaches a
rasteriser the tiling is gone. `pdf-model` expands a tiling during interpretation — the cell is run
once and `pdf_render::Cell::repeat` appends a displaced copy per site (ADR 0430) — so a backend
sees translated commands and no lattice at all. Putting the snap in a backend would mean carrying
the lattice, the cell's clips and its soft masks through the display list and rebuilding them per
site in three rasterisers, which is where cross-backend disagreement comes from (the defect
`render-raster --test corpus` exists to catch).

The thing that *does* know the device pixel, and is already read during interpretation, is
`ViewState`'s magnification: "[h]ow large the page is being drawn, in logical pixels per default
user space unit", put there by §12.5.3's `NoZoom`, which makes an annotation's placement a
function of the same number. `Open::magnification` computes it in device pixels, and the viewer
sets it on every frame.

## Decision

**The lattice is snapped in the interpreter, from `ViewState`'s magnification, and the arithmetic
lives in one function all three backends can see.**

### 1. The arithmetic — `pdf_render::snap_lattice`

§8.7.3.1 puts site (*i*, *j*) at *i* × `/XStep` and *j* × `/YStep` in pattern space, so the lattice
is two *step vectors*. Carried through the pattern matrix's linear part and the device scale `s`
they are `u = s·(a·XStep, b·XStep)` and `v = s·(c·YStep, d·YStep)` device pixels. "[S]paced
consistently … by a multiple of a device pixel" is then one sentence of arithmetic: **round each
component of `u` and of `v` to the nearest whole pixel**. Every site is a whole number of `u` and
`v` from the key cell, so rounding those two puts every cell on the same sub-pixel phase however
the matrix rotates or shears.

What pays for it is the matrix, which is the clause's own remedy: the adjusted matrix is the one
sending `(XStep, 0)` and `(0, YStep)` to the rounded vectors. `/XStep`, `/YStep` and the matrix's
**translation** are left exactly as the file states them — §8.7.3.1 makes the translation the
tiling's phase, and the clause asks for consistent spacing rather than for a place to start.

Codes 1 and 3 get this. Code 3's extra permission is declined: a permission is not a requirement,
and taking it would buy speed this tree does not need at the price of a wider distortion.

### 2. The bound is a condition, not a hope

Each component moves by at most half a pixel, so the *step* is always inside Table 74's tolerance.
The **cell** need not be: `/BBox` may be many steps across (Table 74's NOTE 2), and a cell *k*
steps wide is distorted by up to *k* half-pixels. So the distortion is measured — the furthest any
corner of the cell moves relative to the cell's own anchor — and a snap that would exceed one
device pixel **is not taken**. Nor is one whose step vector rounds to the origin, or whose two
vectors round onto one line: each would be a different tiling rather than a spaced one. The
pattern then keeps the geometry the file states, which is code 2's tolerance; a cell that cannot
be spaced consistently *without* breaking the clause's own bound is a cell the clause does not ask
to be spaced consistently.

### 3. Where the device pixel comes from, and what follows when none was stated

`Interpreter::lattice` asks `ViewState::magnification`. `None` is *nobody has said* — not 1.0 —
which is what the corpus gate, the oracle, `raster_golden`, `pdf-transform` and every other
non-viewer caller mean. A caller drawing at no particular resolution has no grid to snap to, so
the tiling is placed at the geometry the file states, which is code 2's treatment. **Those gates
therefore do not move**, and that is the correct answer rather than a convenient one: the
adjustment Table 74 describes is defined only relative to a device pixel grid, and a call that
names none has not asked for one.

A `/TilingType` that is absent or outside 1–3 is given code 2's treatment as well. That is a
**choice** and not a reading: Table 74 states no default, and of the three codes code 2 is the only
one that requires nothing of the processor, so it is the one that leaves the geometry the file
*did* state alone.

### 4. A snapped lattice makes the page depend on the magnification

`Interpretation::view_dependent` already means "the magnification can change this page's marks",
and a constant-spacing tiling placed under a stated magnification is the second thing that makes
it true — the first being §12.5.3's `NoZoom`. So the flag is raised, and `Open::reinterpret`
re-interprets such a page on a zoom rather than re-rasterising its list. It is raised whenever such
a tiling is *placed*, not only where the snap was taken: whether it is taken is itself a function
of the magnification.

**And the content half cannot be replaced.** ADR 0777's `Replacement` keeps the content and re-runs
§12.5.3's annotation pass at the new magnification; that is sound only while the content is
magnification-independent, which a snapped lattice is not. `interpret_into` therefore withholds the
checkpoint from such a page, and `view_dependent` and `replacement.is_some()` stop being the same
question — `tests/replacement.rs` and `examples/replacement_census.rs` now assert the direction
that still holds.

## Consequences, and what was measured

- **`examples/tiling_type_census`** now measures the lattice as well as counting the codes: over
  `doc/pdf.js`, how many pages' display-list geometry a stated magnification moves, on how many a
  constant-spacing tiling was placed at all, the share of pixels each mover moved, and a **control**
  — the same comparison over documents holding no code 1 or 3 pattern. The control is **not**
  silent, which is the point of having it: §12.5.3's `NoZoom` is the other thing a stated
  magnification places, and without the control a round would read its contribution as the
  lattice's. What separates them is a third column counting only pages that carry no such
  annotation, where nothing but the lattice can have moved (trap 13). The numbers are the
  command's, not this file's (`CLAUDE.md`, *Where knowledge lives*).
- **A good share of real patterns already state a lattice on the grid**, and that is worth knowing
  rather than assuming: an `/XStep` of 20 under an identity matrix is a whole number of pixels at
  every integer magnification, so for those the snap is arithmetic that changes nothing. The census
  says what share. It is also, in hindsight, part of why ADR 1031's departure was worth so little —
  the rest being that most of the movers move a fraction of a pixel.
- **`DisplayList::geometry_digest` cannot be used to ask whether a page moved**, and finding that
  out is what the trap-1 look cost. It hashes each command's variant, its clip and mask
  identifiers, its blend mode and the *number* of segments in its path — not the transform on it —
  so a lattice that relocated every site hashes the same. `issue16038.pdf` page 1 moves an eighth
  of its pixels with the two digests equal. The digest is sound for the pair it exists for, where
  the geometry is identical by construction; its doc comment now says so and names the witness,
  and the census compares whole lists.
- **Cost.** One dictionary lookup and one branch per tiling pattern, and where a magnification was
  stated, a dozen floating-point operations — once per pattern, never per site. No gate's pages/s
  moves, because no gate states a magnification.
- **What a later round must not do** is move the snap into a backend "because the clause says
  device pixel". It says device pixel *grid*, and a grid is a scale; the scale is known where the
  page is interpreted and the lattice is gone by the time a rasteriser runs. Moving it would cost
  the display list's flatness, the three backends' agreement, and ADR 0430's one-interpretation
  cell.

## Alternatives rejected

- **A `Command` the backends expand.** The lattice, the cell's clips and its soft masks would all
  have to be rebuilt per site in each of `render-cpu`, `render-raster`'s scene and `render-gpu`.
  Three rasterisers reimplementing one placement is exactly what `pdf-render`'s opening paragraph
  exists to prevent.
- **Snap at a fixed nominal scale — treat `None` as 1.0.** It would move `raster_golden`, the
  oracle and the corpus gate to a grid none of them draws on, and it would make a page rendered at
  150 dpi carry a lattice snapped for 72.
- **Let code 3 take its extra distortion.** The permission is for speed. This tree's tiling cost is
  in the copies, not in the arithmetic, so there is nothing to buy with it.
- **Report the departure instead** — ADR 1031 measured that and declined it, and nothing here
  re-derives the report. What it revises is only that ADR's closing guess about *which* part of the
  tree knows a pixel.
