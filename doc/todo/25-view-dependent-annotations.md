# Annotations and events that depend on the view

Status: `NoZoom` and `NoRotate` done (ADR 0168); `/Fo` and `/Bl` want a focus model; `/FixedPrint` waits on printing.
Priority: 25
Corpus: 15 documents write an `/AA`; 124 annotations in 51 documents set `NoZoom`
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

## `NoZoom` and `NoRotate` — **done in the two-hundred-and-seventeenth session** (ADR 0168)

Both are applied. What this file said — that they "make an appearance's size or orientation
depend on the *view*, which a resolution-independent display list cannot express" — was a reason
about this project's architecture rather than about the standard, and splitting it in two
dissolved most of it:

- **`NoRotate` depends on §7.7.3.3's `/Rotate`, which is in the file.** No vocabulary, no
  re-interpretation, nothing from a host.
- **`NoZoom` depends on the magnification**, which arrives through `ViewState` — where rule 1
  says a statement about the view belongs — and `Interpretation::view_dependent` says whether a
  page has an annotation that would notice, so 923 of the 974 documents never re-interpret on a
  zoom.

Measured: 124 annotations in 51 documents set `NoZoom` and 127 in 51 set `NoRotate`, 82 of each
being popups this tree draws nothing for. **No corpus document has a `NoRotate` annotation this
tree draws on a page with a non-zero `/Rotate`**, so that half is checked by a hand-built fixture
whose numbers are one composition of two matrices.

## `/FixedPrint` — still owed, and it is a printing decision

Table 193's entry on a §12.5.6.22 watermark annotation states "graphics that are to be printed at
a fixed size relative to the target media, and fixed relative position on the target media", so it
waits on a printing path rather than on a display one. Table 167's Print flag is in the same
position and has been since the twenty-first session.
