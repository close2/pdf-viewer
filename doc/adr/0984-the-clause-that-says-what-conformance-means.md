# 0984 — The clause that says what conformance means, and the population that could not see it

Session 973. Status: **accepted**. It widens the conformance ledger's population to clause 6,
writes the eight rows that population was missing, and replaces the constant that decided the
population with a claim the standard checks.

## 1. The clause

ISO 32000-2 clause 6 is titled *Conformance*. It is the only place in the standard that says what
conformance means **for a program** rather than for a file, and it states eleven `shall`s:

| subclause | `shall`s | addressed to |
|---|---|---|
| §6.2 Conforming PDF documents | 1 | a PDF file |
| §6.3.2.1 General | 5 | a PDF writer, and a PDF processor |
| §6.3.2.2 PDF processors providing rendering | 3 | a processor that renders a page |
| §6.3.2.3 Interactive PDF processors | 2 | an interactive processor |

`CLAUDE.md`'s *what done means* is written around the third row of that table:

> §6.3.2.2 places three obligations on a rendering processor: render the page contents as
> defined, respect the default or user-specified optional content configuration (§8.11), and draw
> the appearance stream of every annotation whose flags call for one (§12.5.3, §12.5.5). Where
> that ordering disagrees with the corpus's, the specification's wins.

**And the ledger had no row for any of it.** Forty-seven citations of §6.3.2.2 stood in
`pdf-font`, `pdf-model`, `viewer-core` and their tests — the clause is read, implemented and
tested — with nothing in the project's coverage denominator recording that it had been.

## 2. Why no instrument could say so

`conformance::ledger` decided its population with a constant:

```rust
pub const TECHNICAL_CLAUSES: std::ops::RangeInclusive<u16> = 7..=14;
```

and the two checks that would otherwise have fired both walk `covered()`, which is that constant:

- `Problem::MissingRow` reports a subclause with no row — **for the subclauses in the
  population**.
- `Problem::CitedButUnreviewed` reports a clause the code cites whose row is `unreviewed` — by
  iterating the population and looking for citations of each, so a citation of a clause outside it
  is never examined.

So the instrument's population was its own output. A clause the constant did not name had no row,
and having no row is exactly what made it invisible: nothing was missing, because nothing was
expected. This is the shape `doc/todo/02` §1 warns about in a different register — the map is not
the territory — and it is why the fix is not "add clause 6 to the constant".

**Where the constant came from is worth recording, because it was reasonable.** `CLAUDE.md`'s
scope section is organised by clause, and it names 7, 8, 9, 11, 12, 10 and 14 — the clauses that
decide what a page looks like. Clause 6 decides nothing about a page. It decides what the program
*is*, which is why the scope section quotes it and the population did not contain it. ADR 0206 hit
the same seam from the other side when it found the eight normative annexes outside every
instrument here, and its sentence applies unchanged: "`CLAUDE.md`'s scope section names clauses
because that is how the standard's *body* is organised, and its closed exclusion list says nothing
about an annex".

## 3. The fix: a population the standard decides

The constant is replaced by three lists and a check that reads the standard against them.

```rust
pub const NORMATIVE_CLAUSES:   [u16; 9]       = [6, 7, 8, 9, 10, 11, 12, 13, 14];
pub const NORMATIVE_ANNEXES:   [char; 8]      = ['D', 'E', 'F', 'I', 'K', 'L', 'O', 'Q'];
pub const INFORMATIVE_ANNEXES: [char; 9]      = ['A', 'B', 'C', 'G', 'H', 'J', 'M', 'N', 'P'];
pub const EXCLUDED_CLAUSES:    [(u16, &str); 1] = [(4, "…")];
```

`check` now counts the word `shall` under every top-level clause and every annex of
`doc/md/ISO_32000-2_sponsored_EC3.md` and reports two things:

- `Problem::UnrecordedRequirement` — a clause that states `shall` and is in none of the four
  lists, or an **annex in none of them at all**, whatever it states. An annex letter nobody has
  classified is a letter no instrument here can see, which is where all seventeen of them were
  before ADR 0206.
