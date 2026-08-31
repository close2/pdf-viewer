# ADR 0764 — Two silences under one number, and the field that separated them

Status: accepted, 2026-09-01. Session 837. Cites ISO 32000-2 §9.7.5.1 and its NOTE, §9.7.4.2,
§9.10.2 and §9.5's NOTE 5. Amends `doc/todo/21` §7's first priced remainder and leaves ADR 0152's
trade, ADR 0270's split and ADR 0763's two halves exactly where they are. It is ADR 0422's
mechanism used a second time: a count that crosses the crate boundary instead of a report.

## The question, and it was left in the shape a round can lose

ADR 0763 declined to report a substituted face with no vertical form, in one paragraph:

> **A report.** A substituted face with no vertical form draws the producer's character in the
> producer's place in a shape the substitute had, which is the same shortfall as a face with no
> glyph for a character at all — and ADR 0152 priced that: a report costs the oracle a judged
> page, and this is a statement about a face rather than about a file. It stays counted-by-nothing
> and described here, exactly as ADR 0270 left its neighbours.

Two things in that are true and one is not. The refusal to report is right, and the analogy to a
face with no glyph is right. **"Exactly as ADR 0270 left its neighbours" is not**: ADR 0270 left
its neighbours *counted* — `Interpretation::codes_without_a_glyph` and
`codes_reaching_a_blank_glyph` are the whole of that decision — and this one was left counted by
nothing at all. So the two shortfalls the paragraph calls "the same" were one silence with one
number under it, and the number was about the other one.

That is trap 5's subject with the sign it is easiest to miss: not an input refused without a word,
but a *reading of a clause* that goes unhonoured with every instrument in the tree reporting zero.
`Interpretation::is_complete()` is true, `unsupported` is empty, the readback is perfect, the text
gate is unmoved, and the picture is wrong. `VerticalText.pdf` was in exactly that position for its
whole life before ADR 0763 and no gate in this tree could say so.

## The decision: the third option, again

ADR 0422 asked this question in three terms rather than two and the same three apply:

- **A report is wrong**, and for a sharper reason than the cost. §9.5's NOTE 5 puts the choice of
  a substitute outside the standard, so a face with no `vert` feature is not this program failing
  to do something the standard requires — it is this machine's font catalogue. An `Unsupported`
  entry is this program stating what it could not draw, and the page *was* drawn. The cost is real
  too and is ADR 0152's arithmetic: a page that reports leaves the oracle's judged set.
- **Silence is wrong**, because it is the state the tree was in and nothing could see it.
- **The count crosses.** `Shortfall` grows a fourth field,
  `Shortfall::without_a_vertical_form`, and every consumer ADR 0422 built for it carries the
  fourth number without any of them being asked to decide anything new.

## The condition, derived from the clause and printed before it was believed

Trap 11 is what decides whether a count is worth anything, and the condition here is §9.7.5.1's
NOTE read as a conjunction of four facts, all of which must hold:

1. the font is a **substituted** composite one, so §9.7.4.2's "CIDs shall not participate in glyph
   selection" applies and the face is reached by character;
2. its `CMap` is in **writing mode 1**, so the NOTE is about it at all;
3. the character collection **calls this CID that character's vertical form**, which is
   `predefined::is_vertical_form` and is the collection's own statement out of Table 116's pair —
   not "this CID is somebody's vertical form" and not "this page is vertical";
4. the face has a **glyph for the character** and states **no `vert` or `vrt2` form** of it.

Fact 4's first half is what makes the two silences disjoint rather than merely distinct: a
character the face cannot draw at all never reaches this question, because there is no glyph to
ask about — it is `uncovered_character`'s, which is ADR 0152's population. So the two counts
partition rather than overlap, and `pdf-model/tests/vertical_forms.rs` asserts exactly that as a
trichotomy: for each of the witness's vertical CIDs, the face cannot draw the character, or draws
the form, or draws it upright and is counted for it — **exactly one**, on any machine.

Fact 3's second half is what keeps the count from meaning "a vertical page". A CID the collection
gives one form under both writing modes has no form to lose, however poor the face is, and the
same test asserts that of the three unrotated CIDs on the witness's first column.

## Calibrated against the defect, both ways (trap 13)

A count that reads zero on a tree with the defect in it is worth nothing, and on this machine the
honest run *is* zero — the face this catalogue offers for Adobe-Japan1 states the forms. So the
defect was planted and the instrument watched:

