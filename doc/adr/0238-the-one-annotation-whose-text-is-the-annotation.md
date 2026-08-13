# ADR 0238 — The one annotation whose text is the annotation

Status: accepted, 2026-08-08 (session 401). **Amended by ADR 0304 (session 469)**, which takes the
one decision this ADR deferred: `set_free_text` and `free_text_at` now reach the free text
annotation the *file* states, so the two sentences below saying they refuse it, and the sentence
saying Table 167's `Locked` and `LockedContents` are therefore unreachable, describe this round
rather than the tree.

## Context

ADR 0196 gave a person §12.5.6.10's four text markups and named what it left owed in its last line:
free text, and a host that sends the command from a drag. `doc/todo/33` carried three reasons; the
three-hundred-and-eighty-eighth session re-read them and found two expired — the text has been laid
out by §12.7.4.3 since the twenty-third session, and a host has had somewhere to type since the
three-hundred-and-seventy-first. What was left was real, and this round is it.

§12.5.6.6 is the one markup subtype whose text *is* the annotation:

> A free text annotation ( PDF 1.3 ) displays text directly on the page. Unlike an ordinary text
> annotation (see 12.5.6.4, "Text annotations"), a free text annotation has no open or closed
> state; instead of being displayed in a popup window, the text shall be always visible.

That single sentence decides the whole shape of what follows, and it is worth saying why. A
highlight has *no geometry of its own* — §12.5.6.10's four "appear as highlights, underlines,
strikeouts … in the text of a document", so a selection is the whole of what they need and
`Edit::Markup` carries no coordinates at all. A free text annotation has nothing on the page to be
over. Its geometry has to come from somewhere else, and the only somewhere else a pointing
interface has is a **drag**.

## Decision

### Two verbs, because creating a box and saying what is in it are two things

```
Edit::FreeText    { from, to, colour }   → Done::FreeText    { page, rect, colour }
Edit::SetFreeText { annotation, text }   → Done::SetFreeText { annotation, text }
Query::FreeTextAt { at }                 → Answer::FreeText  { annotation, text }
```

`Edit::Markup` takes its whole meaning from the moment it arrives — mark up *what is selected* —
and so it needs one verb. This subtype needs two, and the second is what a keystroke sends sixty
times in a minute. Folding them into one verb carrying a rectangle *and* a string was the
alternative, and it costs the log its meaning: a replay would add one annotation per keystroke,
because there would be nothing in the log to say that the second entry was about the first one's
object.

**The two corners are in device pixels of the viewport**, which is what every other point in this
vocabulary is (`Command::Pointer`, `Query::Offset`, `Query::FieldAt`). The map out of them is
`Viewer`'s rather than `Open`'s, because it needs the viewport's size and the display's scale —
properties of the window and not of an open document — so `Open::resolve` takes the mapped pair as
an argument. What reaches the log is the rectangle in **default user space**, which is ADR 0196's
rule applied one subtype over: the log records what was *done*, and a replay after the zoom moved
must reproduce the same rectangle.

**The annotation is named by its object, and a host learns it by asking.** `Query::FreeTextAt` at a
point inside the rectangle answers with the annotation and with Table 166's `/Contents` as the log
now has it. No event carries the identity back, because a host can ask — which is the rule this
vocabulary has grown by since it was written, and the reason it has nine questions and no new
events.

**The object is stable across a replay**, which is what makes naming one in the log sound at all:
`ViewState::add_free_text` allocates from the document's own highest object number plus however
many annotations have been added, so replaying the log adds the same annotations in the same order
under the same numbers. Undo takes the text out; a second undo takes the box away; redo puts both
back and the second entry still names the first one's object.

### Only an annotation this session added takes text, and that is a boundary rather than a gap

`ViewState::set_free_text` refuses anything that is not in its own `added` list, and
`ViewState::free_text_at` answers `None` for every free text annotation the **file** states. That
is deliberate, and the reason is `CLAUDE.md`'s: an edit is a log beside an immutable document, and
appending an annotation is a different act from replacing an object the producer wrote. Aiming a
keyboard at something nothing can change would be an interface that pretends to work.

