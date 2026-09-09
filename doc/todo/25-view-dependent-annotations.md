# Annotations and events that depend on the view

Status: **done.** `NoZoom` and `NoRotate` since the two-hundred-and-seventeenth session (ADR
0168), all ten of Table 197's events raised, and §12.5.6.22's `/FixedPrint` applied in the
nine-hundred-and-forty-second (ADR 0934). Every item this road was opened for is carried out;
what is left of the subject is RFC 0004's printing half, which is that RFC's.
Priority: 25
Corpus: 15 documents write an `/AA`; 124 annotations in 51 documents set `NoZoom`; **one** of the
4172 PDFs under `doc/` that open states a `/FixedPrint`
Clauses: §12.6.3 Table 197, §12.5.3 Table 167, §12.5.6.22 Tables 193 and 194
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

## `/FixedPrint` — **done in the nine-hundred-and-forty-second session** (ADR 0934)

**This section said the entry "waits on a printing path rather than on a display one" and that was
wrong, found in the nine-hundred-and-thirty-third session by reading §12.5.6.22 for a different
question** (ADR 0906). The clause introduces the entry's effect with a `shall` on *rendering*:

> When rendering a watermark annotation with a FixedPrint entry, the following behaviour shall
> occur

and its second bullet names the algorithm every annotation in this tree already goes through — the
transformed annotation rectangle "shall be used in place of the annotation rectangle referred to in
steps 2 and 3 of \"Algorithm: appearance streams\"", which is what `annotation::placement` carries
out against `/Rect`. The clause then forecloses the printing excuse twice over: "interactive PDF
processors shall use the dimensions of the media box" when one is displayed on-screen, and Table
194's own `/FixedPrint` row says that where the target media are unknown, drawing "shall be done
relative to the dimensions specified by the page's MediaBox entry". So the media dimensions a
screen needs are stated by the standard, not owed to a printer. Table 167's `Print` flag is a
separate question and stays where it was.

**The correction was the whole of the work.** `annotation::fixed_print` computes the transformed
annotation rectangle and `annotation::decided` hands it to `placement` in place of `/Rect`; nothing
else moved, because the substitution the clause states is an *argument* to §12.5.5's algorithm
rather than a second placement mechanism. Of the three things this file said a round would have to
derive, two were stated outright and the third came out smaller than its sentence:

1. `/Rect` translated to the origin and transformed by Table 194's `/Matrix`, then the smallest
   upright rectangle around the resulting quadrilateral. Stated outright — and *which* corner goes
   to the origin is stated too, one clause over: §12.5.5's step 2 is about "the lower-left corner
   (the corner with the smallest x and y coordinates)".
2. `/H` and `/V`, "as a percentage of the width of the target media (or if unknown, the width of
   the page's `MediaBox`)". Stated outright, with the on-screen media dimensions above. A
   percentage of a width is a *distance*, so the corner it is measured from is the media's own —
   which is where the third item's residue turns out to live.
3. **The one that needed a derivation.** "[G]iven a matrix B that maps a scaled and rotated page
   into the default user space, a new matrix shall be computed that cancels out B and translates
   the origin of the media (e.g., printed page) to the origin of the default user space." B is what
   places a page onto a *sheet*, which is what the paragraphs after the EXAMPLE are about: they
   open "[i]n situations other than the usual case where the PDF page size equals the media size"
   and go on to tiling and n-up. The on-screen sentence makes the page's media box **be** the
   media, so a screen is that usual case by construction and B's scale and rotation are the
   identity by the clause's own stipulation. What is left of the sentence is the translation
   between two origins, and §8.3.2.3's NOTE 1 says why that is not nothing: "the origin of default
   user space always corresponds to the lower-left corner of the output medium … it is not
   required". So the term is the media box's lower-left corner, and it is nought for every file
   whose media box starts there.

**One choice, recorded as one: §7.7.3.3's `/Rotate` is not cancelled.** §12.5.6.22 never names the
entry, it says the on-screen behaviour is "the same as for other annotations", and §12.5.3's
`NoRotate` is the flag that exists to depart from turning with the page — reading a second such
mechanism into the B sentence would make that flag redundant on this one subtype and would leave a
watermark unable to ask for the behaviour its own `/F` states. The same scoping settles the other
half of the interaction: the substitution names steps 2 and 3, so §12.5.3's fixed point stays the
rectangle the file states, and so does §12.7.4.3's substitute `/BBox`, which is step 1's operand.

**Counted before it was written, and the count is why the fixtures are hand-built.** RFC 0004's
survey had found `/FixedPrint` essentially absent from the corpora;
`crates/pdf-model/examples/fixed_print_census` put a number on it over every PDF in this tree — of
the 4172 that open, five state a watermark annotation and **exactly one states a `/FixedPrint`**:
`isartor-6-5-2-t01-fail-d.pdf`, of the Isartor PDF/A-1b suite. The 974-document gate corpus states
none, and neither do the four `doc/corpora` submodules. That one witness is a **check on the
arithmetic that cannot rank it**: its `/Rect [148.75 272.25 446.25 569.75]` on a 595 × 842 media
box, under `/Matrix [1 0 0 1 -148.75 -148.75]` with `/H 0.5` and `/V 0.5`, transforms to exactly
its own `/Rect` — so every one of the four terms has to be right for the mark not to move, its
raster is byte-identical before and after the change, and no picture of it could have told a
correct implementation from none at all. Trap 8's shape, and trap 1's inversion beside it.

RFC 0004 carries the *print* half of the same entry — the two bullets after the EXAMPLE, page
tiling and n-up, each conditioned on a selection a program with no print path never makes — and
after this the two halves are one substitution apart: the paper's dimensions in place of the media
box's.
