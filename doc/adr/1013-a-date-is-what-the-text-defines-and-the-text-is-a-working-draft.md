# 1013 — A date is what the text defines, and the text is a working draft

Session 993. Status: **accepted**. Recorded under the questions-directory rule: `doc/questions/A53`
appeared and this is the round that acts on it. It takes `Lexical::Date` off the XMP
Specification's six profiles of ISO 8601 and onto ISO 8601 itself, read from the working draft
the owner obtained; keeps the `GPSCoordinate` check the same answer confirms; and, for `A60`,
writes down what "cheap to adapt to part 3" means for `crates/pdf-archive` in the terms of the
code rather than of a wish.

Context: `crates/pdf-archive/src/iso_8601.rs` (new), `src/table/metadata.rs`, `examples/dates.rs`
(new), `doc/third-party-data.md`; ISO/WD 8601-1 (ISO/TC 154/WG 5 N0038, 2016-02-16) clauses 2.2,
3.2 to 3.6 and 4.1 to 4.3; `TechNote 0010` A020; ADRs 0931, 0933, 1010; `doc/questions/Q53`,
`A53`, `A46`, `A60`.

## 1. What the question was, and what the answer settled

`TechNote 0010` A020 resolves that an XMP value is validated on its type alone and lists the rule
per basic type: `Date` by ISO 8601. Session 942 took the item (ADR 0933) and left two points open
in `Q53`. `GPSCoordinate` is a basic type the list never mentions — does a silence withdraw the
check? And `Date` is given as a standard this project did not hold, so the check was the XMP
Specification's six profiles *of* that standard, and everything in some other of its forms — the
basic `20260910` was the example — was refused against A020's own rule. `Q53` named three ways
out and chose to leave it, with principle 5 refusing the second: widening the check by hand to
forms "a reader can enumerate without the text" is implementing a standard from memory of it.

`A53` answers both. The `GPSCoordinate` check stays: a resolution that names a list of types and
not this one withdraws nothing about it, which was ADR 0931's third condition applied to a
silence, and the owner confirms it is the reading to keep. And the owner downloaded the text —
the option `Q53` listed third — so the widening is done **from the text alone**, which is the only
way principle 5 lets it be done at all.

## 2. The text is a working draft, and that shapes every citation

What is held is **ISO/WD 8601-1**, a working draft of *Part 1: Basic rules*, and its cover says
in as many words that it is not an ISO International Standard and may not be referred to as one.
Two consequences, both carried:

- **Every citation names the WD, by its document number and clause**, never "ISO 8601-1:2016".
  The module comment of `iso_8601.rs` says what is held and what is not, and `A53` records that
  the published part is the purchase to make on the day a difference between the two matters.
  Nothing in this round found such a difference, and nothing in this round could have: one
  document was read.
- **Nothing quotes it.** The copyright page grants reproduction to participants in the standards
  process and to nobody else. So the grammar is written in this crate's own words under the
  draft's clause numbers — the discipline `doc/pdfa/`'s reprints, ISO 16684-1's preview and
  `TechNote 0010` already impose — and `doc/third-party-data.md` carries the row. The draft's own
  example *values* (`19850412`, `1985-W15-5`) appear in tests: a date is data, not prose.

## 3. The grammar, and three readings it needed

`iso_8601.rs`'s module comment is the grammar — a table of the date forms of clause 4.1, the
time-of-day forms of 4.2 as 4.3 combines them with a date, the element ranges from 4.1.2.1,
4.1.3.1, 4.1.4.1 and 4.2.1 over the calendars of 3.2, and the two combination rules of 4.3.3.
It is not repeated here. Three decisions inside it are not in the text and are this ADR's:

**A form is admitted when the body defines it; a NOTE defines nothing.** The draft's notes are
informative, and two of them would have changed the check: 4.3.2's NOTE lets the `T` be omitted
by agreement, and 3.4.1's NOTE 1 lets lower case stand in where upper case is unavailable. Neither
is admitted. `2016-02-01 13:19:21` is refused for its space (3.4.1's *body* forbids one) and
`…t10:15:30z` for its letters. Every value of those shapes in the corpora was refused before
too, so this is a reading rather than a movement.

**A form the body permits "by mutual agreement of the interchange partners" is admitted.** The
expanded years of 4.1.2.4, 4.1.3.3 and 4.1.4.4, the years before 1583 of 4.1.2.1, the proleptic
calendar of 3.2.1. A validator is not a party to any agreement and cannot tell that one is absent;
refusing what the text defines would be the false failure that `Q53` called the wrong direction
for this crate. One shape of that is worth naming: in basic format an expanded date is a sign and
a run of digits whose split between year and day is the agreement's — `+0019850412` is a calendar
date with a six-digit year or an ordinal date with a seven-digit one — so `Date::Agreed` admits
the run as whichever expanded basic form its length allows, and 4.3.3 c)'s "complete" is answered
as *some complete reading exists*.

