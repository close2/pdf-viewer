# 3710 — The next ten held files are read, and three of their defaults are logged or corrected

Session 2252, the silent-repair campaign's census burn-down, fifth pass. Status: **accepted**.
Context: ADR 3477, ADR 3574 (the census and its `HELD` ratchet), ADR 3565, ADR 3691, ADR 3709;
ISO 32000-2 §8.9.5.1 Table 87, §8.10.2, §12.5.5, §12.7.4.3; XFA 3.3, *Repeating Elements using
Occurrence Limits*.

## Context

After ADR 3709 the ten largest held files were `pdf-font`'s `restate.rs`, `provider.rs`,
`cff_stretch.rs`, `stretch.rs` and `collection.rs`, and `pdf-model`'s `image/mask.rs`,
`xfa/template.rs`, `rich_text/layout.rs`, `collection.rs` and `annotation.rs` (`type3.rs`, as
large, was left to the lane editing Type 3 drawing). Two Sonnet helpers read every site and wrote a
marker only where the default is a decline, a bound or a writer of this program's own; each marker
was read back here, and one — an absent XFA event `activity` read as no activity — was withdrawn,
because the event element's syntax lists `click` first and whether that is its default is not yet
read.

## Decision

- **An image's absent depth on the colour-key route** (§8.9.5.1, Table 87): `mask.rs` reads 8
  where `/BitsPerComponent` is absent, as the decode does, and now records
  `model/image-depth-absent` there too, as ADR 3691 does on the other routes; so does the grid
  arithmetic of `short_of_its_grid`. A negative depth there declines.
- **A stored appearance with no `/BBox`** (§8.10.2 makes it required): the three places
  `annotation.rs` take §12.7.4.3's box for a constructed appearance instead go through one
  function, which records `model/form-bbox-unusable`; the class's sentence now names that reading.
- **`font/collection-cut-to-one-face`** is named by its item beside the face fallback in
  `collection::extract`, whose one reading caller records it (ADR 3565).
- **An absent `occur max` duplicates `min`**, as XFA 3.3's *Repeating Elements using Occurrence
  Limits* states; it was read as 1, so `<occur min="3"/>` was called a varying occurrence and its
  template dynamic. A defect, not a repair, and a test pins it.

## Consequences

- `HELD`: `restate.rs` 9 → 0, `provider.rs` 8 → 0, `stretch.rs` 7 → 0, `collection.rs` (font)
  7 → 0, `image/mask.rs` 8 → 0, `cff_stretch.rs` 8 → 4, `xfa/template.rs` 7 → 5,
  `rich_text/layout.rs` 6 → 3, `collection.rs` (model) 6 → 3, `annotation.rs` 6 → 2.
- Held as repairs, for the rounds after: `cff_stretch.rs`'s FDSelect that does not place a glyph
  (Font DICT 0 used) and a CID-keyed font with no `/FDArray` offset; `xfa/template.rs`'s
  `fracDigits`, `leadDigits` and `occur` bounds that are no integer (the template reader holds no
  document) and the absent `activity`; `rich_text/layout.rs`'s `/DR` face that does not load, the
  absent `text-valign` and the absent `Tf` size; `collection.rs`'s `/Position` outside 0 to 100 and
  a folder `/ID` that is no integer; `annotation.rs`'s `/Ff` of another type falling to the parent
  and a fixed print `/H` or `/V` that is no number.
