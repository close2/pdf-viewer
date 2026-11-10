# 2780 — An inline image's data hides no name from the exemption

Session 1984, a single archive round.
Status: accepted and built. In `pdf-archive`: `reach.rs` (`read_names` steps over an inline image's
data at the end `pdf_model::inline_image::scan` finds).
Test: `crates/pdf-archive/tests/reach.rs`'s `a_name_after_an_inline_images_data_is_still_read`.
Context: `Exempt` (ISO 19005 section 6.2.2's last sentence, `doc/todo/62` section 3), ADR 0957
and `Rewrite::RenderingIntent` (an unknown intent restated as `RelativeColorimetric`).
Clauses: ISO 19005-2 sections 6.2.2 and 6.2.6; ISO 32000-2 §8.9.7 (inline images), §8.6.5.8.

## Context

veraPDF failed the `as-if-printed` output of `AIAA-2002-4016-606` on ISO 19005-2 section 6.2.6:
32 image `XObject`s state `/Intent /RelativeColormetric`, a name not among §8.6.5.8's four. The
checker named 114 of the source's 146 such images, and the conversion restated exactly those.
The other 32 were held exempt as reached only through resource names no content stream invokes —
and every one of them is drawn by a `Do` on its page. The exemption's reader lexed each content
stream as tokens from start to end, inline images included: a sample byte `(` in the data of
`BI … ID … EI` opened a string that swallowed every operator up to the next `)`, the `Do`s among
them. `Exempt`'s documentation states that its approximations only under-exempt; this one
over-exempted, and withdrew failures from every requirement the exemption narrows.

## Decision

1. **An inline image's data is stepped over, never lexed.** §8.9.7 makes the bytes between `ID` and
   `EI` image data. On `ID`, the reader asks `pdf_model::inline_image::scan` — the interpreter's
   own reading of the image whose `BI` preceded it, which checks the end it derives from `/L`, the
   sample arithmetic or the filter against an `EI` — where interpretation resumes, and seeks there.
   The image dictionary's own names, read on the way to `ID`, still count: `/CS` can name a colour
   space resource.
2. **The conversion is unchanged.** The images are now named by section 6.2.6's row, and
   `Rewrite::RenderingIntent` restates them as it restated the 114: §8.6.5.8 says a processor that
   does not recognise the name uses `RelativeColorimetric`, so the entry is restated to the name
   every conforming reader already used (ADR 0957's stated class). No pixel moves.

## Consequences

`AIAA-2002-4016-606`'s outputs state no unknown intent, and veraPDF's 6.2.6 failure goes. Any other
file whose inline image data holds a `(` before a used name no longer exempts the resources behind
that name, so a census can show a document newly refused or newly converted on a requirement the
exemption narrowed; each is a failure the checker should always have reported.
