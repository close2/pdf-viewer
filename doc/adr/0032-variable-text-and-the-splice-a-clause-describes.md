# ADR 0032 — Variable text, and the splice §12.7.4.3 actually describes

Status: accepted, 2026-07-30.

## Context

§12.7.4.3's variable text was the largest item left on the demand list: **13 corpus
documents**, and the one job that closes four separate refusals at once — a text field's `/V`,
a check box's tick, a `FreeText`'s `/Contents` and a push-button's caption. The twenty-first
session's ADR 0030 built every annotation appearance whose subtype clause states a *shape* and
stopped exactly where a *layout* began, because this crate had never laid text out: everything
it draws is a content stream the document wrote.

The spec item is the same family, which is the fifth time that has been available: §12.7.4 and
§12.7.5 are 13 subclauses and every one of them was `unreviewed`, as were §12.7, §12.7.1,
§12.7.2 and §12.7.3 above them.

A survey of the corpus first, before any code, because a gap sized by a corpus is a hypothesis
about a clause. Of the widget annotations on the corpus's first pages that would need a
constructed appearance: **147 are empty text fields** waiting for a person, 36 are text fields
holding a value, 19 are comb fields, 9 are push-buttons with a caption and no `/AP`, 14 are
check boxes, 4 are combo boxes, and **not one is a list box**. So the shape of the work was a
single line of text in a box, with wrapping and combs behind it.

## Decision

### A text string is a clause, and it is not Latin 1

A field's `/V` and a free text annotation's `/Contents` are §7.9.2.2 *text strings*, and
nothing in this tree decoded one. `crates/pdf-syntax/src/text_string.rs` is that clause: the
UTF-16BE and UTF-8 byte order markers the clause states, surrogate pairs paired as its own
sentence requires, §7.9.2.2.2's language escape sequences removed, and Annex D's **Table D.3
compiled in** for everything else.

The table is worth its own paragraph because the tempting shortcut is wrong in a way nothing
reports. `PDFDocEncoding` is not ISO Latin 1: 0x18 to 0x1F are accents where Latin 1 has
control codes, 0x80 to 0x9E are punctuation, and **0xA0 is EURO SIGN** where Latin 1 has
NO-BREAK SPACE. A Latin 1 decode draws a space where a document states a currency symbol.

It lives in `pdf-syntax` rather than `pdf-font` because a text string is a *string object
type* — §7.9.2, inside clause 7 — and Table D.3 is a code-to-Unicode table needing no glyph
names, which is exactly what separates it from Annex D.2's font encodings.

### A font is addressed by character, by running the mapping backwards

Everything else in this tree starts from a code the document wrote. Writing a content stream
needs the inverse, and `LoadedFont::code_for` is it — built by asking every code the font
defines what it means and what glyph it reaches, and answering a character with the first code
that both means it and has a glyph.

That construction is the point. The forward and backward directions traverse the same tables,
so a code this returns is a code that *draws* the character asked for; a separate reverse table
could disagree with the drawing path and nothing would notice. It is built lazily, because a
document may load hundreds of fonts and contain no form field.

For a composite font it returns nothing. A `CMap`'s codespace ranges decide how many bytes a
code occupies (§9.7.6.2) and inverting that is a different question from inverting a 256-entry
table; the refusal is reported by name rather than guessed at, and `addresses_characters` makes
"this font lacks that character" and "this font cannot be addressed by character" two different
statements.

### What the clause states, and what it hands back

`crates/pdf-model/src/variable_text.rs` implements §12.7.4.3, and the honest way to describe it
is in two lists.

Stated, and implemented: the `/DA` string parsed for its `Tf`, its colour and its text state;
the font resolved in Table 224's `/DR`; `/Q`'s three quaddings; a size of 0 auto-sized; at most
one `Tm`, with its translation replaced and the rest carried through; Table 231's Multiline,
Comb and Password; Table 232's `/MaxLen` dividing a comb field's box; and the marks inside the
`/Tx BMC` … `EMC` pair the clause's own EXAMPLE shows.

