# ADR 0304 — The annotation the producer wrote, and the flag that says it is not locked

Status: accepted, 2026-08-13. Session 469. Amends the §12.5.6.6, §12.5.3 and §7.5.6 ledger rows and
closes `doc/todo/33`'s first open item. Extends ADR 0212's restriction shape with a fifth clause.
Changes nothing any earlier ADR decided.

## The question

This program could put a free text annotation on a page and type into it (ADR 0238) and could not
touch one the **file** states. `doc/todo/33` carried the reason: "appending an object is the writing
`CLAUDE.md` permits, and replacing one the producer wrote is a decision nobody has made". Sixty-eight
sessions later, that is the decision.

It is worth taking rather than the other two capability items the round was offered, and the reason
is a count. `crates/pdf-model/examples/free_text_census` walks every page of the 971 corpus documents
that open:

```text
971 document(s) opened, 27 stating a free text annotation
  73 free text annotation(s)
  54 state /Contents with something in it
  67 carry an /AP /N stream, 11 of which hold a BMC marked-content region
  Table 167: 1 ReadOnly (bit 7), 3 Locked (bit 8), 1 LockedContents (bit 10)
  0 state Table 177's /CL callout line
```

Twenty-seven documents ask. Table 177's `/CL`, the other half of `doc/todo/33`, is stated by **none**
of them and would need a colour no clause states; `RadiosInUnison` (`doc/todo/30`) has a corpus
population of zero by ADR 0197's own table; and §12.4.4.2's presentation states nothing any corpus
document exercises. This is the one of the four where the standard states what to do *and* a real
document asks for it.

## What the standard states

### That replacing the producer's object is the writing already permitted

§7.5.6's own list of what an incremental update carries is three things, and this tree had
implemented one and a half of them:

> The contents of a PDF file can be updated incrementally without rewriting the entire file. When
> updating a PDF file incrementally, changes shall be appended to the end of the file, leaving its
> original contents intact.

The clause names "objects that have been changed, replaced, or deleted". A field's `/V` and a page's
`/Annots` were already replaced this way; the producer's bytes stay in the file byte for byte under
the update either way. So the thing that had been refused was never the *writing* — `write.rs` has
taken a map of replacements since the hundred-and-thirty-sixth session — but what `pdf_model::view`
was willing to say. The refusal named an architecture that did not exist.

### Where the appearance comes from once the text is the reader's

Table 177's `/DA` row states a precedence and it is the sentence that looks like it forbids this:

> The annotation dictionary's AP entry, if present, shall take precedence over the DA entry

Read as written, that is a precedence between **two things the file says about one text**: the
producer's stream and the producer's default appearance string describe the same `/Contents`, and the
stream wins because it is the more specific statement. It says nothing about a text the file does not
state. Once a person has retyped the note, the stored stream describes an annotation that no longer
exists, and §12.5.6.6 says where the appearance comes from instead:

> Subclause 12.7.4.3, "Variable text", describes the process of using these entries to generate the
> appearance of the text in these annotations.

So the construction this program already had — `appearance::free_text`, which is what draws an
annotation with no `/AP` and what `ViewState::add_free_text`'s own annotation gets — is the answer,
and the stored stream is set aside for exactly the annotations whose text a reader replaced.

### Replaced rather than spliced, and the census is why

The obvious reading was the other one. §12.7.4.3's closing paragraph updates an existing stream in
place:

> The interactive PDF processor shall then replace the existing contents of the appearance stream
> from /Tx BMC to the matching EMC with the corresponding new contents … (If the existing appearance
> stream contains no marked-content with tag Tx, the new contents shall be appended to the end of the
> original stream.)

`appearance::regenerate` already implements that for a widget, and §12.5.6.6 sends this subtype to
this subclause, so reusing it looked like the derived choice. **The census refutes it.** Of the 67
free text annotations carrying a stream, **11 hold a marked-content region and 56 do not** — so for
five annotations in six the clause's parenthesis applies and the new text would be *appended to* a
stream that already draws the old one. Two notes on top of each other is not what any sentence here
asks for.

The parenthesis is right for a widget and wrong here because the two subtypes' streams are different
kinds of thing. A widget's stream is background, border and artwork **with** the variable text in a
marked region, which is why everything outside the region must survive; a free text annotation's
whole appearance *is* its text — "a free text annotation … displays text directly on the page" — so
there is nothing outside the region to keep. The subtype whose appearance is entirely variable is the
subtype whose appearance is entirely regenerated.

The stream dictionary is likewise built afresh rather than kept, which follows from where the marks
are: this construction writes in the page's own default user space with `/BBox` the annotation's
`/Rect`, so §12.5.5's map is the identity (ADR 0196's argument, one subtype over), and a `/Matrix` the
producer wrote for their own marks would move them.

