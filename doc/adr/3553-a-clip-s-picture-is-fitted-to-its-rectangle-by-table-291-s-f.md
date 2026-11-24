# 3553 — A clip's picture is fitted to its rectangle by Table 291's `/F`

Session 2213. Status: accepted. Context: RFC 0015 section 6; ADRs 3350, 3538, 3545; ISO 32000-2
§13.2.5 Table 291, §13.2.2.

## Context

Table 291's `/F` is "the manner in which the player shall treat a visual media type that does not
exactly fit the rectangle in which it plays", in six values: 0 meet, 1 slice, 2 fill, 3 scroll
("A scrolling user interface shall be provided if the media rectangle is wider or taller than the
play rectangle"), 4 hidden, 5 "Use the player's default setting (author has no preference)", the
default. "An unrecognised value shall be treated as the default value if the entry is in a BE
dictionary. If the entry is in an MH dictionary and it has an unrecognised value, the object shall
be considered non-viable." Round 2211 left every picture filling the rectangle and §13.2.5
`partial` for it.

## Decision

1. `pdf_model::media::Fit` carries the value in `Playing::fit`, across the confined wire as its
   integer. An unrecognised value is an `MH` refusal and a `BE` default, as the table says.
2. **Scroll is not provided.** No window has a scrolling interface over a screen annotation, so
   `/F 3` in `MH` makes a clip with pictures non-viable — whether the picture overflows is known
   only after decoding, so the requirement cannot be shown met at judgement — and in `BE` is set
   aside, said, and shown as hidden: unscaled and cut, all of `/F 3` but its interface. A sound
   honours every value: it is fitted to nothing.
3. **Geometry, one function for every window**: `viewer_host::video::placed` answers the part of
   the picture shown, in samples, and the quadrilateral it covers. Meet and 5 take the largest
   single scale that shows all of it; slice the smallest that covers the rectangle; fill one scale
   a side; hidden and scroll none. The device layer draws that part (`raster_gpu::Layer::within`)
   under `Placed::affine`; the processor window and GTK draw the part cut out (`Placed::part_of`).
4. **Three choices the table leaves open, made and named.** The player's default (5) is meet: it
   distorts nothing and hides nothing. "Not scaled" is one sample a window pixel, since the table
   names no unit and the NOTE's SMIL `hidden` is a pixel's. A picture that does not fill, or is
   cut, stands at the rectangle's top-left corner, SMIL's default registration. Samples are taken
   as square; the stream's sample aspect ratio is not read.

## Consequences

- §13.2.5 stays `partial`, now on `/F 3`'s interface alone.
- The device layer's placement mapped the picture's texel space as though it were a unit square,
  so on the device a picture was drawn as its first sample stretched over the rectangle; the
  placement is now the sample-space affine, tested by its corners.
