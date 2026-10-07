# §12.7.4.3's remaining edges

Status: **done — nothing this file lists is owed.** The last document, `freetext_no_appearance.pdf`, draws since
ADRs 1413 and 1414. The file stays whole because comments under `crates/`
(`examples/variable_text_census.rs`, `view.rs`, `tests/saving.rs`, `tests/corpus.rs`) and
`doc/todo/65` cite it, and what it keeps is the reasoning behind the closed items, which is the
reason a later round will not reopen any of them. The row is `partial` on rich text's residue
alone: the formatting XFA 3.3 states is laid out run by run (ADRs 1634, 1635, 1649), and `doc/todo/65`
bucket 4 lists what the runs do not reach yet; a `/DA` text matrix with no inverse is
the clause carried out, and the report says why the field is blank (`Owed::SingularTextMatrix`).
Priority: 22
Corpus: 0 documents
Clauses: §12.7.4.3, §12.7.5.3, §12.7.5.4, §9.6.5.2, §9.7.6.2
Code: `crates/pdf-model/src/variable_text.rs`, `crates/pdf-model/src/view.rs`, `crates/pdf-font/src/shaping/`
Census: `crates/pdf-model/examples/variable_text_census.rs`

## ~~A `/DA` font `/DR` does not define~~ — **the last of seven drawn** (ADRs 1413, 1414)

A malformed file rather than a clause gap: §12.7.4.3 requires the name to match a `/DR` entry.
Since ADR 0112 the value is laid out in a stand-in **where the stand-in can draw all of it**, and
the missing font is named. The rule is asymmetric on purpose: a Latin stand-in drawing a paragraph
of Arabic's punctuation and nothing else is worse than a blank, and the first version of that ADR
drew six dots on an otherwise empty page.

**Five of the seven named one of §9.6.2.2's fourteen and nobody had noticed.** `/Helv`, `/HeBo`,
`/TiRo`, `/ZaDb` and their ten siblings are the four-letter abbreviations of the standard 14 and
there is no fifteenth, so the table is a *bijection* with the clause's own list rather than a habit
read off a corpus. A `/DA` naming one names a font this binary carries, and the value is drawn in
the face the name means: a documented choice about a malformed file, and one that buys ADR 0133's
own argument — those pages reproduce where no fonts are installed.

**`freetext_no_appearance.pdf` (`/Helv`, a paragraph of Arabic) needed three things together**, and
the order they depend in is the lesson: a glyph source first, because with no face the value
produced no codes and neither ordering nor shaping had anything to act on (ADR 1247 settled which
of the three declined), then joining, then order. ADR 0348 read and priced them; ADR 1414 took the
glyph source from the machine — a face covering every character the *shaped* value displays, asked
after both routes into the compiled-in fourteen, so what those draw is unchanged everywhere — and
the joining from the UCD's own tables as presentation-form code points; ADR 1413 took the order
from `unicode-bidi`, measured against every line of `BidiCharacterTest.txt`. **The `/Differences`
route stayed shut and it was right to**: 36 distinct missing characters against 31 free codes, and
no Adobe Glyph List name for any Arabic character. Where a machine offers no covering face the value
is still not drawn at all rather than in part, and a test that needs one skips saying so (ADR 1154).

