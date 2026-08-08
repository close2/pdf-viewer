# §12.7.4.3's remaining edges

Status: partly reported, partly unreached. **Table 231's `DoNotScroll` closed in the
three-hundred-and-thirty-eighth session** (ADR 0197) and **the baseline's guard and the list box's
cost both closed in the four-hundred-and-third** (ADR 0240); what those left is one query's shape
and one clause with no witness.
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

Still owed, and it is one file:

- `freetext_no_appearance.pdf` (`/Helv`, a paragraph of Arabic) declines, and the `/Differences`
  route cannot help it: the AGL names those characters `afii…` and no Helvetica has the glyphs.
  **That one is `doc/todo/21`'s per-character fallback**, where it is the only witness left.

Closed and kept here for the reasoning:

- **`bug1865341.pdf`** (two-hundred-and-eighty-fourth session, ADR 0184). Its value is *Załącznik*
  and its missing set was **one character**, `ą` — not a glyph any Helvetica lacks but a **code**
  neither §9.6.5.2 encoding has. A font this module invents may state its own encoding, so it does:
  `/Encoding << /Differences [1 /aogonek] >>`, with the name from the Adobe Glyph List that
  `read-fonts` already carries. Kept only when it reaches strictly more characters.
- `poppler-395-0-fuzzed.pdf` names `/Rufscript` and `issue19389.pdf` names `/F1`: neither denotes
  anything, so both stand in and both say so, which is the case the table is narrow enough to
  leave alone.

## A composite `/DA` font — 0 documents, and the last of the clause's edges

A **composite** `/DA` font needs §9.7.6.2's codespace ranges inverted, to turn a character back
into the code the font wants. `variable_text::set_in` refuses one by name today
(`Owed::FontUnusable`), so it is reported rather than silent, and **no corpus document states
one** — measured, not assumed. Trap 8's rule applies in the direction that keeps it open: a corpus
cannot rank a requirement no document exercises, so this waits for a spec-track round or a witness
rather than for the corpus to ask.

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

## ~~Table 231 bit 24's `DoNotScroll`~~ — **done in the three-hundred-and-thirty-eighth session**

ADR 0197. `LaidOut::overflows` answers whether the value needs more room than the box gives, on the
axis the clause names — horizontally for a single line, vertically for several, and a count of
Table 232's cells for a comb — and `ViewState::set_field` takes the longest prefix that fits, over
every widget of the field, shortest wins. **260 corpus widgets over 8 documents set it**, which
`examples/field_flag_census` measured in that round and which makes it the most-stated of every
type-specific flag in Tables 229, 231 and 233; four of the twenty have no witness at all.

### What it left owed: a host cannot read back what was accepted

`Query::FieldAt` answers with §14.9.3's two names and **no value**, so a host with a text box of its
own has no way to learn that a field took less than it sent. Nothing is wrong today — `viewer-ui`
sends no `Edit::SetField` at all — and it becomes wrong the moment a host types. The fix is the
familiar shape from ADRs 0166 and 0167: where a host needs a thing a variant does not carry, the
variant changes and every consumer fails to compile.

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