It also settles a `shall` without having to argue about it. Table 167 bit 10 is `LockedContents` —
"do not allow the contents of the annotation to be modified by the user" — and bit 8 is `Locked`.
No annotation this program creates sets either, and no annotation the file states can be reached,
so there is no path from a keystroke to an object carrying one. The §12.5.3 ledger row said those
flags were unreachable "until `doc/todo/33` lands"; that sentence expired this round and now says
what makes them unreachable instead of predicting it.

### The caret needed no new arithmetic, because §12.5.6.6 hands this subtype to §12.7.4.3

> Subclause 12.7.4.3, "Variable text", describes the process of using these entries to generate the
> appearance of the text in these annotations.

`appearance::caret`, `appearance::offset_at` and `appearance::selection` all began by reading a
`Field` and refusing anything that was not a text or combo-box field. They are unchanged. What
changed is one level below them: `frame` and `laid_out_in` dispatch on `/Subtype`, so a free text
annotation gets Table 177's `/RD` inner rectangle where a widget gets its border inset under Table
192's `/R`, and `free_text_layout` where a widget gets `field_text`. Everything above that — the
caret's two points, the point-to-offset inverse, the shapes over a range — is one implementation
for both.

That is the shape `doc/todo/33` predicted in one line and it is worth keeping the line: *the layout
underneath is shared and the way in is not*. `ViewState::typeable_at` is the way in, and it asks
about an added annotation **before** a widget, because the interpreter draws what a person added
after the page's own `/Annots` and the last thing drawn is the thing on top.

### What is written, and the three choices in it

Table 177's Required `/Subtype` and `/DA`; Table 166's `/Rect`, `/Contents` and `/F 4`; a `/BS`.
The standard describes *reading* a free text annotation and states nothing about creating one, so
three of those are choices and are recorded as choices:

- **The `/DA`'s size is 12 points.** §12.7.4.3 makes zero available — "[a] zero value for size
  means that the font shall be auto-sized" — and auto-sizing is not what a person drawing a text
  box means: it grows one character until it fills whatever rectangle was dragged.
- **The colour goes into the `/DA` and not into Table 166's `/C`.** That entry is an icon's
  background, a popup window's title bar and a link's border, and this subtype has none of the
  three; the colour of the *text* is §12.7.4.3's, which is the `/DA`.
- **The `/BS` states Table 168's `/W` 0**, and this one was found by running the program rather
  than by reading. Table 166's `/Border` states "Default value: [0 0 1]", so an annotation saying
  *nothing* about its border has one a point wide — and no clause anywhere says what colour to draw
  it, which is why `appearance` refuses a free text border and reports it. The first window run of
  this round therefore printed a report about the annotation the program had just created. A file
  this program writes may not leave that question open, so the annotation says it has no border.
  The alternative was inventing a colour.

The same run corrected a sentence that had been false for fourteen sessions: `free_text`'s comment
said the border was "reported only where a width is stated for it", and the condition fires on
Table 166's default too. The condition is right — the default is the table's — and the sentence was
not.

### §12.7.4.3's `/DR` is a `shall` about the file this program writes, and it is now kept

