# §12.7.4.3's remaining edges

Status: **one document, and it is `doc/todo/21`'s.** Table 231's `DoNotScroll` closed in the
three-hundred-and-thirty-eighth session (ADR 0197), the baseline's guard and the list box's cost
both closed in the four-hundred-and-third (ADR 0240), and **the composite `/DA` font closed in the
five-hundred-and-second** (ADR 0337). What is left of this file is one refusal that belongs to
another item, and the reasoning behind three closed ones.
Priority: 22
Corpus: 1 document
Clauses: §12.7.4.3, §12.7.5.3, §12.7.5.4, §9.6.5.2, §9.7.6.2
Code: `crates/pdf-model/src/variable_text.rs`, `crates/pdf-model/src/view.rs`
Census: `crates/pdf-model/examples/variable_text_census.rs`

## A `/DA` font `/DR` does not define — **1 document, from 7**

A malformed file rather than a clause gap: §12.7.4.3 requires the name to match a `/DR` entry.
Since ADR 0112 the value is laid out in a stand-in **where the stand-in can draw all of it**, and
the missing font is named. The rule is asymmetric on purpose: a Latin stand-in drawing a paragraph
of Arabic's punctuation and nothing else is worse than a blank, and the first version of that ADR
drew six dots on an otherwise empty page.

**Five of the seven named one of §9.6.2.2's fourteen and nobody had noticed** — the
two-hundred-and-fifty-eighth session. `/Helv`, `/HeBo`, `/TiRo`, `/ZaDb` and their ten siblings
are the four-letter abbreviations of the standard 14 and there is no fifteenth, so the table is a
*bijection* with the clause's own list rather than a habit read off a corpus. A `/DA` naming one
therefore names a font this binary carries, and the value is drawn in the face the name means: a
documented choice about a malformed file, and one that buys ADR 0133's own argument — those pages
reproduce where no fonts are installed.

Still owed, and it is one file — **and it is the only thing this item still owes**:

- `freetext_no_appearance.pdf` (`/Helv`, a paragraph of Arabic) declines, and the refusal is now
  read, costed and pinned (ADR 0348, session 513). **The `/Differences` route is shut
  machine-independently** — measured, where this file used to guess at the mechanism: the value
  has 36 distinct missing characters against the invented array's 31 free codes, and the Adobe
  Glyph List `read-fonts` carries has *no name at all* for an Arabic character (zero `afii`
  entries), so `named_glyphs_reach_more` can reach no face on any machine. **And it is more than
  `doc/todo/21`'s per-character fallback**, which is the correction this entry needed: a chain of
  faces asked per character would draw isolated forms left-to-right even where it found every
  glyph — the wrong-but-plausible page, worse than the blank. Drawing this value takes an Arabic
  glyph source this binary does not have (no compiled-in face has one Arabic glyph — measured,
  against the standing assumption that Liberation Sans carries them), joining-form selection and
  right-to-left ordering, together or not at all; ADR 0348 has the cost of each and the order
  they depend in. `pdftoppm` draws this witness as its full stops scattered on an empty page,
  which is ADR 0112's rejected construction, looked at.
  `tests/variable_text.rs::the_arabic_free_text_declines_whole_and_names_both_halves` pins the
  blank and the report. Until a round takes ADR 0348's list whole, this file is kept for the
  closed arguments below, which are the reason a later round will not reopen any of them.

Closed and kept here for the reasoning:

- **`bug1865341.pdf`** (two-hundred-and-eighty-fourth session, ADR 0184). Its value is *Załącznik*
  and its missing set was **one character**, `ą` — not a glyph any Helvetica lacks but a **code**
  neither §9.6.5.2 encoding has. A font this module invents may state its own encoding, so it does:
  `/Encoding << /Differences [1 /aogonek] >>`, with the name from the Adobe Glyph List that
  `read-fonts` already carries. Kept only when it reaches strictly more characters.
- `poppler-395-0-fuzzed.pdf` names `/Rufscript` and `issue19389.pdf` names `/F1`: neither denotes
  anything, so both stand in and both say so, which is the case the table is narrow enough to
  leave alone.

## ~~A composite `/DA` font~~ — **done in the five-hundred-and-second session**

ADR 0337. The refusal's stated reason — a character cannot become a code without inverting
§9.7.6.2's codespace ranges — was true and was not a reason, which is the lesson worth keeping
from it: *a note that names the clause making something hard has not said why it is not done.* It
stood for four hundred sessions.

`CMap::each_addressable_code` inverts them, and the inversion is a **test** rather than a
construction: a code is offered only if the same `CMap` would extract exactly it from its own
bytes, so a file whose codespace is one byte wide and whose `cidrange` states two-byte codes
offers nothing rather than a code no string can contain. `variable_text::show` then writes a code
as the one to four bytes it occupies. Two refusals replace the one: a `CMap` stating more codes
than the bounded walk visits, and §9.7.5.1's writing mode 1 — the clause makes the mode decide
*which metrics* place a glyph, and this layout has one axis.

