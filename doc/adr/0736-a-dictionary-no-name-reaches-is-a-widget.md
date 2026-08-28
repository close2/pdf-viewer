# 0736 — A dictionary no name reaches is a widget, and the cell that caught up with its paragraph

Status: accepted.
Context: the errata selection rule's fourteenth use — the first run on the family guard ADR 0732
added to step 3, and the fifth consecutive use whose base count reproduces the previous use's
closing arithmetic.

## The rule, unchanged

ADR 0627's rule with ADR 0637's repair to step 2, ADR 0653's tie-break, ADR 0671's fourth step,
ADR 0691's writing rule, ADR 0712's placement rule and ADR 0732's family guard:

> Rank each ledger row by the errata annotations that fall on it whose issue number this tree
> names nowhere, attributing a heading only to a row in the heading's **own** family. Rank once
> over the live rows and once over **every** row, take the head of the two, and prefer the settled
> row where they tie. Reassemble the issue from every clause `emit` files it under, and read the
> issue whole — and a verdict written under a heading is a claim about a page, not about a clause,
> until the rectangle has been placed.

Of the issue numbers in `doc/ISO_32000-2_sponsored_EC3.pdf` that carry a strike or a caret under
the recipe's own single-issue line parse, **71 were named nowhere at this round's base**, of a
population of 302 — the thirteenth use's closing arithmetic, 73 less its two verdicts, reproduced
by the greps rather than quoted from the record. The multi-issue parse counts 310 and 73, which
are likewise the thirteenth's figures less the same two.

## The guard is what chose this round's work

ADR 0732 built the family guard after Annex A's four annotations had made a six-annotation head
out of `§14.13.10`, an `inapplicable` row with two of its own. The repair was to the recipe; the
collection has not moved, so **the unrepaired instrument offers the same false head here**, and
running both is what says the guard does what it was built for:

| | full ranking's head |
|---|---|
| without the family guard | §14.13.10, `inapplicable`, six — four of them Annex A's |
| with it | six rows tied at three, §14.13.10 holding two |

Twelve annotations fall in the state the guard counts separately rather than attributing, and
they reproduce the thirteenth use's own split: 4 under Annex A, 2 under Annex H, and 6 under
clauses 2 and 3, which the ledger starts after.

With the guard, over live rows one row reaches three — **§12.7.4.1**, `partial` — and over every
row six do, one of them §12.7.4.1 itself. Step 4 prefers the settled row on a tie and ADR 0653's
tie-break picks the row the ranking calls §13.7.2.3.2, whose caret turns a table cell's
`(Optional;` into `(Required;` where the other four settled rows move an example's syntax, a
linearisation version, a spelling and a clause's account of its own history. That head is inside
`CLAUDE.md`'s clause-13 exclusion — and its annotations are Table 341's, which is §13.7.2.3.1's,
the outline having filed them one clause late — so it confirmed its row and paid nothing. **The
walk downward therefore ran to the live head**, which is the row ADR 0732 deliberately left in the
population as this use's first candidate.

## The finding: Table 226's `/T` contradicted §12.7.4.2, and the code was one degree short of it

Table 226 prints `/T` as

> (Required) The partial field name (see 12.7.4.2, "Field names").

and §12.7.4.2, two subclauses along, states

> A field dictionary that does not have a partial field name ( T entry) of its own shall not be
> considered a field but simply a Widget annotation. Such annotations are different
> representations of the same underlying field

— a sentence about a case the table forbade. **Issue #28 takes the contradiction out on the
cell's side**: a StrikeOut over the `(Required)` with a Caret writing *Optional*. It is the shape
ADR 0728 met one round apart on §12.10.3, where a clause's paragraph contradicted its own table
and the erratum amended the paragraph; here it is the table that gives way, and the placement is
in `doc/errata-read.md`.

What the amendment changes for a reader is **which files are conforming**. While `/T` was required
a dictionary stating none was malformed, and what this tree did with one was a repair; now it is a
conforming file, and §12.7.4.2's sentence is what decides it.

`pdf_model::view::widgets_by_field_name` quotes that sentence in its own doc comment and applied
it to a **kid**:

- a kid with no `/T` joins its parent's name, which is right, and is what
  `one_field_name_may_reach_several_widgets` has pinned since ADR 0245;
- a dictionary reached from `/AcroForm /Fields` with no `/T` and **no ancestor stating one** was
  keyed under the *empty* name.