- `Problem::StaleExclusion` — an entry of `EXCLUDED_CLAUSES` whose clause states no `shall`. Trap
  25 in reverse: an excuse for a clause that has stopped needing one reads as a pass, because the
  check it silences would have found nothing there either.

A heading's span in `ClauseIndex` includes its subclauses', so counting over spans would count one
sentence once per level it sits under; `requirements_by_group` counts the text from each numbered
heading to the **next**, which partitions the document exactly once. A test holds that.

Per trap 13 the check is calibrated against the defect before it is believed: five unit tests, two
of them a pair — a clause outside the population that states `shall` **is** a finding, and the
same clause stating nothing **is not**. Without the second the check would pass for every clause
of the standard and mean nothing.

### What the standard's own counts say about the four lists

| group | `shall`s | where it goes |
|---|---|---|
| 0, 1, 2, 3, 5 | 0 | nothing to cover |
| 4 Notation | 19 | `EXCLUDED_CLAUSES`, argued below |
| 6 – 14 | 5 616 | the population |
| A, B, C, G, H, M, P | 0 | informative |
| J, N | 22 | informative — see below |
| D, E, F, I, K, L, O, Q | 264 | the population |

**Clause 4 is the one judgement call**, and it is written into the list beside the clause rather
than left implicit. §4.1's single `shall` binds the standard's own prose — a token character is to
be named "by their INCITS 4-1986 (R2017) (ASCII 7-bit USA codes) character name written in upper
case". §4.2's eighteen are all of one form: "[a]ny use of the term X throughout this document
shall be inferred as referring to" some other standard. The second eighteen genuinely bind this
program — they decide which edition of IEC 61966-2-1 `sRGB` means, which parts of ISO/IEC 10918
`JPEG` means, which Adobe collection `Adobe-Japan1` means — but never at a site of their own: each
is discharged in the clause that uses the term, and a row here would be a second place to keep
that in step. That is an argument, not a certainty, and the next round to disagree with it has one
line to edit and a check that will not let the disagreement be silent.

**Annexes J and N print `shall` and are informative, which is not a contradiction.** ISO/IEC
Directives Part 2 makes an informative annex a place for information, so a `shall` there restates
a requirement stated normatively elsewhere — which is exactly how J.3.1 writes it: "Clause 7.3.2,
"Boolean objects" clearly states that the keywords shall be true and false". The rows those
sentences belong to are clause 7's and clause 10's. `INFORMATIVE_ANNEXES` exists so that the check
can tell an annex this project decided about from one nobody has classified.

## 4. The eight rows

Read against the code, not generated:

| row | status | what it turned on |
|---|---|---|
| §6.1 General | `implemented` | introductory; the ledger itself is the answer |
| §6.2 Conforming PDF documents | `writer-side` | the `shall` is on a *file*; the two writing paths meet it |
| §6.3 PDF processors | `implemented` | a heading; it decides that this program is three of §6.3.1's kinds |
| §6.3.1 General | `implemented` | definitional, plus the `may` that offers Annex O — taken |
| §6.3.2 Conformance of PDF processors | `implemented` | a heading; the three-level ordering `CLAUDE.md` uses |
| §6.3.2.1 General | `partial` | four of five `shall`s met; the fifth is §7.6.4.1's standing debt |
| §6.3.2.2 PDF processors providing rendering | `implemented` | all three obligations, including the clause's own "unless otherwise instructed" |
| §6.3.2.3 Interactive PDF processors | `implemented` | §8.11's interactive aspects, `/Locked` included |

Three of them are worth more than a row of a table.