**The element ranges are the calendar's, and the calendar is computed from the text.** A month
past twelve, a 30 February, a 366th day of a common year, a week 53 of a year that has 52, an hour
of 24 on a time point — each is a value the draft does not define a form for, and each is refused
by name. Two of them needed arithmetic the text supplies rather than states: the month lengths
and leap rule of 3.2.1, and the number of weeks in a year from 2.2.10's definition of the week
number (the first week holds the year's first Thursday) together with 3.2.2's reference point (1
January 2000 is a Saturday). The year is folded to its residue modulo 400 before either is asked,
because the Gregorian calendar repeats every 400 years — 146 097 days, exactly 20 871 weeks — so a
year of any agreed length costs nothing and overflows nothing. **Nothing in either corpus
exercised a range refusal**: every calendar date any producer wrote is a day that exists.

## 4. What moved, by name

`examples/dates.rs` is the instrument and it is kept: a census of every scalar of every
`Date`-typed property in every metadata stream of a corpus, classified by the form it is in the
draft's own notation or the refusal it earns. Run before and after over the two tracked corpora
(`doc/pdf.js`, `doc/veraPDF-corpus`) and over the four `doc/corpora/` submodules, and the history
file has both tables. Eleven values moved, all from refused to admitted, on two forms:

- **Local time, no zone designator** (4.3.2's empty designator): `xmp:CreateDate` in pdf.js's
  `ZapfDingbats.pdf`, `issue12120_reduced.pdf`, `issue18032.pdf`, `issue20232.pdf` and
  `issue20453.pdf`. The XMP Specification's profiles require a designator; the standard does not.
- **A difference from UTC in hours alone** (4.2.5.1's `±hh`): `xmp:CreateDate` and
  `xmp:ModifyDate` in veraPDF's `6-2-4-1-t01-pass-a` and `6-2-4-1-t01-pass-b` — both **PDF/A-4e**
  witnesses — and in `7.1-t06-fail-a`, a PDF/UA-1 witness, each of the shape `…T11:54:39.000+03`.
  The first two are files their author built to conform, and the reason they were never an
  `over` is worth stating: `metadata/properties-use-known-schemas` binds part 2 alone (part 4
  replaces section 6.6.2.3's machinery with one sentence pointing at ISO 16684-2), so under the
  target their directory names the date check never ran on them. Had the same producer's file
  been placed under `PDF_A-2b`, it would have been.

Nothing moved the other way: every value admitted before is admitted now, which the profiles
being forms of the standard guaranteed and the census confirmed. What stays refused is refused by
name — `D:20221116191452+00'00` for the `D` where a year begins, `…21Z+01:00` for the plus after
the UTC designator, `…TIME13…` for ending where an hour was expected, `Date: …` for its space —
and the finding prints that phrase after the value, because "an ISO 8601 date" names a standard
with a hundred forms and a reader should not have to open it to see which one a value is not.

**What the corpora write is narrow.** Across both populations the admitted values fall into
thirteen forms, and all but seventy-nine of them are the profile `YYYY-MM-DDThh:mm:ss±hh:mm` or
its `Z` variant; the history file has the tables. No producer in either population writes a
basic-format date, an ordinal or week date, an expanded year, a comma as the decimal sign, or a
time of day reduced to the hour. The widening is worth having for A020's
sake — the rule is the standard's — and the corpus says it will seldom be exercised, which is
`CLAUDE.md`'s two denominators answered separately, as they should be.

## 5. What did not change, and why

- **What a packet is written in.** `pdf_model::xmp` still emits the XMP Specification's profiles,
  ISO 16684-1's subset of ISO 8601, and nothing here asks it to emit anything else. The widening
  is on the admitting side alone, which `A53` states outright. Should the converter ever want
  another form — a local time, say — `xmp.rs`'s date writer would need the form and a reason,
  and `iso_8601.rs` already admits the result.
- **`Lexical::Coordinate`.** Kept, on the answer's first sentence.
- **`coverage.rs`.** No `says` there cites the date type: ISO 19005-2 section 6.6.2.3.1's sentence
  is carried by `metadata/properties-use-known-schemas` and says nothing about dates, so the
  reading is unchanged and the frontier still prints `none`. The ground for the date type lives at
  the row's `Lexical` and in the module, where a reader of the finding is sent.
- **The corpus harness.** Byte for byte unchanged: `over` is zero on all six targets before and
  after, the one `missed` is the standing `6-6-2-3-3-t03-fail-b` (errata A029), and no other
  column moved. That is not the widening failing to bite; it is the two denominators again, as
  ADR 0933 met them with A002. Every value the census moved sits in a file held to a part the row
  does not bind or to no PDF/A target at all, so the robustness instrument cannot see a change the
  coverage instrument — the census over every packet, whatever the target — sees plainly. Which
  is why the census is kept as an example rather than run once.

## 6. `A60`: what "cheap to adapt to part 3 later" means here, priced

The owner declined ISO 19005-3 and asked that adapting the validator and converter to it later
be cheap. `A46`'s shape is what makes it so, and this section says what a part-3 column would
actually touch so that the next person can price it without re-deriving:

- **`Part` gains a variant, `Three`, and `Target` a constructor for it** — with part 2's three
  levels if part 3 keeps them, which is how it is described and which nobody here has read.
  Every `match` on `Part` and `Target` in the crate is
  exhaustive, so the compiler lists the sites; `grep -n 'Part::Two' crates/pdf-archive/src` is
  the command, and today it prints a dozen rows across `requirement.rs`, `target.rs`, four table
  files, `coverage.rs`'s tests and `clarification.rs`'s. Each is a rule that differs by part —
  the header's version digits, the filter list, the annotation subtypes exempt from an
  appearance, the `cmap` subtables a symbolic font may carry — and each arm for part 3 is read
  from part 3's text, which is the row-by-row fill the next bullet prices.
- **`Clauses` gains a third field, `three: Option<&'static str>`.** ISO 19005-3 is widely
  described as ISO 19005-2 with one subclause relaxed — the embedded-file rule — and the same
  clause numbering; **that description is not held and is not to be implemented from**, but if it
  is right then nearly every row's `three` is its `two`, entered row by row from the text on the
  day it is bought, and `Clauses::both` grows a sibling. `editions::SHIFTS` needs nothing: part 3
  adheres to ISO 32000-1:2008 as part 2 does, so the same edition table serves.
- **`Applies::FromLevel` already reads a level of "part 2"; it becomes a level of "a part with
  levels"**, which is a doc comment and one match arm.
- **`clarification.rs` is the one place that is ahead of the code**: every `TechNote 0010` item
  already carries its `parts` line, and seventeen of the twenty-eight name ISO 19005-3 by name.
  `reaches` is a list of `Part`s; adding `Part::Three` to those seventeen is the whole change
  (`grep -c 'parts: ".*ISO 19005-3' crates/pdf-archive/src/clarification.rs` counts them), and
  the test that
  refuses a resolution printed under a part it does not name will hold it.
- **`coverage.rs` needs a `Subclause` list and readings for the part**, which is the genuinely
  new work — the sentence-by-sentence audit that ADRs 0981 and 1010 did for parts 2 and 4 — and
  it is what buying the text buys.
- **The converter** (`crates/pdf-transform/src/archive/`) reads a target and a requirement table;
  its census row per requirement already has a part column. Its embedded-file handling is where
  part 3 differs, and `doc/pdf-a-conversion-limits.md` section 3.1's two answers become three.

The price, then, is one enum variant, one field with a row-by-row fill, seventeen list entries,
a readings file, and the converter's attachment path — none of it structural. **What would make it
expensive is the thing this crate does not do**: a `match` that assumes two parts by testing "is
it four", a requirement identifier carrying a part's numbering, or a clarification looked up by
identifier alone. All three were argued away before part 3 was a question (ADRs 0933 and
`requirement.rs`'s comment on identifiers), which is why the owner's instruction costs this round
nothing to obey.

## What it would take to change this

For the grammar: the published ISO 8601-1:2016, if it differs from the draft on any form a
value in front of this validator takes — at which point the citation changes with the text. For
the NOTE rule: nothing short of the body being amended. For the ranges: a producer found writing
a calendar date that does not exist and meaning something by it, which would be a question for
`doc/questions/` rather than a change here. For section 6: the purchase.