Handed back, in the clause's own words — "positioning values it determines to be appropriate …
and any layout rules it employs", and for auto-sizing "an implementation dependent function":

- **Where a baseline sits.** Nothing in ISO 32000-2 says where in a field's box its text sits
  vertically. A font descriptor stating Table 122's `/Ascent` and `/Descent` is the document
  answering, and it outranks anything chosen here; a standard-14 font has no descriptor, and
  the fallback splits the em three to one. A *constant* rather than the substitute font
  program's own metrics, deliberately: substitution is the only machine-dependent code in the
  tree, and layout should not join it.
- **How far apart two lines are.** 13/12 of the size — which is not invented. §12.7.5.3's own
  EXAMPLE lays out two lines of a multiline text field at `/Ti 12 Tf` with `0 -13 Td` between
  them, so the standard's only worked example of this operation spaces lines at that ratio. A
  `TL` in the `/DA` outranks it, because that is the document stating the same thing.
- **What auto-sizing picks.** The largest size at which the value fits the box on both axes,
  found by twenty halvings rather than solved, because line breaking is not a continuous
  function of the size.

### The one finding the oracle produced, and it is the whole of `/NeedAppearances`

Table 224's `/NeedAppearances` is "a flag specifying whether to construct appearance streams
and appearance dictionaries for all widget annotations in the document", and the obvious
reading is: when it is true, rebuild the appearance from `/MK` and the value. That reading
shipped for about an hour and the oracle rejected it.

`text_field_own_canvas_calc.pdf` page 3 carries a text field with **no value**, no `/MK`, and a
stored appearance stream whose entire content is a light grey bar inside a `/Tx BMC` … `EMC`
pair. Rebuilding from `/MK` draws nothing at all. `poppler` and `mupdf` draw nothing either —
but for a reason the rebuild does not have, and reading the clause's closing paragraph gives it:

> The interactive PDF processor shall then replace the existing contents of the appearance
> stream from … BMC to the matching EMC with the corresponding new contents

**Regeneration is a splice, not a rebuild.** Everything outside the marked-content pair is the
file's own artwork and survives; only the `/Tx` region is rewritten, and a field with no value
rewrites it to nothing. The clause even states the case where there is no such pair — "the new
contents shall be appended to the end of the original stream" — which is the opposite answer
for a stream whose marks sit outside it. Two fixtures differing in nothing but where the marks
sit get opposite answers, and `crates/pdf-model/tests/variable_text.rs` holds both.

The resources follow the same paragraph: `/DR`'s entries are copied into the stream's own
`/Resources`, and "the one already in the Resources dictionary shall be left intact" — which is
a *per-resource* rule rather than a per-category one, so the merge is two levels deep.

### The flag does not reach three of the four field types, and the field types say why

§12.7.4.3's subject is a field that "may contain text whose value is not known until viewing
time". Three of the four types §12.7.5.1 lists have no such text, and each says so in its own
subclause:

- a push-button "retains no permanent value … it shall not use the V and DV entries"
  (§12.7.5.2.2);
- a check box's and a radio button's states each "shall be defined by an appearance stream in
  the appearance dictionary of the field's widget annotation" (§12.7.5.2.3, §12.7.5.2.4), and
  the value *selects* among them rather than describing them;
- a signature field's value is a signature dictionary, and signing "entails updating at least
  the V entry and usually also the AP entry" (§12.7.5.5).

So only a text field and a choice field have their stored appearance spliced. That is a
derivation from the field types rather than a caution about the flag, and it is what keeps a
push-button's icon on the page.

### Where it refuses, and why each refusal is a different kind

- **A `/DA` naming a font `/DR` does not define.** The clause makes the match a requirement on
  the writer — "The specified font value shall match a resource name in the Font entry of the
  default resource dictionary" — and states no recovery. Reported by name, which is exactly
  what a content stream naming an absent font already gets. Inventing Helvetica from the
  resource name `/Helv` would need a name-to-typeface table no clause states. **5 corpus
  documents**, four of them free text annotations in files with no interactive form dictionary
  at all.
