# ADR 0715 — The two sentences a table kept to itself

Status: accepted, 2026-08-28. Session 777, a general-improvement round choosing its own subject.

Table 384's `/Summary` and `/Short` — the two entries §14.8.5.7's own ledger row named as its
remainder — are read and reach a person, which closes the table and moves the row to
`implemented`.

## 1. Why this subject

The spec-driven track named it twice over, in its own instruments:

- `cargo run -p conformance --bin entries` prints §14.8.5.7 among the `partial` rows whose stated
  table has entries the row's own code does not name, and the row's note already said which:
  "[w]hat is left is `/Summary`, a table-level sentence nothing yet says, and `/Short`, whose
  value is what a screen reader should repeat in place of a long header".
- `doc/todo/31` had recorded both as an item to take "when a witness appears, **or as spec-driven
  work with that count written beside it**" — and a corpus that states neither entry is exactly
  the population a demand-driven round can never reach, which is `CLAUDE.md`'s "work is chosen
  from both" put to work.

The three siblings of this batch hold the errata ranking, the confined-boundary host and a
launch-flake test; this touches none of their files.

## 2. What the two entries are, in the standard's own words

Both sit in Table 384 beside the four already read (`/RowSpan`, `/ColSpan`, `/Headers`,
`/Scope`), and both are text strings, optional, not inheritable:

- `/Summary` — "A summary of the table's purpose and structure", with the entry's NOTE naming the
  consumer: "For use in non-visual rendering such as speech or braille." Its condition: "This
  entry shall only be used within Table structure elements".
- `/Short` — "Contains a short form of the content of a TH structure element's content", with an
  EXAMPLE that states the moment it exists for: when "for each table cell the applicable header
  cells are read to the user", it "can become cumbersome for a user to repeatedly have to listen
  to the full contents of a TH structure element". Its condition: "This entry shall only have an
  effect for structure elements of type of TH".

## 3. The decisions

**Where each reaches a person is a choice, and it is documented as one.** The standard states
what each entry contains and who it is for; it does not state a channel, because it knows no
platform. On this platform (ADR 0214's AccessKit/AT-SPI bridge):

1. **`/Summary` becomes the table node's description** — the same channel and the same argument
   as §14.8.4.8.3's headers (`tree::headers`): the description is the one channel of this
   adapter that reaches a person for a sentence *about* an element. It does not ride the node's
   name, because that field is deliberately the element's own marked text, and a summary spoken
   as if the table's content said it would be indistinguishable from a caption the author drew.
   The prefix `summary:` says which kind of sentence is coming, as the headers' prefix does.

2. **`/Short` substitutes in the repetition and only there.** `spoken_headers` — the text a
   header cell is *said as* in front of each cell it describes — takes the stated short form
   instead of walking the header's subtree; the header cell's own node keeps its full content.
   That is the EXAMPLE's own scoping: what is cumbersome is the repetition, not the cell, and a
   person who moves *to* the header is asking for what it says. The entry's sentence — "[a]n
   option to have the short form of the content of the TH structure element read out aloud is
   sometimes preferred" — frames a preference; with no preference surface in this program (the
   restriction-levels shape is about document restrictions, not reading options), the stated
   short form is used where it exists, which is the reading under which a producer who wrote the
   entry gets what they wrote it for.

3. **The type conditions are applied where the mapped role is in hand** — in `viewer-core`'s
   walk, not inside `pdf-model`'s readers — following the split `header_scope` already made:
   which standard type an element maps to is §14.7.3's question, answered once per element in
   the walk, and Table 384's "only … within Table" / "only … for … TH" sentences are about that
   mapped type. `Tree::table_summary` and `Tree::header_short` read the attribute by §14.8.5.3's
   priority (both routes: `/A` and the class map — the pdf-model test pins the class route);
   the walk asks each only for the type its sentence names. A `/Short` planted on a table or a
   `TD` is a statement §14.8.5.7 does not define and does not cross — the headless fixture
   plants both and asserts them absent.

4. **Two new fields on `AccessibilityNode`, not a reuse of `name`.** `summary` and `short` cross
   the confined boundary as two `option_str`s beside `header_scope`. Copying the short form into
   every cell's answer was rejected for the reason `headers` crosses as indices: a copied string
   is a second statement that can disagree with the first.

## 4. What the population is — measured, and half the expected silence was wrong

`examples/cell_header_census` now counts `/Summary` beside `/Short` (it counted `/Short`
already), and **names each witness document** rather than only counting, because this round is
the round that learned why that matters. The expectation, written in `doc/todo/31` and in the
ledger row, was a population of nothing. The measurement:

- `/Short`: 0, everywhere — doc/pdf.js (964 opened, 90 tagged) and the whole corpus-cache
  crawl (66 211 files).
- `/Summary`: 0 in doc/pdf.js — and **194 `Table` elements across 26 documents of the SafeDocs
  crawl**. The item every list carried as unwitnessed spec-driven work had real demand that no
  instrument had looked for.

One witness was read end to end rather than trusted to the unit tests:
`cc-main-2021-31/0423/0423767.pdf`, whose page-10 table crosses carrying its producer's own
sentence, "Table displays MAT codes, Descriptors, and Payment Rates" — checked through
`Query::AccessibilityTree` on the real file, via a temporary patch to
`viewer-core --example accessibility_cost`, reverted after.

The gate tests stay fixtures written from the clause (trap 4's stated exception, as ADR 0711's
collections), because the witnesses are machine-local crawl files no gate walks; the census is
the instrument that names them.

## 5. Calibration (trap 13)

Each planted wrong shape fails a test, and one calibration improved the test itself:

- `table_summary` switched to `inherited_attribute` **passed** at first — the fixture's nested
  cell stated no `/P`, so inheritance had nothing to climb. The fixture now states it, and the
  planted shape fails on "not inheritable".
- The walk with the type conditions dropped fails the headless test on the planted `TD`.
- A `/Short` that prepends but still walks the header's subtree fails the tree test on the
  child's word appearing.
- The wire codec with the two fields decoded in swapped order fails the protocol round trip.

## 6. Consequences

- §14.8.5.7's row is `implemented`: all six entries of Table 384 are read, each reaching a
  person or a refusal with its clause condition applied.
- `doc/todo/31` loses its `/Short` and `/Summary` bullets.
- The `entries` sweep no longer prints §14.8.5.7.
- 26 crawled documents whose producers wrote table summaries for a screen reader are now read
  as their producers specified — a population that was assumed empty until this round counted
  it, which is §4's lesson: **a claim that an entry is unwitnessed is a measurement somebody
  has to have taken, and "has not been counted" is not the same claim as "counted, zero".**
  `doc/todo/31` had recorded exactly that distinction and it was right to.
