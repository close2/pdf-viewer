# 3559 — A page box this reader could not read is written as the box it drew the page with

Session 2214. Status: **accepted and built**; extends ADR 3142 from a media box with a side of no
length to every box `pdf_model` logs it substituted. Context: ADR 0389 (`Page::DEFAULT_MEDIA_BOX`, a
choice), ADR 3541, ADR 3543's Consequences ("left unbuilt by name"). Clauses: ISO 19005-2 and ISO
19005-4 section 5.1; ISO 32000-2 §7.7.3.3 Table 31, §7.7.3.4, §14.11.2.1.

## Context

Eight documents failed their output's section 5.1 on `model/media-box-substituted`: a page whose
ancestry states no media box that is a rectangle with an area, missing, three numbers, or zero by
zero. Round 2210 left it unbuilt as "the reader's words in the producer's mouth".

## Decision

1. **Each such box is written on the page as the rectangle the page was drawn with**
   (`sites::boxes_as_drawn`, written by `write_boxes_as_drawn`): `/MediaBox` as `pdf_model::Page`'s
   media box — an ancestor's where usable, else the A4 of ADR 0389 — and a crop, bleed, trim or art
   box that is no rectangle with an area (stated, or for `/CropBox` inherited) as the box this
   reader computed for it, §14.11.2.1's default. Counted under `page-boundary-removed`, whose
   sentence names the case.
2. **No loss word.** ADR 3142 decided this for a box of no length: the file states no box, so
   nothing of the producer's is lost, and what the output states is what the window showed; the
   report's sentence says the box is this viewer's choice. The owner's stance is met by stating it,
   not by a refusal.
3. **Only on an object stating `/Type /Page`.** `issue15590`'s `/Pages` entry names a JavaScript
   action that this reader walks as a page; writing a box there would make an action a page, and
   veraPDF could not parse the result. It stays refused by the class's name.

## Evidence

Seven of the eight convert under both profiles at 2b and 4 and pass veraPDF 1.30.3; the test
`a_media_box_that_is_no_rectangle_is_written_as_the_box_this_reader_drew`.