- **A save writes the machine face subset and embedded** (ADR 1425): §9.9.2's tagged subset of the
  glyphs the value displays, no `cmap` under the `CIDFont` (§9.9.1), a `/CIDToGIDMap` stream where
  the glyphs were renumbered, every stream an object of its own (§7.3.8.1), in the incremental
  update and the archive's writer alike. A face whose `OS/2` `fsType` forbids embedding is not
  written: the widget goes out with Table 224's `/NeedAppearances` and `view::Written::unconstructed`
  names it (ADR 1159's route), and the archive's writer refuses it.

## ~~A `/DA` font name that is not text, and the escaping that goes with it~~ — **done in ADR 0453**

ADR 0453, and the reason it is worth keeping as a paragraph is the shape of the argument that
deferred it rather than the fix. ADR 0439's sweep found the read defect —
the `Tf` operand folded to a `String` before probing `/DR` — and declined to fix it, because this
module **writes** the same name into the appearance stream it constructs and wrote it with no `#xx`
escaping at all. Correcting the lookup alone would have found the document's font and then named a
different one, so read and write were one decision and the sweep costed it here instead of taking
half of it. **That was right**, and it is the standing example of a deferral that names its
condition: the note said what the round after it owed, and that round did exactly that list.

`pdf_syntax::Name::escaped` is §7.3.5's writing direction in one place, called by the object
serialiser and by this module; a `pdf_syntax::Name` runs from the `Tf` operand to the `/DR` probe to
the appearance's `/Resources` key, so the operand and the key cannot drift.

**The write half is the one with witnesses and this file predicted the opposite**, which is the
lesson worth keeping: it said "[n]o corpus or crawled document is known to reach it; the seven
`/DA` documents this file counts all name ASCII", and both halves of that were about the wrong
population. `examples/variable_text_census` counts the crawl and prints what it matched — five
objects over two documents, every one a font name with **spaces** in it, which is ASCII and is
exactly what a writer with no escaping breaks. Not one is outside UTF-8, so it is the *read* half
that has no witness anywhere. All five carry an `/AP`, so nothing on this disk draws differently
and what they reach is a save or an edit.

Closed and kept here for the reasoning:

- **`bug1865341.pdf`** (ADR 0184). Its value is *Załącznik*
  and its missing set was **one character**, `ą` — not a glyph any Helvetica lacks but a **code**
  neither §9.6.5.2 encoding has. A font this module invents may state its own encoding, so it does:
  `/Encoding << /Differences [1 /aogonek] >>`, with the name from the Adobe Glyph List that
  `read-fonts` already carries. Kept only when it reaches strictly more characters.
- `poppler-395-0-fuzzed.pdf` names `/Rufscript` and `issue19389.pdf` names `/F1`: neither denotes
  anything, so both stand in and both say so, which is the case the table is narrow enough to
  leave alone.

## ~~A composite `/DA` font~~ — **done in ADR 0337**

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

## ~~`/DS` and `/RV` are XFA, which `CLAUDE.md` excludes~~ — **read again in ADR 1197**

Two sentences of one line, both wrong, and the lesson is the one principle 5 states about a claim
that a clause hands something over: **the exclusion was never re-read against the clause that
names it.** `CLAUDE.md` excludes XFA on §K.1's permission, and that permission is about
schema-driven page generation; Table 228's entries are AcroForm entries whose *format* the
standard describes by reference, which is not the same act. And this tree already read the
identical construct — Table 172's and Table 177's `/RC` carry the same words and
`popup::rich_text` has taken their character data since ADR 0224 — so one construct named the same
way in three tables was being read in two and declared out of scope in the third.

What follows is in §12.7.4.3's row: a rich text value's **characters** are drawn, because
§12.7.5.3 makes "[t]he contents of this text string or stream" what the appearance is built from,
and its **formatting** is laid out in XFA 3.3's terms, the text being held (ADRs 1634, 1635) — from
Table 228's `/RV` and `/DS` both, 60 and 411 widgets of `examples/field_flag_census`'s count.

## ~~§12.7.5.4's list box~~ — **drawn since ADR 0407**

**ADR 0407 reversed this section's conclusion by reading one sentence further, and the argument
below is kept because the reversal is only legible beside it.** What ADR 0240 had
written here is next, unchanged.

> The clause states *which* items are selected and *in what order* they are shown — Table
> 234's `/Opt` "shall be presented to the user", Table 233's `Sort` row adds "PDF readers shall
> display the options in the order in which they occur in the Opt array" — and states nothing
> whatever about how a selected item differs from an unselected one. Drawing the options would draw
> all of a list and none of its selection, which is ADR 0112's asymmetry one clause over.

Two sentences of that are exactly right and the inference from them is not. The clause does state
what is shown — the same paragraph makes each option "a text string that shall be displayed on the
screen" — so refusing the list is refusing what the clause states in order to avoid what it does
not. ADR 0106's test is the one that decides it: a selection mark is drawn *over* an item that is
drawn either way, so it is additive, and a refusal of it may not take the item with it. ADR 0112's
asymmetry is about something else — a stand-in that draws *part* of a value, where the drawn part
is wrong rather than incomplete — and a list drawn whole with no highlight is not part of a list.

**What it cost the corpus was 0**, which `examples/variable_text_census` measured before any code:
**10 list-box widgets over 8 documents, every one with an `/AP` `/N` stream, none in a
`/NeedAppearances` document**. That measurement was right and it measured the wrong population:
the construction is only ever reached under a regeneration, so the count that mattered was of
documents whose value has *changed*, which no census of files can take. The
clause's `shall` about the items is discharged by `pdf_model::form::ChoiceControl`, which hands a
host the items in the array's order (ADR 0235).

**Revisit it with a document in front of you**, said this file, "not on principle: a list box whose
file states no appearance stream is the case the choice was made without, and there is none in 974
files." That instruction is the reason the item sat for a hundred and sixty-eight rounds, and it was
looking for the wrong document — the one that was needed is any list box at all, with a person's
choice applied to it.

**The *host* half was finished first** (ADR
0248), and that is what made the page half a debt rather than a curiosity: this program could
choose an item and could not show the result. A host can now say which
items a person selected — Table 233 bit 22 sets **4** of the corpus's widgets, over 4 documents —
because `Edit::SetField` carries a set of Table 234 `/Opt` indices instead of one string. §12.7.5.4's
two shapes of `/V` and Table 234's `/I` are both written. So the clause's `shall`s are discharged
everywhere except the one place it states nothing — and since ADR 0407 that place is the *mark*
alone: the options are drawn, from Table 234's `/TI`, and which of them the value selects is
reported rather than invented.

## ~~Table 231 bit 24's `DoNotScroll`~~ — **done in ADR 0197**

ADR 0197. `LaidOut::overflows` answers whether the value needs more room than the box gives, on the
axis the clause names — horizontally for a single line, vertically for several, and a count of
Table 232's cells for a comb — and `ViewState::set_field` takes the longest prefix that fits, over
every widget of the field, shortest wins. **260 corpus widgets over 8 documents set it**, which
`examples/field_flag_census` measured in that round and which makes it the most-stated of every
type-specific flag in Tables 229, 231 and 233; four of the twenty have no witness at all.

### ~~What it left owed: a host cannot read back what was accepted~~ — **closed, twice over**

This section read *"`Query::FieldAt` answers with §14.9.3's two names and **no value**… nothing is
wrong today, because `viewer-ui` sends no `Edit::SetField` at all"* and was stale in both halves:
`viewer-ui` has typed into fields since ADR 0201 and
`Answer::Field` has carried the value for as long. Found by `doc/todo/01`'s third sweep
(ADR 0247), which is exactly the shape that sweep hunts — a note whose stated reason
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

## ~~A field's baseline reads the same two entries with a weaker guard~~ — **done in ADR 0240**

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
