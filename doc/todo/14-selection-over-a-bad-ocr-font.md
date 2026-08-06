# Selection over a badly built OCR font

Status: **raised by the project owner in the three-hundred-and-sixty-second session**, from a
failure they have seen in other viewers: over a scanned page's OCR layer, no blue highlight
appeared and the cursor had to be aimed half a line below the text to select anything. Two of the
four risks below are already answered in this tree; **one is real and one is unverified**.
Priority: 14 — a defect, and one no gate in this tree can see
Clauses: §9.2.2, §9.3.5 (`Ts`), §9.3.6 (Table 106's rendering modes 3 and 7), §9.4.4, §9.6.2
(`/Widths`, `/MissingWidth`), §9.8.1 (Table 122's `/Ascent` and `/Descent`)
Code: `crates/pdf-font/src/lib.rs::vertical_extent`, `crates/pdf-model/src/content.rs::glyph_quad`,
`crates/viewer-core/src/select.rs`

## Two different "dimensions", and only one of them can move a glyph

For a simple font the *font program's* metrics position nothing. §9.6.2's displacement comes from
the font dictionary's `/Widths`, falling back to `/MissingWidth` or to the standard-14 metrics
where the array is absent; the glyph program's own advance is not consulted. So a badly built
embedded font is harmless for layout, and what breaks a page is a wrong `/Widths` — which is
exactly what sloppy OCR producers emit.

That splits the failure in two, and only the second matches the symptom:

- **Horizontal (wrong `/Widths`).** The glyphs are drawn in the wrong places *and* the selection
  quads come from the same arithmetic, so the two stay consistent: the highlight is ragged and
  does not match the scan underneath, but it covers what the text layer says. **There is no second
  calculation in this tree to get out of step** — `select.rs` takes `Placed::quad` from the
  interpreter's own positioning.
- **Vertical, and this is the one to chase.** A text object states a baseline, a size, a text
  matrix, `Tz`, `Ts` and `TL`. **It never states a line box.** The height of a selection rectangle
  is not in the file, so a viewer invents it, and the usual source is Table 122's `/Ascent` and
  `/Descent`. OCR output is notorious for writing zeroes there, or values in the wrong units. A
  viewer that trusts them gets a zero-height or badly offset rectangle — no visible highlight, and
  a caret that registers half a line off.

## What this tree already does, verified rather than assumed

`pdf_font::vertical_extent` reads Table 122's two entries and **falls back to the em box, 1 above
the baseline and 0 below, unless both are present and `ascent > descent`**. Its own comment makes
the argument: the em box is a *defined* quantity rather than a guess. So the two commonest OCR
shapes — both entries absent, both zero — already produce a full-em box rather than nothing.

Table 106's invisible modes are also already right, which is the other half of the owner's note:
mode 3 and mode 7 accumulate readback and quadrilaterals like any other mode, because §9.3.6
requires the advance — "[t]he e and f components of Tm shall be updated for each glyph drawn when
using text rendering mode 3 or 7 in exactly the same way as would be done for other text rendering
modes". Nothing in the selection path filters on rendering mode. **An OCR layer is only useful
because it is selectable, and a viewer that lost mode 3 would lose every scanned document
silently** — worth a test naming that, since nothing asserts it today.

## The real gap: the guard checks ordering, not plausibility

`ascent > descent` is the whole test. It rejects `(0, 0)` and it **accepts** these:

| descriptor | extent | what the highlight does |
|---|---|---|
| `/Ascent 0 /Descent -200` | `(0.0, -0.2)` | a box entirely *below* the baseline — the owner's "half a line below" exactly |
| `/Ascent 100 /Descent -50` | `(0.1, -0.05)` | 0.15 em tall, a sliver over 1-em glyphs |
| `/Ascent 3000 /Descent -1000` | `(3.0, -1.0)` | four em tall, swallowing three lines above and below |

Each of those is a file stating a number no glyph in it reaches, which Table 122's own definitions
forbid — "[t]he maximum height above the baseline reached by glyphs in this font" is a *measurement
of the font*, so a font whose glyphs are 0.7 em tall and whose `/Ascent` is 0 has stated something
untrue. **A plausibility band is therefore derivable rather than invented**: the entries are in
glyph space, §9.2.2 fixes the em at 1000 units, and a value outside — say — a tenth to twice the em
is not a measurement of any face.

The degenerate `/Widths` case belongs here too and is the one that produces *no* highlight rather
than a misplaced one: OCR that writes every width as 0 and positions each glyph with an explicit
`TJ` offset. Then `advance` is 0, `glyph_quad`'s corners at `(0, ·)` and `(advance, ·)` coincide,
and every quad is a zero-width sliver — `quads_for`'s run merge joins them into a run of no width
at all. §9.6.2 does not forbid a zero width, so this is a **choice** to make and to write down, not
a clause to obey.

## What a round taking this owes, in order

1. **The census first**, which is `doc/todo/13`'s rule and what made §10.5 a small change: how many
   of the 974 state a descriptor whose `/Ascent` or `/Descent` is outside a plausible band, and how
   many state a `/Widths` that is all zeroes. Neither number is known, and the answer decides
   whether this is one file or a population. `examples/transfer_function_census` is the shape.
2. **A fixture and a gate**, because *no instrument in this tree can see this defect*. The text gate
   compares extracted characters and never asks where the quads are; the oracle compares pixels and
   an invisible OCR layer draws none. **Selection geometry over an OCR layer is a blind spot
   between the two gates**, and a wrong-height highlight would ship without a single number moving.
   That is the strongest argument in this file for doing it at all.
3. **Then the band**, applied in `vertical_extent` alone — one function, one place, with the
   fallback it already has.

## What not to do

- **Do not move the fallback into `select.rs`.** The extent is a property of the font, read once
  per show operation; a consumer-side correction would be a second calculation, which is the thing
  the horizontal case is safe from precisely because it does not have one.
- **Do not use `/FontBBox`.** Table 122 defines it as the glyph bounding box "expressed in the
  glyph coordinate system", and for a Type 3 font that system is the `/FontMatrix`'s — a different
  scale per file. `/Ascent` and `/Descent` are in a fixed 1/1000 em by §9.2.2, which is why they
  are what this reads.
- **Type 3 needs checking rather than assuming.** `interpret` gives a Type 3 font the em box
  outright, on the ground that Table 122 requires neither entry of one; whether that box then goes
  through the `/FontMatrix` on its way to the quad is **unverified** and is the second thing to
  read.
