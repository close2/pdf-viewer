# Annotations and events that depend on the view

Status: `NoZoom` and `NoRotate` done (ADR 0168); all ten of Table 197's events raised; `/FixedPrint` waits on printing.
Priority: 25
Corpus: 15 documents write an `/AA`; 124 annotations in 51 documents set `NoZoom`
Clauses: §12.6.3 Table 197, §12.5.3 Table 167
Code: `crates/pdf-model/src/annotation.rs`, `crates/viewer-core/src/interact.rs`

## Table 197's ten trigger events — **all ten are raised**

`/E`, `/X`, `/D` and `/U` are raised by the pointer since session 174; `/PO`, `/PC`, `/PV` and
`/PI` since the two-hundred-and-fourth, together with both of Table 198's (ADR 0164); and `/Fo`
and `/Bl` since the two-hundred-and-fifty-seventh.

**The last two were recorded here as wanting "keyboard focus, which `viewer-core` does not have —
there is no focus model in `Command` at all, and adding one is a vocabulary change rather than a
clause", and no message was needed.** Both entries are "(Optional; PDF 1.2; widget annotations
only)"; the clause says what happens when an annotation "receives the input focus" and nothing
about how it comes to. So a press inside a widget's active area gives it the focus and a press
anywhere else takes it away — a choice, and the one every pointing interface makes, recorded as
one. A page turned raises `/Bl` too, wherever the pointer is.

What a *keyboard* would add is Table 31's `/Tabs` order — which annotation comes next — and that
is a different clause (§12.5.1's row says so). **A blocker that names a vocabulary is the fourth
shape of stale reason this project has found**, after one that names a capability, one that names
an architecture, and one that names what a program would have to *have* rather than *say*.

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

**And neither flag reaches §12.5.6.10's four text-markup subtypes since the
two-hundred-and-thirty-sixth session** (ADR 0172), which is a choice under a conflict rather than
a derivation: §12.5.3's "shall always maintain the same fixed size on the screen" and
§12.5.6.10's "shall appear ... in the text of a document" cannot both hold at a magnification
other than 1, and the standard states no precedence. Counted first: 511 text markup annotations
across 34 documents, 211 of them carrying `NoZoom`, and all 211 are strike-outs in
`ISO_32000-2_sponsored_EC3.pdf` at one flag value.

## `/FixedPrint` — still owed, and it is a printing decision

Table 193's entry on a §12.5.6.22 watermark annotation states "graphics that are to be printed at
a fixed size relative to the target media, and fixed relative position on the target media", so it
waits on a printing path rather than on a display one. Table 167's Print flag is in the same
position and has been since the twenty-first session.
