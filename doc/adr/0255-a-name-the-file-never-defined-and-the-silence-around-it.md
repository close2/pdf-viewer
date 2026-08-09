# ADR 0255 — A name the file never defined, and the silence around it

Status: accepted, 2026-08-09. Session 419. Left by session 418, which found its own defect had been
invisible for the whole of this project's life because of the silence this round removes.

## The clause first

ISO 32000-2 puts the requirement on the *file*, twice, and neither sentence is about a reader:

> A content stream's named resources shall be defined by a resource dictionary, which shall
> enumerate the named resources needed by the operators in the content stream and the names by
> which they can be referred to.

— §7.8.3, of every content stream there is; and of the operator itself, Table 86 in §8.8.1:

> Paint the specified XObject . The operand name shall appear as a key in the XObject
> subdictionary of the current resource dictionary (see 7.8.3, "Resource dictionaries"). The
> associated value shall be a stream whose Type entry, if present, is XObject .

§8.10.1's step b) says the same thing a third time — "[t]he name shall be defined in the XObject
subdictionary of the current resource dictionary". So a `Do` naming a resource nothing defines is a
**malformed file**, and the standard states nothing about what a reader does with one. That part is
this project's own rule and not a clause: draw what can be drawn, and say what could not.

**The clause was read for the case before the code was written, and it changed the answer once.**
§7.8.3's fourth bullet used to make the page's dictionary a fallback for a form that names something
its own does not define — "All resources that are referenced from those forms and fonts shall be
inherited from the resource dictionary of the page on which they are used" — and Errata Collection 3
strikes it (Issue #128), replacing it with an informative NOTE 3 that *reports* the rule of earlier
versions rather than stating it. Table 93's `/Resources` cell moves the same way (Issue #292):
"Sometimes required" rather than "Optional but strongly recommended", with independence from the
enclosing content stream "required" from PDF 2.0. The fallback that survives is on the entry's
**absence**, which is what this tree implements and what it will keep implementing for pre-2.0 files;
what does not survive is any warrant for reaching past a `/Resources` a stream did state.

## What was silent

`draw_xobject` returned on three conditions and said nothing on any of them: an operand that is not
a name, a name the `/XObject` subdictionary has no key for, and a key whose value is not a stream.
`apply_ext_gstate` and `pattern` did the same one category over. Against that, a missing **font** has
said "no /Font resource named /F1" since the interpreter had fonts, and `sh` has said "/Sh0 is not in
/Shading" since it had shadings. Three of Table 34's categories reported and three did not, and
nothing recorded the difference as a decision — trap 5's "[w]here a clause gives a parameter two
routes, implementing one of them is the failure mode that reports nothing", arriving one axis over.

The cost is on the record. Session 418's Type 3 defect — a glyph description's `/Resources` searched
in the wrong two places — drew nothing on any file that exercised it and was found through an
erratum rather than through a page, because the name that resolved to nothing resolved to nothing
*quietly*.

## What was made loud, and where the honesty is

`Unsupported::MissingResource { category, detail }`, reaching `Interpretation::unsupported` like
every other report, worded by `viewer_core::report::describe` as a statement about the document
rather than about this program: *the document names a /XObject resource it does not define*. It is a
new variant rather than a reuse of `Unsupported::Operator`, because "the operator Do is not
implemented" is a false sentence — `Do` is implemented and the file is wrong — and because a name
the file never defines has no subtype, no dictionary and no size, so the only true thing to say is
which of Table 34's subdictionaries was asked and what it answered.

**The boundary is `Interpreter::resource`, and that is why the round took three categories rather
than one.** Its three callers are `Do`, `gs` and `scn`; `Tf` and `sh` go through `resource_entry` and
already report; `cs`/`CS` reports through `ColourSpace::parse`. After this round every resource
lookup that can cost a mark reports its miss, which is a statement that can be checked, where "the
XObject one now reports" would have left the reader to find out about the other five.

`/Properties` is deliberately **not** in it, and the reason is the same one that put the others in.
A `BDC` whose property list is missing costs no mark: the section's operators still draw, the
optional-content test answers "not hidden", and what is lost is an `/ActualText` or an `/Alt`. A
report there would take a page off the oracle's judged set to describe a difference no raster holds.

**The condition is `is_hidden()`, and it was derived from §8.11.3.1 rather than from convenience.**
Content in a group the configuration turns off "shall be skipped, as if there were no `Do` operator
to invoke it", so a `Do` there was never going to mark the page and cannot have lost one.
`paint_shading` had exactly this rule for `sh` already — "a hidden layer skips it whole, including
the report a shading we cannot build would otherwise make". Two neighbouring cases the risk analysis
named turn out to reach nothing at all: an `XObject` a resource dictionary *defines* and no content
stream draws never reaches `Do`, and a name a form uses that only the page defines is the
whole-dictionary fallback above rather than a miss.

## What the corpus said, document by document

**68 → 70 incomplete of 974, and both are new reports** (trap 5). Nothing else on the list moved:
the other 68 lines are identical.

- **`issue6541.pdf` — `/R41 is not in /XObject`.** Four levels of nesting: the page draws `/R57`,
  which draws `/R45`, which draws `/R44`, which fills a rectangle with tiling pattern `/R42`, whose
  cell is `q /R41 Do Q`. The **pattern** states its own `/Resources` — a `/Font` and a `/ProcSet`,
  no `/XObject` — so `/R41` is looked up there and is not there. The page defines it. **The file is
  wrong by §7.8.3's third bullet**, which requires a pattern's resource dictionary to contain "all
  the resources used by that content stream"; the fallback that would have found it is the one EC3
  struck. What was lost: **nothing at all.** `/R41` is a form XObject whose stream is zero bytes
  long, so the whole chain paints an empty pattern and every renderer draws a blank page. Ours and
  `poppler`'s differ by 0.002 of 255.
- **`issue8702.pdf` — `/Meta6 is not a stream`.** Object 10 lives in an object stream, and §7.5.7
  forbids one to hold a stream, so the producer's `/Subtype /Form` arrives as a bare dictionary with
  a `/BBox` and a `/Matrix` and no content anywhere in the file. The `Do` sits over a filled
  rectangle on a PowerPoint slide. **Nothing can draw it**, and nothing does: `poppler` prints
  *Syntax Error (1722): XObject 'Meta6' is wrong type* and its page is ours to 0.04 of 255. Both
  pages were looked at side by side.

Neither report costs a mark, and that is worth stating rather than hiding. **A report is not a claim
that ink is missing; it is a claim that the file asked for something this reader could not resolve.**
For `issue6541.pdf` the reader genuinely cannot know — the name resolves to nothing, so nothing says
the object is empty. For `issue8702.pdf` it can see that the dictionary carries no data, and that
*is* the loss: the producer stated a form XObject and the file does not contain it.

**Extending to `gs` and `scn` cost zero further documents**, measured before it was committed. The
only corpus witness is `operator_list_cycle.pdf`'s `/a0 gs`, in a form whose `/Resources` states a
`/Pattern` and nothing else, and that document is already on the list for the form-depth cycle it is
named after. `scn` has no witness in 974 files, which is said out loud.

## What the oracle paid

Exactly what ADR 0152 predicts and no more: **two pages leave the complete set and no verdict
moves.**

| | before | after |
|---|---|---|
| pages / complete / incomplete | 1794 / 1690 / 104 | 1794 / **1688** / **106** |
| agrees, of them complete | 905 / 863 | 905 / **861** |
| contradicted, of them complete | 68 / 66 | 68 / 66 |
| ambiguous, of them complete | 786 / 750 | 786 / 750 |
| our geometry, reference geometry, not comparable, no render | 1/0, 2/2, 14/9, 18/0 | unmoved |

Both documents were *agreeing* pages, so the two that left came out of `agrees`' complete column and
nowhere else. The undiagnosed-ambiguous list printed empty, the contradicted ranking's head is
`bitmap-symbol-context-reuse.pdf` at 28.91 as before, and the reference cache hit rate was 99.7%.

The text gate's denominator moved by the same two documents in both directions — 24243 → 24187 words
and 24043 → 23987 — so 99.2% is unchanged, which is what that denominator is for.

**`doc/todo/00`'s step 7 is not owed, and it is provable rather than believed**: the whole diff under
`crates/*/src` is `self.note(…)` inserted in front of a `return` that already existed, plus comments.
No display list can differ, and the oracle's 1794 verdict lines and quorra's `912 / 36 / 9 / 17` say
so.

## The by-product, which is a defect in this tree

Making `gs` loud failed a test in `pdf-model/tests/soft_masks.rs`. Its fixture named `/GA` and `/GB`
in a mask group whose `/Resources` defined neither, so both fills were painted opaque, both halves of
the mask came out at 1.0, and the test asserted the same number twice about a difference the file did
not contain — under a doc comment saying "two rectangles of the *same* black at two different
constant alphas" and a comment *inside* it admitting the opposite. §11.5.2's test was not testing
§11.5.2. The fixture states `/ca 0.25` and `/ca 0.75` now and asserts 191 and 64, both derived from
the clause.

That is the argument for this change compressed into one file: a report is worth what it finds, and
the first thing this one found was in here.

## The sweep, and two more populations of quotation

The round's spec-track item was §8.10's ledger rows against the code and their table citations.
**Every table those rows cite is the right table** — 34, 56, 74, 75, 86, 93, 94, 95 — which is a
negative result and the first the ninth sweep has produced. §7.8.3's row was wrong about something
else: it said "every resource lookup falls back to the enclosing dictionary rather than failing",
which the code has never done and which this round would have contradicted anyway.

Reading §7.8.3 for the clause found the stale quotation in `content.rs` described above, and *that*
found two holes in `tools/spec-errata`:

- **A quotation in an ordinary `//` comment.** `prose_quotations` read `///` and `//!` only, on a
  stated reason — "a `\"` in a `//` comment is not making `CLAUDE.md`'s claim" — which is a claim
  about `CLAUDE.md` that `CLAUDE.md` contradicts. Thirteen landings, two of them findings (§7.8.3
  and §8.9.7).
- **A quotation with an ellipsis in it.** `overlaps` compared a quotation whole, so a quotation of
  *parts* of a sentence matched only where the struck passage was shorter than it — blind to exactly
  the shape `CLAUDE.md`'s own convention produces. Eight more landings, four of them findings, two
  of which are the two files session 418 recorded itself as having missed.

Six stale quotations corrected in six files. `doc/errata-read.md` carries a verdict apiece.

**One false positive set the rule.** Asking whether *any* elided segment matched reported
`image.rs`'s §11.6.5.2 comment against a sentence about `/BaseFont`, on the four words "the same as
the". `overlaps` now asks for one segment quoting the passage whole **or** every segment inside it.

## What is not done

- **A fallback to the page's dictionary for a name a stated `/Resources` omits.** It would draw more
  on `issue6541.pdf`-shaped files and the standard defines nothing either way, so it is a choice
  rather than a reading. It is not taken, for the reason session 127 had to undo the same shape for
  fonts: a page's `/Fm0` and a form's `/Fm0` are two objects as often as they are one, and reaching
  past the dictionary that names them is how a reader draws the wrong one and says nothing. The
  report is what makes the question decidable next time — it names the documents.
- **`Font` and `Shading` keep their own wordings.** Three variants now say "a name the resource
  dictionary does not define" in three sentences, which is one sentence too many; unifying them is a
  refactor and this round is not it.
- **Quotations in Markdown under `doc/`.** The fourth and fifth populations are read; a sixth —
  every quotation of the standard in this file, in `doc/HANDOVER.md`, in `doc/todo/` and in the other
  254 ADRs — is checked by nothing at all.
