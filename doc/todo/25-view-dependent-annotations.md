# Annotations and events that depend on the view

Status: reported; two of them may be unreachable by construction.
Priority: 25
Corpus: 15 documents; 90 annotations measured
Clauses: §12.6.3 Table 197, §12.5.3 Table 167
Code: `crates/pdf-model/src/annotation.rs`, `crates/viewer-core/src/interact.rs`

## Six of Table 197's ten trigger events — 15 documents

`/E`, `/X`, `/D` and `/U` are raised by the pointer since session 174. What is left:

- `/Fo` and `/Bl` want **keyboard focus**, which `viewer-core` does not have — there is no focus
  model in `Command` at all, and adding one is a vocabulary change rather than a clause.
- `/PO`, `/PC`, `/PV` and `/PI` want a **page-visibility model**, which a one-page-at-a-time
  window does not have. A host that scrolls a continuous page tower would.

## `NoZoom`, `NoRotate`, `/FixedPrint`

Table 167 bits 4 and 5 make an appearance's size or orientation depend on the *view*, which a
resolution-independent display list cannot express: the whole point of the list is that a zoom
re-rasterises without re-interpreting, and these say the geometry changes with the zoom.

**Measured rather than assumed**: 90 corpus annotations set `NoZoom` — 78 of them popups this
tree draws nothing for, 11 `Text` and 1 `FileAttachment`. So the population that would actually
look different is twelve annotations.

The honest options are to re-interpret an annotation on zoom (which breaks the list's promise for
that one command) or to give the display list a "this command's transform is view-relative"
marker (which every backend then has to honour). Neither is obviously right, which is why it is
recorded rather than started.
