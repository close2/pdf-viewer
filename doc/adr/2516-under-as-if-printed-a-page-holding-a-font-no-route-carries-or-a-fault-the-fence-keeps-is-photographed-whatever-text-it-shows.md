# 2516 — Under `as-if-printed` a page holding a font no route carries, or a fault of its content stream the fence keeps, is photographed whatever text it shows

Status: implemented. Session 1841, the second archive slot of batch one hundred and twenty-five.
Builds item 1 of owner answer A694 ("q694 1) yes allowed"); the gate it widens is ADR 2394's,
the list it is built over is `doc/pdfa-remedies.md` (rows R08 and R03) and ADR 2490's ranking.
Context: `doc/questions/Q694`, ADR 0816 (the converter edits no producer's content stream).

## What the answer allowed

ADR 2394 photographs a page as the window draws it only where the page's readback names no
character, because turning text a reader can extract into pixels is a loss the owner priced
separately. A694 item 1 allows the widening under `as-if-printed`, at 2b and every part 4 flavour,
to a page holding an R08 font (a font no route of this tree carries) or an R03 fault (a defect in
the page's content stream that ADR 0816's fence refuses to edit), the loss recorded per page with
the page's text named. `as-shown`, 2u and 2a keep the refusal. A694 item 3 adds the reading that
governs the rest: where the input is bad the converter bakes into the file the state the viewer
shows.

## The decision

1. **A loss of its own, `page-as-image`.** `text-as-image` is granted by both shipped profiles at
   `fonts/font-programs-embedded`, so a wide gate keyed on it would have widened `as-shown`. The
   new word is a `Loss`, a field of `Authorisations`, and is among the conditional losses of
   the font site, which `as-if-printed`'s `discard` there grants and `as-shown`'s does not; the
   profile grammar is unchanged. A page photographed by the wide gate records both words, so the
   narrow loss still means what ADR 2394 meant. The class is information
   (`doc/pdfa-remedies.md`, the loss index): readable text becomes pixels.
2. **The gate is a value, `Gate { offered, anywhere, fenced }`.** `offered` is ADR 2394's target
   test; `anywhere` is the profile's grant and the target together; `fenced` is the set of pages
   and operators the R03 rows name. Nothing else about the narrow gate changed: a page it admits
   is photographed as before, with the sentence it had.
3. **Fenced rows.** `archive/fenced.rs` names the R03 rows whose subject is bytes inside a content
   stream: an operator no standard defines, an inline image's `/I`, a hex string with a byte that
   is no digit, a Type 3 glyph whose `d0`/`d1` width disagrees, an `ActualText` in the Private Use
   Area, and a name that is not UTF-8. A row is fenced for a page only when its places all name a
   page and none is whole-file; otherwise the refusal stays. Rows whose subject is an object, a
   box or the transfer function are not in the set (see Not decided).
4. **Fonts photographed around.** When a font's every route refuses and the wide gate is open,
   the font is held aside and the pass re-run with it deferred, up to 16 fonts; the 17th keeps
   its refusal. A font refused for a reason the photograph cannot answer (two widths for one
   glyph, a descriptor that is not an object) keeps it too.
5. **What the window shows is baked in where the document is at fault.** The wide gate drops the
   readable-text test; it keeps the test that the page is drawn whole, with one change. A report in
   `Unsupported` that states a fault of the page's own stream is baked in, as A694 item 3 reads:
   an operator short of its operands, a path with no current point, a matrix with no inverse, an
   undefined resource name, a stream damaged part-way, a soft mask dictionary the clause does not
   describe. The rest stay refused, because there the window's picture is missing or approximating
   something a well-formed document asked for, and the photograph would record this reader's gap as
   the document's loss: an image or shading this reader cannot build, a stream it cannot decode, an
   annotation, optional content, a page box or dictionary, a drawing stopped at a bound, and the
   constructions §9.3.8, §11.4, §11.6.2 and §11.7.4 define that this reader composites otherwise
   (`TextKnockout`, `TransparencyGroup`, `CompositedInParts`, `Overprint`). A694 item 3 licenses
   baking in bad input, not a renderer's shortfall on good input.
6. **The page's text is named.** The report, its JSON and the file's `xmpMM:History` give each
   photographed page's text, white space collapsed, cut at 1000 characters with the count left
   out. A page that was photographed because of a fence row also names the row.
7. **A tagged document, and the condition kept.** A document stating a structure tree (§14.7) is
   photographed where the profile also authorises `tagging-of-photographed-pages`, which
   `as-if-printed` does at the fonts row (A707): the tree's leaves that name a photographed page
   (an integer of an element whose `/Pg` is the page, a marked-content reference whose `/Pg` or
   whose element's is, and an object reference to a stream only the old content drew) are taken
   out, each element that lost one written under its own number, every element and the parent
   tree kept, and the page loses its `/StructParents`. A leaf of an annotation's appearance
   stays, since the annotation stays; an integer whose element names no page names none to
   find, and stays. Each page's count of leaves is in the report, the JSON and `xmpMM:History`
   (`crates/pdf-transform/src/archive/fonts/untagged.rs`). The parent tree's entry for the page
   is left, reached from nothing drawn, rather than rewritten: the number tree is the file's to
   shape, and an entry nothing looks up costs no reader anything. Level A is offered no
   photograph at all, so it keeps the refusal A707 asks it to keep. Optional content (§8.11)
   is still refused, as ADR 2394 has it; no document of the corpora asks, and A707 leaves it
   until one does.
8. **Bounds.** See ADR 2517.

## Why not the other way

Granting `text-as-image` alone and testing the profile by name would have put
a profile's name in the converter, which only reads the profiles in `doc/profiles/`. Dropping the readable-text test from the
narrow gate would have changed `as-shown`. Rewriting the content stream to remove the fault is
what ADR 0816 forbids and what CLAUDE.md's authoring line excludes.

## Not decided here

R03 rows whose subject is an object rather than a page's bytes (a Type 3 width, an `ActualText`
or a UTF-8-invalid name found in an object), the MediaBox row and the transfer function are not
photographed; their rows keep their refusals. R10 fonts are excluded. The rewrite label of a
photographed page is `substitute-font-program`, shared with the narrow gate; a label of its own
would split a column the profiles read.