**Still 0 corpus objects**, which `examples/variable_text_census` now counts alongside the
baseline and the list box, so the fixtures are pairs differing in one entry and the corpus is
silent about all of it.

`/DS` and `/RV` are XFA, which `CLAUDE.md` excludes.

## ~~§12.7.5.4's list box~~ — **measured and left refused in the four-hundred-and-third session**

ADR 0240. The clause states *which* items are selected and *in what order* they are shown — Table
234's `/Opt` "shall be presented to the user", Table 233's `Sort` row adds "PDF readers shall
display the options in the order in which they occur in the Opt array" — and states nothing
whatever about how a selected item differs from an unselected one. Drawing the options would draw
all of a list and none of its selection, which is ADR 0112's asymmetry one clause over.

**What it costs the corpus is 0**, which `examples/variable_text_census` measured before any code:
**10 list-box widgets over 8 documents, every one with an `/AP` `/N` stream, none in a
`/NeedAppearances` document**. Where the refusal does fire under a regeneration,
`appearance::regenerate` leaves the file's own stream standing and names the shortfall. The
clause's `shall` about the items is discharged by `pdf_model::form::ChoiceControl`, which hands a
host the items in the array's order (ADR 0235).

**Revisit it with a document in front of you**, not on principle: a list box whose file states no
appearance stream is the case the choice was made without, and there is none in 974 files.

**The refusal stands and the *host* half is finished**, since the four-hundred-and-twelfth (ADR
0248). The page still draws nothing for a list box, for exactly the reason above; what changed is
the other direction, which this file never owed and `doc/todo/30` did. A host can now say which
items a person selected — Table 233 bit 22 sets **4** of the corpus's widgets, over 4 documents —
because `Edit::SetField` carries a set of Table 234 `/Opt` indices instead of one string. §12.7.5.4's
two shapes of `/V` and Table 234's `/I` are both written. So the clause's `shall`s are discharged
everywhere except the one place it states nothing, which is what a refusal with an argument behind
it should end up looking like.

## ~~Table 231 bit 24's `DoNotScroll`~~ — **done in the three-hundred-and-thirty-eighth session**

ADR 0197. `LaidOut::overflows` answers whether the value needs more room than the box gives, on the
axis the clause names — horizontally for a single line, vertically for several, and a count of
Table 232's cells for a comb — and `ViewState::set_field` takes the longest prefix that fits, over
every widget of the field, shortest wins. **260 corpus widgets over 8 documents set it**, which
`examples/field_flag_census` measured in that round and which makes it the most-stated of every
type-specific flag in Tables 229, 231 and 233; four of the twenty have no witness at all.

### ~~What it left owed: a host cannot read back what was accepted~~ — **closed, twice over**

This section read *"`Query::FieldAt` answers with §14.9.3's two names and **no value**… nothing is
wrong today, because `viewer-ui` sends no `Edit::SetField` at all"* and was stale in both halves:
`viewer-ui` has typed into fields since the three-hundred-and-forty-ninth (ADR 0201) and
`Answer::Field` has carried the value for as long. Found by `doc/todo/01`'s third sweep in the
four-hundred-and-eleventh, which is exactly the shape that sweep hunts — a note whose stated reason
is a capability the tree acquired sixty sessions ago.

**And the round that found it closed the second half too**, which is why this is worth keeping as a
paragraph rather than deleting. The value a host reads back is not always the field's characters:
Table 231 bit 14 answers with bullets, so a host obeying the read-back rule sent the bullets as the
next value — and `viewer-ui` did. `Answer::Field`'s value is `Option<pdf_model::view::ShownValue>`
now, the characters beside `obscured`, from the one reading that produces either; the fix followed
the familiar shape from ADRs 0166 and 0167, and every consumer failed to compile until it said what
it does. ADR 0247.

**And a caret is the other half**, which is `33`'s: a truncation is what this looks like to a host
that sends whole values, and *nothing happening as you type* is what the clause describes.

## ~~A field's baseline reads the same two entries with a weaker guard~~ — **done in the four-hundred-and-third session**

ADR 0240. `Metrics::read` calls `pdf_font::measured_extent`, so one band derived from Table 120's
own definitions decides what those two entries can say for both of the things in this tree that
read them; `DEFAULT_ASCENT` and `DEFAULT_DESCENT` stay as the fallback for a pair it refuses,
because they answer *where in its box does this text sit* rather than *how tall is this line*.

**The number this file carried was the wrong population's**, and that is the entry's lesson. It
said the change was owed to "the 42 font dictionaries that write a `/Descent` without its sign" —
which is `font_metric_census`'s count of the fonts a *page's* content streams draw with.
`Metrics::read` only ever sees a font `/DR` defines and a `/DA` names, and of the 695 objects
§12.7.4.3 lays text out for across the corpus — 622 widgets and 73 free text annotations — **254
state a pair and every one of them reads the same under both rules**. Sharing the band moved no
pixel, which the corpus, oracle, quorra and text gates then confirmed line for line.