| the tree | the trichotomy test | `vertical_form_census` on the curated corpora |
|---|---|---|
| as committed | passes, on the *form supplied* arm | 0 codes over 0 documents |
| `VerticalForms::read` made to return an empty map — a face with no `vert`, which is every Latin face | passes, on the *counted* arm | **15 codes over 1 document**, `VerticalText.pdf` |
| that, **and** `unsupplied_vertical_form` made to answer `None` — the instrument broken | **fails** on CID 7911, the first pair | — |

The third row is the one that matters, and it is the row a sweep that was never calibrated does
not have: with the defect present and the instrument broken, something has to go red. The middle
row is the count firing on the only document either corpus has.

## What is *not* changed, and each is a decision

- **`Shortfall::is_whole()` stays what it was.** A code drawn upright was named and was drawn;
  what differs is the shape of the mark. Folding it in would make that method's own sentence false
  and would give `viewer-accessibility`'s status group a sentence to speak — and a reader who
  cannot see the page loses nothing at all to a bracket drawn upright. The count crosses for the
  consumers that show a picture and is deliberately not spoken. The same reasoning ADR 0422 used
  to word a naming gap apart from a refusal, one step further out.
- **No pixel moves.** `Downward::read` no longer refuses a face with no vertical form, so the
  route is now *kept* for such a font — and `Form::Unsupplied` draws the glyph the caller already
  had, which is what the previous `None` did. The difference is that the question can now be
  asked.
- **No new data and no new reading.** Both halves of ADR 0763's rule are untouched.
- **Nothing on any horizontal page.** The condition's first two facts are resolved when the font
  is loaded, so a document that opens no substituted vertical composite font reaches none of this.

## The census, and why it prints two populations

`crates/pdf-model/examples/vertical_form_census.rs` is the instrument for the *population*
question, and it prints the clause's population beside the program's because trap 13's second
shape is that those are different censuses. The clause's — a `Type0` font stating writing mode 1,
with no embedded program, in a collection Table 116 publishes a vertical `CMap` for — is read out
of the files' own dictionaries and is the same on every machine. The program's — how many codes
those documents draw upright — is this machine's font catalogue, and says so.

That is what a zero needs to be legible. Over the 1251 curated documents the clause's population
is **two files** — the PDF Association's `VerticalText.pdf` and `doc/pdf.js`'s `issue11555.pdf` —
and this machine loses nothing on either, which on its own would have read as "no such defect
exists".

**The second of those two was missing from the census's first run, and that is trap 25 in a
census this round wrote.** The walk was `document.xref().object_numbers()`, copied from
`examples/hollow_glyph_census`, and `issue11555.pdf` writes its whole `Type0` — `/BaseFont
/KozMinPro6N-Regular`, `/Encoding /90ms-RKSJ-V`, no embedded program — **inline inside the page's
`/Resources`**, so it is not an object the table names and the census found no font in that
document at all. A population that misses what is there reads exactly like a clean corpus: the
first run said the pdf.js corpus states no substituted vertical font while the corpus's one
`90ms-RKSJ-V` document sat in it. `collect_type0` recurses into nested dictionaries, arrays and
stream dictionaries now — finite with no cycle guard, because a direct object is a tree — and the
`--pdfjs` run goes from finding nothing to naming `issue11555.pdf`. **`hollow_glyph_census` still
has the hole**, and §9.7.4.2's row says so rather than being re-run this round: its figures are a
floor.

**Over the 65 944 `CC-MAIN-2021-31` documents it is not zero**, and that is the finding this round
has that ADR 0152's 974-document arithmetic could not have produced: of 194 272 `Type0`
dictionaries in the 65 703 that open, 1312 state writing mode 1, 98 of those embed no program, all
98 name a collection with a published pair, and they sit in **42 documents** — of which **one**,
`7311602.pdf`, draws **33 codes** upright on this machine. `PDFVIEWER_TRACE_VERTICAL_FORM=1` names them, which is trap 11's rule
about reading what a condition matched: two characters, Adobe-Japan1's vertical CIDs 7923 (っ) and
7939 (ヶ), both of them small kana. So the shortfall is **per glyph rather than per face** — the
same machine supplies the brackets and full stops of `VerticalText.pdf` from the same feature —
and a design that had asked only "does this face state `vert`" would have answered yes and counted
nothing. The other 41 of the 42 lose nothing.

The population figures above are the *files'* and hold anywhere; the 33 is this catalogue's. Both
come off the run rather than out of this document — `doc/todo/21` §7 carries the command.

## The gates

`doc/todo/02` §2's sequence, on a quiet machine, with the census run before it rather than beside
it. The corpus gate gains a **fourth silence line** — `codes drawn upright where §9.7.5.1 named a
vertical form` — beside ADR 0270's two and ADR 0311's one, and it is a measurement rather than a
gate, exactly as its three neighbours are.