The sentence is unconditional and the second case is the one it was written for: with no ancestor
there is no underlying field, so the dictionary is a widget and nothing more. Keying it by the
empty name made a **field out of annotations sharing nothing** — `form::fields` grouped every such
widget in a document under one name and offered a host a single control over them, and
`ViewState::set_field` under that name wrote one value into all of them.

Two corrections, one function:

- **A node whose ancestry states no `/T` is left out of the table.** It is still drawn —
  `crate::annotation` needs no field — and handed to nobody, which is the path `form::fields`
  already documents for a widget the field tree does not reach. **The test is the entry and not
  the string**: a `/T` of zero length still names a field, because §12.7.4.2 conditions on the
  entry's presence, and two fields sharing a name are the case the same clause makes the *writer*
  answer for.
- **A `/Fields` entry stating a `/Parent` takes its ancestors' name.** §12.7.3 makes that array
  "[a]n array of references to the document's root fields (those with no ancestors in the field
  hierarchy)", so an entry carrying a `/Parent` has contradicted the clause — and Table 226's
  `/Parent` is the file's own statement of which field it belongs to, which §12.7.4.1's
  inheritance walk has trusted for `/FT`, `/Ff` and `/V` since it was written. **This one is a
  recovery rather than a requirement and is recorded as a choice** in §12.7.4.2's and §12.7.3's
  rows.

## The population says which half the corpus witnesses

`crates/pdf-model/examples/unnamed_field_census` walks `/Fields` with `pdf_syntax` alone — never
through the function under test, which is trap 8 — and counts four widths: every dictionary
stating no `/T`, those no ancestor names either, documents with two or more such leaves, and the
nameless roots stating no `/Parent`. Over `doc/pdf.js` and `doc/corpora`:

- **1239 documents open**, 176 with an `/AcroForm` stating a `/Fields` array, and 19 with a field
  dictionary stating no `/T`;
- **1** has one no ancestor names — `doc/pdf.js/test/pdfs/opt_demo.pdf`, which lists a radio
  group's two *buttons* in `/Fields` instead of the field above them, and which is therefore the
  one document where two nameless leaves shared the empty name;
- **0** state a nameless entry in `/Fields` with no `/Parent` on it.

So the recovery is what the corpus witnesses and the refusal is what it does not. On the witness
the table goes from `"" -> [22, 23]` to `"veg" -> [22, 23]`, measured on `main`'s function and on
this one; the refusal is pinned by a fixture, which is what a population of zero asks for.

## Calibration

Trap 13, above the commit that makes the change, three plantings and both directions:

| planted | fails |
|---|---|
| a nameless node keyed under the empty name again | `a_root_field_that_nothing_names_is_not_a_field`, `left: [""]` |
| the `/Parent` ancestry not consulted for a root | `a_root_field_takes_the_name_its_parent_chain_states`, `left: None` |
| a name **dropped for being empty** rather than absent | `an_empty_partial_name_is_still_a_name`, `left: None` |

The third is the over-correction, and it is why the pair exists: a reader that asked whether the
name came out empty instead of whether the entry was there passes the first two tests and loses a
field whose partial name is the empty string.

## Consequences

- `view::widgets_by_field_name` answers about fields only, so `form::fields`,
  `form::delegated_widgets`, §12.6.4.11's hide action, §12.7.6.3's reset and §12.7.8's import all
  stop reaching a dictionary that is not one. None of them loses a widget's *appearance*: a widget
  no field claims is drawn by `crate::annotation`, which is the same path it took before.
- A `/Fields` entry that contradicts §12.7.3 is read through its `/Parent` chain, bounded by
  `MAX_FIELD_DEPTH` for the reason the inheritance walk is bounded: a chain in a hostile file can
  be a cycle.
- Ten errata gain verdicts, taking the rule's population to 61. §12.7.4.1, §12.7.4.2, §12.7.3,
  §14.2 and §14.9.4 carry theirs in the ledger; `doc/errata-read.md` has all eight rows.
- **A fifth blindness is recorded**, and it is the first the `emit` half of the instrument shares:
  an erratum whose substance is an attached file rather than text. `check` sees nothing struck and
  `emit` prints the annotation's title, so the collection's railroad diagrams for §7.3.3 reached
  neither — the eight-hundredth session read them anyway (ADR 0733), which is what makes this a
  shape to write down rather than a debt.
