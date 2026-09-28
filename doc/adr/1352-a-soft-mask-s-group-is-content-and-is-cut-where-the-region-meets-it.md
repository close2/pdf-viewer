# 1352 — A soft mask's group is content, and is cut where the region meets it

Status: accepted and **built**.
Context: `crates/pdf-transform/src/redact.rs` (`Walk::enter_soft_mask`, `Walk::enter_frame`,
`Entered::MaskGroup`, `Walk::record_form_edit`), `crates/pdf-transform/tests/redact.rs`
(`text_inside_a_soft_mask_group_is_removed`).
Supersedes: ADR 1124's refusal of any page with a soft-mask group in force.

## The question

A `gs` whose `/SMask` names a soft-mask dictionary makes §11.6.5.1's transparency group the source
of the shape or opacity of everything painted after it. The group's own marks never reach the page
in colour. Two readings of §12.5.6.23 were available:

1. The mask is not content. Cut the masked marks, which is already built, and leave the group.
2. The mask's marks within the region are content. Cut them from the group's own stream too.

## The decision: the second

The clause's words are "remove all content identified by the redaction annotation" and "remove all
traces of the specified content", and it closes with "Such interactive PDF processors shall also be
diligent in their consideration of all content that can exist in a PDF document". A mask's group
marks the page. It does so as the opacity of every mark painted through it, not in colour, so what
the region showed is partly the group's work. A glyph drawn into a `/Luminosity` group can be read
as a hole in whatever the mask applies to. If only the masked marks were cut, the glyph's codes
would stay in the file. That is a trace by any reading of the word, and ADR 1277 already reached
the same answer for an image's `/SMask`, which it treats as image data.

So the walk enters the group at the `gs` that establishes it. The coordinate system is §11.6.5.1's:
"concatenating the transformation matrix specified by the Matrix entry in the transparency group's
form dictionary … with the current transformation matrix at the moment the soft mask is established
in the graphics state with the gs operator". The walk starts from the initial graphics and text
state, which is what the interpreter evaluates the group under. The `gs` is also where the
interpreter runs the group, so the group's codes fall exactly where the code count (trap 13) puts
them. Text, paths, images and nested forms in the group are removed or cut by the same rules as any
stream.

Cutting the mask changes its values only inside the region, and every mark painted through the mask
there has already been cut. So the page outside the region is unchanged, except that a glyph whose
box straddles the edge is removed whole, ADR 1124's choice.

## What comes with it

- The group is always written as a copy for the redacted page. It is reached through an
  `/ExtGState` entry, a graphics state dictionary and a soft-mask dictionary, and another page may
  share any of the three. A copy is correct in every case, because the original is carried into the
  output only where something else still reaches it (ADR 1196's rule).
- A form drawn more than once on a page is one stream, whether by two `Do`s or by a `Do` and a mask.
  Where the region meets its placements differently, no single edit removes only what the region
  covers, so the page is refused (`Walk::record_form_edit`). Placements that ask for the same edit
  are one edit.
- `/TR` and `/BC` are functions and colours of the whole mask, not of the region, and they are
  carried.