> The specified font value shall match a resource name in the Font entry of the default resource
> dictionary (referenced from the DR entry of the interactive form dictionary; see "Table 224
> -Entries in the interactive form dictionary").

This tree *reads* six corpus documents that break that sentence and recovers from all six by name
(ADR 0112, and the fourteen abbreviations of ADR 0133's argument). Writing a seventh would be a
different thing entirely. So a save that adds a free text annotation states `/Helv` in Table 224's
`/DR`, creating the interactive form dictionary — with the Required `/Fields` as the empty array,
which is what a document with no fields has — where the document has none.

**Where the file keeps each level is where each is rewritten**: the innermost indirect object on the
path from the catalog through `/AcroForm`, `/DR` and `/Font` is the one replaced, and the levels
above it are folded into it only where the file wrote them inline. That is `/Annots`' rule from ADR
0196, applied to a four-level path instead of a two-level one. **The document's own definition
always wins**: nothing is written where `/Font` already has the key, because the clause's sentence
is satisfied the moment `/DR` states the name and what it states is then the document's opinion
about its own resource — which is the same rule `variable_text`'s `Resolution::Named` follows when
drawing.

`/Helv` rather than an invented name, because `variable_text`'s fourteen abbreviations are a
bijection with §9.6.2.2's fourteen standard fonts: a reader that has never heard of this program
knows what the name denotes, and this program draws it from its own binary rather than from
whatever face the machine offers.

### Table 177's `/CL` and `/LE` are reported by name

The clause states the geometry — four or six numbers, "the starting, knee point, and ending
coordinates of the line in default user space" — and states no colour to draw it in, for the same
reason `/BS`'s border has none. So the callout is refused and named, which is trap 5's rule: an
annotation drawn without what it asked for is a silently wrong page.

**That is also the whole of `/IT`.** Its three values are `FreeText` ("a plain free-text
annotation"), `FreeTextCallout` ("intended to function as a callout … through the callout line
specified in CL") and `FreeTextTypeWriter` ("a click-to-type or typewriter object and no callout
line is drawn"), with `FreeText` the default. Two of the three ask for nothing this construction
does not already do, and the third asks for the line `/CL` carries — so reporting the entry covers
the intent, and the row says so rather than listing `/IT` as unread. **The markdown conversion
drops two of those three values and the default line**, which `pdftotext -layout` over `doc/`'s PDF
shows and which is `doc/HANDOVER.md`'s standing caveat about tables earning its keep again.

The corpus cannot exercise the report: the incomplete count is unmoved at 65, so no first page in
the 974 states a `/CL`.

### A host needs a mode, and that is the host's

`viewer-ui` binds `f`: it arms the next drag, the press records a corner, the release sends
`Edit::FreeText` and then asks `Query::FreeTextAt` at the rectangle's middle to learn what it made.
Nothing about that is in the standard, and it is the fourth key this window answers itself rather
than by sending a command — whether a mode is armed is chrome, and rule 5 keeps chrome out of
`viewer-core`. The colour is a dark red, on the same footing the highlight's yellow stands on.

`Typing` gained one field, `Target`, and everything else about typing is unchanged: the same
caret, the same selection, the same arrow keys, the same clamp after every keystroke. Only two
things branch — the question that reads the text back, and the edit that puts it there.

## Consequences

- **A person can draw a text box, type in it, see it, save it and re-open it.** In the window under
  `Xvfb`, on `PDF20_AN001-BPC.pdf`: `f`, then a drag from (380, 840) to (760, 900). The drag alone
  changes **22 pixels**, in a box 2 wide and 11 tall at the rectangle's top-left corner — that is
  the caret and nothing else, because §12.5.6.6's text is the annotation and there was none yet.
  Typing `Reviewed` changes **271 pixels**, x 380–425 by y 840–850, and nothing outside the
  rectangle moves.
- **A second reader shows the same words in the same place.** Saved from the window: 174 090 bytes
  against the original's 173 159, and `cmp -n 173159` says the producer's bytes are there byte for
  byte underneath. Re-opened in a fresh viewer, **246 pixels** differ from the original page, x
  380–423 by y 840–847 — the same words without the caret, drawn from the `/AP` in the file.
  `pdftoppm` differs from the unedited page by **314 pixels** in x 96–147 by y 826–835 at 72 dpi,
  and `mutool draw` by **312** in *the same box*; `pdftotext` extracts the word `Reviewed`.
- **All three consumers carry it.** `viewer-core/tests/headless.rs` states the whole sitting from a
  drag to a re-opened file; `pdf-model`'s `saving.rs` states which objects the update contains and
  that `/DR` names the font; `viewer-confined`'s `confined.rs` draws, types and saves behind the
  seccomp filter, with the red counted on a raster the confined process produced. A vocabulary that
  reached two of the three would be a confined host with less (ADR 0178).
- **Every gate is unmoved but the test count**: 1404 from 1398, which is this round's six. The
  corpus's 65 incomplete, the oracle's 904/69/786, quorra's 912/36/9/17, text at 99.2%, dates,
  XMP and JPEG 2000 all reproduce line for line, and the ledger is 875 rows with the same six
  counts. Nothing here changes a page any corpus document draws.
- **What is still owed** is editing a free text annotation the *file* states, and Table 177's `/CL`
  drawn rather than named. `doc/todo/33` carries both, and neither is a blocker wearing a
  capability's coat: the first is a decision about replacing a producer's object, and the second
  waits on a colour no clause states.