- **A list box.** §12.7.5.4 says which items are selected and states nothing about how a
  selection *looks* — no highlight colour, no rule. Refused on the same grounds as §12.5.6.10's
  text markup, and no corpus document reaches it.
- **A check box the file calls on with neither an `/AP` nor a caption.** The document states
  that the box is ticked and states no mark for the tick. Reported, because a box drawn empty
  is not a near miss — it is the opposite answer. 1 corpus document.
- **A composite `/DA` font**, as above. No corpus document.

### A password field does not echo its value

Table 231 bit 14: characters "shall instead be echoed in some unreadable form, such as
asterisks or bullet characters". A value stored in the file at all already breaks the same
row's NOTE; echoing it as it stands would publish it. Bullets. No corpus document has one,
which is why there is a test.

## Consequences

**Eight corpus documents left the incomplete list, 137 to 129**, and the arithmetic is worth
more than the total: 9 stopped reporting `/NeedAppearances`, 3 stopped reporting a widget's
value and 4 free text annotations stopped reporting their text — while 5 began reporting a
`/DA` font `/DR` does not define and 1 began reporting a check box it draws empty. **Nothing on
that row is a `/NeedAppearances` any longer.**

**The gated set grew by 9 and the contradicted count did not move.** 1611 pages to 1620, 747
agreeing to 754, 102 contradicted to 102 — the second consecutive session in which a feature
added pages to the comparison without adding a disagreement.

**Interpretation costs 0.31%** — 1.9340 G instructions to 1.9400 G by callgrind on
`examples/callgrind_interpret`, baseline measured on this machine at the previous commit. The
first draft cost 0.83%, all of it reaching the catalog for `/AcroForm` once per *constructed
annotation* rather than once per `/DA`; a specification page full of link borders pays for
every one of them and none names a resource.

**25 ledger rows left `unreviewed`**: the whole of §12.7.4 and §12.7.5, §12.7 through §12.7.3
above them, §7.9.2's five subclauses, §7.3.4.2, and §14.6.1 — whose "properly (separately)
nested" rule turned out to be the algorithm the splice needs, read as code rather than as a
constraint.

**A sixth fuzz target.** `fuzz/fuzz_targets/variable_text.rs` puts the fuzzer's bytes into a
widget's `/DA` and `/V` at once, because the two interact: the `/DA`'s size decides whether the
value is auto-sized and the value's length decides how many lines the size has to fit.

**What is left of the demand list is not a clause.** §12.5.6.10's text markup (8 documents) is a
decision about what a highlight looks like, §9.7.5.2's predefined `CMap`s (12) a licensing
decision, and a password prompt (8) is `viewer-ui` work. The largest item that is code is now
a `/DA` font resolved against nothing, and the answer to it is a report.

## What it taught

**A clause's last paragraph can invert its first.** §12.7.4.3 opens by describing a processor
constructing an appearance stream and closes by describing it *splicing* one, and only the
second sentence tells you what happens to a stored stream. The rebuild reading is defensible
from the opening, produces the same answer on every document with a value, and differs on
exactly the document that has none.

**Two references agreeing can be evidence — when you can say what they are agreeing about.**
Trap 9's shapes are all about agreement that means nothing. This is the converse and it is worth
recording: `poppler` and `mupdf` blanked a page we drew, the clause said why, and the reading
that explained their output was also the one the clause states. Agreement is evidence *after*
the clause is read, never instead.

**A rule the corpus cannot exercise is defended by one test, and that keeps being true.** Comb
fields exist in the corpus 19 times and not one holds a value; no corpus field is
right-quadded; no corpus field is a password field or a list box. Fourth consecutive session to
find load-bearing rules no real file reaches.

**An eager lookup on a cold path is a hot-path cost when the path is per-annotation.** Reading
`/AcroForm` to build every constructed appearance's `/Resources` is obviously cheap and was
2.7× the whole feature's cost. Measure the thing that runs per object, not the thing that looks
expensive.
