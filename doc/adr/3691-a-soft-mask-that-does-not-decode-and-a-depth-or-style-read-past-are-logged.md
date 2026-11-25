# 3691 — A soft mask that does not decode, and a depth or a style read past, are logged

Session 2247, the silent-repair campaign's census burn-down, fourth pass. Status: **accepted**.
Context: ADR 3477, ADR 3574 (the census and its `HELD` ratchet), ADR 3669 (the list this takes
up), ADR 1441 (a substitute face's style); ISO 32000-2 §8.9.5.1 Table 87, §9.8.1 Table 120,
§11.6.5.2 Table 143.

## Context

Round 2242 left five files held, soft masks first: a mask whose data did not decode left its image
drawn opaque with nothing said, and an absent `/BitsPerComponent` was read as 8 on three routes.
`pdf-font`'s `substitute.rs` read a `/FontWeight` outside 1 to 1000 or a `/FontStretch` Table 120
does not name as absent, also in silence.

## Decision

- **`model/soft-mask-undecodable`** (§11.6.5.2): `mask_plane`, the one door every route that
  decodes a mask goes through, logs it when the decode fails — except where the failure is this
  reader's (`ImageError::TooLarge`, a decoder that could not be reached). The image is still drawn
  without the mask: an image present and opaque is closer to the page than one dropped.
- **`model/image-depth-absent`** (§8.9.5.1): where no codec states the depth and the entry is not
  a non-negative integer, the general decode (`dispatch`) and the device-scale mask route read 8,
  as before, and log it; so does the eight-bit grey fast path for a mask, except behind
  `JPXDecode`, whose depth Table 87 leaves to the codestream.
- **`font/descriptor-style-unusable`** (§9.8.1): logged where `Style::derive` meets either entry
  stated as no value the table admits. The normal weight and width where nothing states one are
  ADR 1441's documented end of the order, not a repair, and are marked so.
- The remaining sites of `soft_mask.rs` are a grid read as 0 only to size a combination or word a
  report — the mask's own decode refuses a grid that is not there, and logs that — and are marked.

All three classes are recorded at `Site::WholeFile`, as stream damage is: the image decoder is
handed a stream without its object. `pdf_archive::repairs::MAPPING` maps them to section 5.1 alone.
They are logged where an image is decoded or a face is chosen, so a conversion meets them only where
it draws a page.

## Consequences

- `HELD`: `image/soft_mask.rs` 8 → 0, `substitute.rs` 3 → 0, `image/dispatch.rs` 4 → 2.
- Left, in order: `rich_text/markup.rs` (markup that does not parse), `jpeg2000.rs` (short boxes),
  `geospatial/system.rs` (a nameless system and datum) — each needs its reader to hold the document
  or to return what it read past, which none does yet.
