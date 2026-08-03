# Annotations and events that depend on the view

Status: reported; the remaining trigger events want a focus model, and two flags may be unreachable by construction.
Priority: 25
Corpus: 15 documents write an `/AA`; 90 annotations set `NoZoom`
Clauses: §12.6.3 Table 197, §12.5.3 Table 167
Code: `crates/pdf-model/src/annotation.rs`, `crates/viewer-core/src/interact.rs`

## Two of Table 197's ten trigger events — and both want a focus model

`/E`, `/X`, `/D` and `/U` are raised by the pointer since session 174; `/PO`, `/PC`, `/PV` and
`/PI` since the two-hundred-and-fourth, together with both of Table 198's (ADR 0164). What is
left:

- `/Fo` and `/Bl` want **keyboard focus**, which `viewer-core` does not have — there is no focus
  model in `Command` at all, and adding one is a vocabulary change rather than a clause. It is
  the same shape as the panels: a clause blocked on an *interface* rather than on a reading.

**The four that left this list did so because the reason was stale, not because anything got
harder.** They were recorded as wanting "a page-visibility model, which a one-page-at-a-time
window does not have" — and a window that turns pages is one, with exactly one page in it.
§12.6.3 says the `/PV`–`/PO` distinction exists because "[a]t any one time, while more than one
page may be visible, depending on the page layout", so in this layout the two coincide by
derivation. Read a blocker that names what the *program* lacks with suspicion; that is
`doc/todo/01`'s third sweep and this is its second catch in three sessions.

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
