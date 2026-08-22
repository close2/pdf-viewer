# ADR 0493 — The population a flag could not reach

Status: accepted, 2026-08-22. Session the six-hundred-and-sixty-seventh, a clause round under
`doc/todo/01`'s binding rule, continuing the seventh step of its technique. Amends §10.7.5's,
§11.3.5.2's, §11.5.3's, §12.5.3's, §12.5.6.6's and §12.5.6.7's ledger rows, and adds a population
flag to `examples/witness_census`, a blend-mode question to `examples/luminosity_mask_census` and
Table 167's flags to `examples/spec_annotation_census`. Extends ADR 0490; changes nothing ADR 0217,
0403, 0431 or 0490 decided.

## 1. What this decides

ADR 0490 established that **a ledger negative measured before `CC-MAIN-2021-31` arrived is a
negative nobody has measured**, and told a round to re-derive one over the crawl as well as over
the control. Six rows were re-derived here and two turned out false. What the round adds is
narrower than the rule and is the reason four of the six could not have been checked before:

> **An instrument has a population, and a claim can only decay as far as its instrument can
> reach.** `examples/witness_census` — the sweep built in the five-hundred-and-seventieth session
> precisely so that absence claims could be re-run — had `doc/pdf.js`, `doc/corpora` and this
> project's own fixtures hard-coded as its scope. §12.5.6.7's row therefore said "**[n]o document
> in any population this project measures** states `/Cap true`", which was a true sentence about
> the census and read as a sentence about the world.

So the flag comes first and the reading second: `--crawl` is on that census now, beside `--pdfjs`,
as a third scope rather than a fourth root, because ADR 0490's control-and-growth pair needs the
two answered apart.

**And a name census is not a structural one, which this round paid for in the same hour.**
`witness_census --crawl CL` reports 81 crawled documents stating `/CL`; a spot check of four found
three where `/CL` is an `/XObject` or `/Font` resource key and one where it is Table 177's callout
line. ADR 0403 says this in its own words — "a name being present is not the structure being
present" — and the number that went into §12.5.6.6's row is `free_text_census`'s **33 of 1724 free
text annotations**, from the walk that knows what a callout line is.

## 2. The two rows that were false

**§12.5.6.7's `/Cap true`.** The clause's caption — "the text specified by the Contents or RC
entries shall be replicated as a caption in the appearance of the line" — has been drawn since the
five-hundred-and-seventy-fourth session (ADR 0431) against hand-built fixtures, on the stated
ground that no document anywhere states the entry as `true`. Over the crawl, four documents state
`/Cap` as a name and **two write `true`**: `cc-main-2021-31/1530/1530384.pdf` and
`2514/2514866.pdf`, twenty-two `/Subtype /Line` annotations apiece, each carrying the `/CO` offset
pair this clause places the caption by. The construction has producers. (One of the two also writes
`/Cap` on a `/Polygon`, where Table 181 defines no such entry — a producer's own business, and not
this row's.)

**§12.5.6.6's `/CL`.** "`examples/free_text_census` counts 0 of 73 free text annotations stating a
`/CL`" is right about the corpus and wrong about the world: the same census over the crawl finds
**33 of 1724 over 270 documents**. The callout line, its `/LE` ending and Table 177's
`FreeTextCallout` intent were defended by hand-built pairs alone (trap 8) and now have files.

## 3. The four that held, and what each one gained

Three of the four were confirmations, and a confirmation is only worth the run when the run is
different from the one that produced the claim. Each of these is:

- **§11.5.3's blend mode inside a mask group of more than one subtractive component.** The row said
  "a report with no corpus member" and §11.3.5.2's said "with no corpus document stating one" —
  the same claim in two homes, ADR 0101's shape, with no population and no command behind either.
  `luminosity_mask_census` now counts exactly what `note_blended_luminosity` fires on, and the
  answer is **0 of 1126 curated and 0 of 65 703 crawled**, against 41 and 21 834 `/DeviceCMYK` mask
  groups. **The zero is a measurement rather than an empty walk**, which is why the census prints
  the blends it finds in *any* space beside the ones it finds in that space, and why a planted
  witness was run through both the census and `interpret` before the number was written down.
- **§12.5.3's Table 167 bit 9.** The old claim was "a scan of every **uncompressed** `/F` in all
  974" — a statement about a byte search, blind to any annotation inside a §7.5.7 object stream.
  `spec_annotation_census` counts the ten flags through the object model, and over **806 668
  annotations in 66 829 documents, 343 591 of which state an `/F`, not one sets `ToggleNoView`**.
  The claim survives on a population sixty-eight times the size and on a walk that could have
  disproved it.
- **§10.7.5's stroke adjustment, ranked for the first time.** The row is `partial` for the clause's
  first requirement, declined as a departure of §10.7.4's family; its only number was "49 corpus
  documents set the parameter true". **19 211 of the 65 703 crawled documents state `/SA`** — call
  it a third of the world asking the question. Nothing is implemented and the argument is unchanged;
  what changed is that the refusal now has a size.
- **§12.5.6.7's cited test**, checked under `doc/todo/01`'s fifth step and passed:
  `a_captioned_line_draws_its_contents_where_cp_states` states `/Cap true` with `/CP /Top` and
  measures ink in the band above the line and none in the band below it, which is the sentence the
  row makes about it.

## 4. The instrument, before and after

ADR 0485's habit. Twelve sweeps run before the edit and after it, ledger-only for the reading.
**Every hit count is unchanged** — `overstated` 8 contradicted with 7 marked, `counts` 4 counting
one family twice, `quotations` 1 diverging, `tables` 6 denials contradicted and 98 absent, `unread`
69 rows / 182 keys, `entries` 177 reported over 49 rows, `pointers` 118 absent and 13 undefined, and
`blockers`, `capabilities`, `inapplicable` and `callers` at their standing populations. The
levels that moved are the sentences this round added: `counts` 6761 → 6767 governed sentences,
`quotations` 1765 → 1767 ledger quotations, `tables` 5805 → 5807 sentences naming a table,
`pointers` 7045 → 7053 paths.

**One level moved that needed reading, and the answer is 663's**: `owed` went 181 unnamed terms over
114 rows to 182 over 115, and the row that left the reading list is §11.5.3's. The term it gained is
**`luminosity`**, and no ledger sentence names it as a debt — the sweep's key extractor reads a
solidus followed by letters as a `/Key`, so the citation `examples/luminosity_mask_census` becomes
the key `luminosity`, which no source writes as `/luminosity` or `"luminosity"`.

**That is a standing noise shape rather than a new one, and naming it is the finding.** Every row
citing an `examples/<word>_census` path pays the same phantom: `examples/border_precedence_census`
yields `border`, which no source names either. So a round that follows `CLAUDE.md`'s "write down
the command" rule inside a `partial` row moves this sweep's level by one, for free, every time. The
repair is not to drop the citation — that would be the instrument deciding what the ledger may say,
which is exactly what ADR 0490 §6 refused — and it is not to teach the extractor about paths
either, because a `/Key` and a path segment are the same characters and the discriminator would be
a guess. It is to know the shape, which is why it is written here.

## 5. Consequences

- `witness_census` grows `--crawl`, so the next round re-deriving a negative has the growth
  population as a flag rather than as a script.
- `luminosity_mask_census` answers §11.5.3's residue question and prints its own control.
- `spec_annotation_census` counts Table 167, which nothing in this tree did.
- `doc/todo/01` gains the `owed` noise shape above and the six rows' state.
- **Nothing's status moved**, and that is right: two rows lost a false sentence about the world and
  four gained the evidence they rested on. What a clause requires did not change.
