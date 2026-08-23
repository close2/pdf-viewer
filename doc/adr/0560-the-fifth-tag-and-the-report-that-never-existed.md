# ADR 0560 — The fifth tag, the clause number a sibling had right, and the report that never existed

Status: accepted, 2026-08-24. Session the seven-hundred-and-first, a clause round under
`doc/todo/01`, reading one family's `partial` rows against each other as well as against the code —
ADR 0538's method, replicated by ADR 0551 one round earlier and by this one now. Amends §14.6,
§14.6.1 and §14.6.2 in the ledger; **moves §14.6.1 from `partial` to `implemented`**; adds one test
to `crates/pdf-model/tests/optional_content.rs`; corrects `Unsupported::MissingResource`'s doc
comment in `crates/pdf-model/src/content/report.rs`; adds one section and one correction to
`doc/errata-read.md`. Extends ADRs 0253, 0255, 0294 and 0475. No pixel moves and no report is added
or removed.

## 1. The family, and why it was the one to read

The blame ordering over `doc/conformance/ledger.toml`'s `note =` lines was re-derived on this base
rather than taken from the file that predicts it (616's rule). §7.6.4.4 is rank 1 and is ADR 0538's
family; §11.3.4 is 2 and §11.3.7, §12.5, §8.6.6, §8.9.6, §8.9.6.2, §9.8.3 and §9.8.3.1 share rank
3–9; §14.6, §14.6.1 and §7.7 follow at 10–12.

§14.6 was taken because **all three of its rows are `partial` and two of them state the same list**
— which tags this tree acts on by name — so the family carries a claim in duplicate, and a claim in
duplicate is where the fifth failure shape has somewhere to show. §11.3's rows were read for shape
first and left alone: they cross-refer heavily but consistently, and §11.3.7's account of what keeps
its two children `partial` agrees with both children's own.

## 2. What was wrong

| row | shape | was | is |
|---|---|---|---|
| **§14.6** | a clause number, twice, that its own sibling had right | "§8.11.3.3's optional content rides on `BDC`/`EMC`" and "`/OC` (§8.11.3.3's optional content)" | §8.11.3.2, *Optional content in content streams*, is the clause that puts the `OC` tag on a bracket. §8.11.3.3 is *Optional content in XObjects and annotations* — the `/OC` **entry**, a different mechanism with an `implemented` row of its own. §14.6.1's row, one line below, has written §8.11.3.2 all along |
| **§14.6** and **§14.6.1** | a count contradicted by the note's own previous sentence | "beyond the **four** this tree acts on by name: `/OC`, `/Artifact`, `/ReversedChars` and `/AF`" | **five**. `/Tx` is the fifth, and §14.6's own note names it one sentence earlier — §12.7.4.3 requires a processor to replace an appearance stream's contents "from / Tx BMC to the matching EMC", and `appearance::find_tx_marked_content` matches the tag by name to find where |
| **§14.6** | a reason that denies two whole clause families | `partial` because "§14.7's and §14.8's semantics are unimplemented" | the ledger's own rows deny it: §14.7.5's subclauses, §14.7.6's and the whole of §14.8.4 are `implemented`, and `Query::LogicalSelection` answers from the tree those rows describe. What keeps §14.6 `partial` is §14.6.2's debt and nothing else |
| **§14.6.2** | a claim about this tree, stated twice inside one sentence, that has never been true | "A name that `/Properties` does not define is *reported* — a name that `/Properties` does not define is *reported*, since …" | `content::resources::note_missing_resource` is called for `/Pattern`, `/ExtGState` and `/XObject` and for nothing else, in any commit. The duplication came in with the sentence's own rewrite: the replacement kept the clause it was restating |

### The fourth is the one where the code is right and the row is wrong

A report here would have been a defect rather than a repair. §8.11.3.2 says the marked content is
optional content "only if the tag is OC and the dictionary operand is a valid optional content
group", so a section whose operand `/Properties` does not define is *ordinary content* and is
drawn — which is what `run.rs` does, silently and correctly.

**And the comment beside the code had it right the whole time.** `Unsupported::MissingResource`'s
own doc comment says a missing `Properties` list "costs no mark at all" and names the reason: the
section's operators still draw. So this is `doc/todo/01`'s fifth and seventh shapes together — two
places about one mechanism, where the one that was corrected is not the one that was wrong, which
is the same sentence §7.8.3's row already wrote about this same pair of rows.

What an undefined property list *does* cost is real and is not a mark: §14.9's four entries,
§14.7.5.2's `/MCID`, an artifact's Table 363 entries and §14.13.5's files all arrive through
`content::marked::property_list`, and a `BDC` naming a list nobody defined loses every one of them
without a word. That is now written into §14.6.2 as part of what keeps it `partial`, rather than
implemented on the spot: the condition a report would fire on is trap 11's whole subject, and
`Unsupported` is the vocabulary for what could not be **drawn**. The comment was one turn behind in
the other direction — it named the two accessibility entries and the group and not the four readers
added since — and now names all of them, which is `doc/todo/01`'s standing observation that a list
of what *is* read decays exactly as a list of what is not.

## 3. The status that moved, and the argument for moving it

§14.6.1 was `partial` because "a tag that *is* a structure type" goes unread. **That is nobody's
requirement**, and the clause that would state it says the opposite in as many words. §14.7.5.2:

> Although the tag associated with a marked-content sequence is not directly related to the
> document's logical structure, it should be the same as the structure type of the associated
> structure element.

A `should`, addressed to whoever writes the file, over a relationship the sentence itself disclaims.
The association a reader is given is the `/MCID`, through §14.7.5.4's parent tree, which
`structure::element` walks and §14.7.5.2's row records as `implemented`.

So the amended §14.6.1 was enumerated modal by modal. A tag operand on every marked-content
operator but `EMC`; a property list on `DP` and `BDC`; a second-class name for a tag no ISO
publication and no logical structure defines (Issue #334's replacement paragraph); properly
separate nesting; a sequence contained in one content stream — every one of them binds a producer.
**One sentence binds a reader:**

> The Contents entry of a page object (see 7.7.3.3, "Page objects"), whether a single stream or an
> array of streams, is considered a single stream with respect to marked-content sequences.

`content::reader::ContentReader::for_page` pumps Table 31's parts into one window, so it holds — and
**nothing in the tree asserted it**. `optional_content.rs::a_marked_content_section_may_span_two_parts_of_the_contents_array`
does now: the `BDC` is the whole of the first part, so a reader taking each part as a stream of its
own paints the square the second part hides. Calibrated per trap 13 by moving the `EMC` back into
the first part, where the assertion fails with its own message; restored, it passes.

`implemented` on that reading. §14.6 stays `partial` for §14.6.2's debt, which is the arithmetic the
sixth sweep checks and is unaffected.

## 4. The errata, read before anything was written

`cargo run --release -p spec-errata -- emit doc/*.pdf` files six annotations on the two pages
§14.6.1 spans. Four were recorded — #126's *rolemapped*, #303's deleted NOTE 1, #334's deleted NOTE
3, #335's marked content within a text object. **Two were recorded nowhere, and the first changes a
requirement.**

**Issue #302, `Review/Completed`, adds two pairs to the properly-nested rule.** The 2020 sentence
names three — BMC…EMC, BDC…EMC and BT…ET — and the carets add the compatibility pair BX and EX and
the graphics state pair q and Q, on both sides of the sentence. Its entire strikeout is the word
*or*: one word, under `check`'s four-word floor, so that instrument is blind to it by construction.
This is the seventh consecutive round in which a bare or nearly bare caret has been the find.

**It is a licence rather than a debt**, which is `CLAUDE.md`'s §10.7.2 shape one clause family over.
§12.7.4.3 has `appearance::spliced` replace the bytes from a `/Tx BMC` to the balancing `EMC`. Under
the 2020 text a conforming file could open a `q` before that `BMC` and close it inside the sequence,
and the splice would have removed the `Q` and left the rest of the page in the saved state; under
the amended text it cannot. The algorithm does not change and its warrant does.

**Issue #301 is capitalisation** in Table 352's `BMC` row and moves nothing.

`doc/errata-read.md`'s §14.6.1 row also claimed a stale `variable_text.rs` comment about Figure 9
was "owed below" while the same document's settlement paragraph records it as done, in
`PERMITTED`'s doc comment. Corrected: one document, two paragraphs, the same disagreement this
round is about.

## 5. The instrument 697 asked for, measured and declined

ADR 0551 closed on a shape no sweep can see — a note contradicting itself — and asked whether one
could be built. The obvious construction is the eighteenth sweep with both sides inside one row:
`overstated::parts` to split the note, `overstated::terms_in` for the `/Key`s and `Table NNN`s,
`overstated::is_an_assertion` and `unread::is_a_claim` for the two stances, and a hit where one part
asserts a term another part denies. Every piece already exists and is public; the measurement took
one throwaway program, which is ADR 0481's method and trap 11's rule.

> **794 rows with a note; 259 assert a term; 930 assertions between them; 46 contradicted inside one
> note, of which 24 carry `capabilities::HISTORY`'s mark. Of the 22 unmarked, every one is noise on
> reading and none is a defect.**

Three noise shapes account for all 22, and two of them are structurally worse here than in the
cross-row sweep:

- **A part that names two terms with one stance each.** "`/Sy` is unread, and `/RD` is read but has
  nothing to qualify" asserts and denies inside one part, so both terms take both stances. Across
  rows this costs nothing — the other row still has to deny the same term — but inside one row the
  pairing is free, and it produced eight of the 22.
- **A table read in part**, the eighteenth sweep's own dominant noise, unchanged: §9.8's Table 120,
  §10.6.5's Table 57, §11.6.5.2's Table 143, §12.7.5.3's Table 231.
- **A correction narrating its retired wording** in words `HISTORY` does not carry — a `~~struck
  through~~ ` sentence beside its replacement (§12.7.5.5's `/P`, four hits), or a sentence beginning
  "`/EFF` was read by nothing until this session".

**The last of those is the reason not to build it, and it is a fact about the population rather than
about the vocabulary.** ADR 0523 made it this project's rule that a correction states the retired
claim in words the sweep matching it can still find — otherwise a repair takes a row out of the
population instead of moving it across. A note that has been corrected for a self-contradiction
therefore *contains* the contradiction on purpose, in both halves, for the next reader to check. So
an instrument whose whole subject is a note holding two opposed sentences has a population defined
to be dominated by the notes that were repaired.

**And it would not have printed this round's findings.** §14.6's contradiction is a *cardinal*
against an enumeration two sentences away, and §14.6.1's is a `partial` reason against a clause's
own modal verb. Neither is an assertion and a denial over a `/Key` or a `Table NNN`, which is the
only join a program has here. Written down as the answer, in `doc/todo/01`, rather than as a
program: **an intra-row contradiction is found by reading, and the reading list is the family.**

## 6. What this decides, and what it leaves

One status moves and one test arrives. No pixel moves; no report is added or removed.

What it leaves is the debt §14.6.2 now states in two halves rather than one: §14.3.2's object-level
metadata in a property list is read by nobody, and a `BDC` whose named property list `/Properties`
does not define loses four clauses' worth of entries in silence. The second is a candidate for a
report and is deliberately not one yet — it costs no mark, so the condition wants the argument trap
11 asks for before any code is written.