**§6.3.2.1 is `partial` for one sentence**: "A PDF processor shall respect and honour standard
security (see 7.6.4, "Standard security handler") when it opens a document for processing." That
is the same debt §7.6.4.1's row already carries out loud — filling a field and adding an
annotation are refused where Table 22 withholds them; printing, assembling and bit 5's copying are
not gated — so the new row points at it rather than restating it. `pdf_model::restriction::Level`
lets the reader switch all of it off, which `CLAUDE.md` principle 3 requires of a program that is
the reader's, and `viewer-core` defaults to `Level::On`, so what a document gets out of the box is
the clause's answer. Recording the departure *as* a departure is the point: a row that said
`implemented` here would have hidden the one place this project deliberately declines a `shall`.

**§6.3.2.2's third obligation ends "unless otherwise instructed"**, and that is the clause's own
exception rather than a licence this project took — it is what lets a host draw a form field
itself and take the field's picture off the page (`WidgetAppearances`). `pdf_model::view::Rendered`
is shaped so that a caller who asks for nothing gets the page the clause describes, which is what
makes the corpus gate and the oracle measurements of a *conforming* rendering rather than of one
configuration of ours.

**§6.3.2.3 is the only clause in the standard addressed to an interactive processor as such**, and
this program is one by §6.3.1's definition. Its second `shall` — "such a processor shall support
all of the interactive aspects of optional content" — is met: §8.11.4.3's `/Order` is the layers
panel, `Command::SetOptionalContentGroup` switches a group by object identity, the switch
re-decides what the page draws rather than re-filtering a raster, and a locked group's switch is
drawn and does nothing.

## 5. A second finding, from the same method

Reading the new §6.3.2.1 row's quotations back against `doc/md/` found one that is not the
standard's. Twenty-three Annex F rows carried, in quotation marks and attributed to §F.1:

> shall be a conforming file

The phrase **occurs nowhere in ISO 32000-2**, and the word `conform` occurs nowhere in its Annex F
at all; `conforming file` is ISO 32000-1's vocabulary and appears three times in
`doc/md/ISO_32000-1_2008.md`. What the rows wanted to say, F.1 says better and in as many words:
"A PDF processor that does not support this optional feature can still successfully process
linearized files although not as efficiently." All twenty-three now quote that sentence, and the
inference they draw from it — the hint tables are an accelerator a reader may ignore — is stronger
for resting on a sentence the standard contains.

`cargo run -p conformance --bin quotations` already reads this file's notes and is where the rest
of that population is counted; it prints the count, this ADR does not. **The lesson is the round's
own**: a quotation in a ledger note is checked by a *report* rather than by a gate, so a
paraphrase wearing quotation marks survives there exactly as long as nobody reads the report — the
same way the §7.6.4.1 row records two hosts quoting a `should` as a `shall` for two hundred and
eighty-five sessions. That belongs in `doc/habits.md`'s *The ledger, and claims about this tree*
and is flagged for session 972, which owns that file this round.

## 6. What this does not do

- **It does not widen the ledger to clauses 1, 2, 3 and 5.** They state no `shall`, and the check
  now says so mechanically rather than on anyone's memory.
- **It does not add a check reading the citations against the rows.** One was written and removed:
  with the population fixed it is subsumed by `MissingRow` for every clause it could honestly fire
  on, and outside the population it fires on noise — `§5a`, `§3a`, `doc/todo/02 §2` and
  `RFC 0002 §6.1` all parse as ISO 32000-2 citations today. **That is a live defect and it is
  reported rather than fixed here**: `citation::ForeignCitation` catches only a document named
  immediately before the `§`, so roughly a hundred and thirty citations of "clauses" 1, 2, 3 and 5
  in this tree are other documents' sections, and `every_citation_names_a_clause_that_exists`
  passes them because those clause numbers exist. `CLAUDE.md` and `doc/todo/02` both state that
  `§N` means ISO 32000-2 and nothing else, and that `cargo test -p conformance` enforces it; for
  this shape it does not. Whoever takes it owns a scanner change and sites in five crates.
- **It does not touch the four levels** of `doc/todo/38`. §6.3.2.1's row names them as the place
  its `partial` is answered, which is where that item already stood.
