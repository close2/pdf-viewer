# 3509 — A tagged document's preserved annotation marks stand outside its structure tree, as an artifact

Session 2202. Status: **accepted**; built. Context: `doc/questions/A1011` (the owner's answer to
Q1011), A707 and ADR 2516 (the same answer for a photographed page), ADRs 1014, 1123, 1163, 2051,
2536, 3463. Clauses: ISO 32000-2 §14.8.2.2.1, §14.8.2.2.2 (its EXAMPLE 1 and its last sentence),
Table 355; ISO 19005-2 sections 6.3.1, 6.7.1, 6.8; ISO 19005-4 sections 6.3.1, 6.9 (paraphrased).

## What is built

1. **Where `tagged::MARKS_IN_A_TAGGED_DOCUMENT` refused, the marks now stand outside the tree.** An
   annotation the target forbids is removed and its normal appearance kept on an appended page
   (ADRs 1014, 1123). In a document with a structure tree, §14.8.2.2.2's last sentence makes
   content no element reaches an artifact, so the page needs no structure type to be described:
   `preserve::marks_stream` encloses its `q cm Do Q` in `/Artifact BMC … EMC`, the form EXAMPLE 1
   requires "to identify a generic artifact", and nothing is added to the producer's tree — no
   element, no `/StructParents`, no parent-tree entry.
2. **The loss `tagging-of-preserved-marks`** (`Loss::TaggingOfPreservedMarks`), because what goes is
   the marks' place in the logical structure, not the marks. It is a profile key,
   `[profile] tagging-of-preserved-marks = "discard"`, and no site's: the condition is the
   document's tree, met alike by every row whose `preserve` appends marks (`recovered-structure`'s
   shape, ADR 2502). `--authorise tagging-of-preserved-marks` takes it on the command line.
3. **Only at PDF/A-2b and the three PDF/A-4 flavours** (`tagged::marks_may_stand_outside`). Level A
   keeps the refusal whatever was authorised, as A1011 says: its subject is the logical structure.
   **Level U keeps it too**: A1011 names 2b and part 4, and Level U requires no structure, so
   whether it follows is asked as `doc/questions/Q1023` rather than assumed.
4. **On an appended page only, never on the producer's.** The annotation rows first try to put the
   marks back onto the page they came off (ADR 1123). In a tagged document every mark now takes an
   appended page instead, declining with `TAGGED_PRODUCER_PAGE`: A1011 is an answer for an appended
   page, and unbracketed marks among the producer's tagged content would change what the
   producer's own page describes.
5. **Named per page.** `report::Preserved::outside_the_tree` marks the row; the report a person reads
   adds a line under it, the JSON row a `losses` array, and the row's `xmpMM:History` clause ends
   "standing outside the structure tree as an artifact … losing tagging-of-preserved-marks".
6. **The profiles.** `as-shown` takes it: the marks are what the window showed. `state-every-loss`
   takes it as `discard` (its statement page is not built for this loss): it converts whatever the
   window can show. `as-if-printed` states nothing, because it discards a forbidden annotation and
   so never appends marks.
7. **What A1011 does not reach keeps its refusal**: an attachment drawn as appended pages
   (`remedies.rs`) is another document's content, not an annotation's appearance, and now refuses
   with a sentence of its own (`tagged::DRAWN_FILE_IN_A_TAGGED_DOCUMENT`), answered by the row's
   `on-failure`; a derived file's pages keep `DERIVED_PAGES_IN_A_TAGGED_DOCUMENT`.

## What is left

A PDF attachment drawn as pages is converted by an inner run under the outer plan's
authorisations (`derived::convert`, ADR 2536), so an inner tagged document's marks now stand
outside its tree too (`digitally_signed_3D_Portfolio` under `as-shown`); the outer report names the
inner run's pages and not the losses it took, for this loss as for every other. Naming them is a
build of its own.

## Proof

`crates/pdf-transform/tests/archive/tagging_of_preserved_marks.rs`: at 2b, 4 and 4e the page reads
`/Artifact BMC` … `EMC`, the producer's tree keeps its one element, the appended page states no
`/StructParents`, and the report and the packet name the loss; at 2a and 2u the refusal stands with
the loss authorised; without it the sentence names the loss; the three profiles' grants as above.