### Table 167's two lock flags, and the one that says it does not apply

§12.5.3's Table 167 has two bits that look like this prohibition and only one is:

> Locked … If set, do not allow the annotation to be deleted or its properties (including position
> and size) to be modified by the user.

whose row then ends — in prose here, because `doc/md/`'s conversion splits *changes* into "chang es"
and the sentence cannot be quoted against it; `pdftotext -layout` over `doc/`'s PDF has the word
whole — that the flag does **not** restrict changes to the annotation's contents, naming a form
field's value as its example. Bit 10 is the one that does:

> LockedContents … If set, do not allow the contents of the annotation to be modified by the user.

So the flag named `Locked` explicitly excludes this operation and the flag named `LockedContents`
states it. A reader that consulted the first would refuse an edit the table permits.

## The decision

**A free text annotation the file states can be retyped, and the retyping is a log entry beside an
immutable document like every other edit.** `ViewState::retyped` maps the annotation's object to what
it now says; `ViewState::free_text_at` answers for the page's own `/Annots` as well as for what a
person added; `AnnotationView` gains a fifth statement, `contents`, which `annotation::decide` uses to
set the stored stream aside and `appearance::free_text_layout` uses in place of `/Contents`; and
`ViewState::save` writes the annotation with its new text and a replaced `/AP`.

**Table 167 bit 10 is read in `restriction::asserted` and not at the point of the edit.** `CLAUDE.md`
is explicit that a document's restrictions are the reader's to set and that "the *policy* is asked,
once, in a place a host can supply — not hard-coded as a refusal at the point of the operation". That
module already answers for §12.8.2.2's `/DocMDP`, §7.6.4.2's Table 22 and §12.7.5.5's field lock; this
is its fifth clause and the second addressed to a **named object** rather than to the document, so
`asserted` takes an annotation beside the field it already took. `viewer_core` decides, `--ignore-
restrictions` turns it off, and the refusal can become ADR 0212's *ask* on the day a host can answer
one. Bit 8 is deliberately not consulted, on the table's own sentence.

**A retyped annotation this program cannot lay out is written with its text and no appearance at
all**, and `Written::unappeared` names it — the same shape as `Written::withheld`, and for the same
reason. Table 177 makes `/DA` Required, so the next reader has everything §12.5.6.6 needs to generate
what this one could not; an `/AP` left standing would be the file's own instruction to draw text
nothing in it claims any more. A note whose text a person *removed* is not this case: it draws
nothing because there is nothing to draw, and nothing is owed.

**Table 177's `/RC` is not a fallback for an emptied note.** `free_text_layout` falls back from
`/Contents` to `/RC` because §12.5.6.2 NOTE 1 makes the two textually equivalent — a fallback between
two things the *file* states. A person who took the text out has not asked for the producer's words to
appear from underneath, so `AnnotationView::contents` being `Some("")` is decisive, exactly as
§12.7.6.3's removed `/V` is for a field.

## What it cost the boundary: nothing

**Not one message was added, and no host changed a line of code.** `Query::FreeTextAt` already
answered "which free text annotation is at this point, and what does it say"; it now answers for the
producer's annotations too. `Edit::SetFreeText` already named its target by object, and an object the
file states needs no argument about stability across a replay — its number *is* its identity, where an
added annotation's had to be argued for. `doc/ui-boundary.md` asks that a message be added only for a
question a host cannot answer for itself, and the six consumers on that boundary gained this
capability by being recompiled.

`pdf_model`'s own API did change in three places, and each failed every consumer's build until it said
what it does: `set_free_text` takes the document (it must check that the object is a free text
annotation of *this* file), `restriction::asserted` takes an annotation, and `Written` has a third
field.

## What it is worth, and what it is not

**It reaches the program**, which is the test `doc/habits.md` names as this project's recurring
refusal shape. Under `Xvfb`, `target/pdf-viewer doc/pdf.js/test/pdfs/freetexts.pdf` was clicked at the
"Hello World From Firefox" annotation Firefox wrote — `note: typing into the free text annotation 32
0` — typed into, and saved with `s`. The written file is 45 447 bytes: `cmp -n 44273` shows the
producer's file byte-identical underneath, object 32 is the annotation with its new `/Contents`, and
object 33 — Firefox's own appearance stream — is replaced under its own number with the regenerated
marks. `pdftotext` and `mutool` both read the new text back, which is evidence about our reading and
not the definition of it.

**What is still owed on this subtype** is Table 177's `/CL` callout line, which no corpus document
states and which needs a colour no clause gives, and Table 167 bit 8 itself, which needs a reader that
*moves* or *deletes* an annotation. This program does neither.
