# ADR 0496 — The other census had the same three roots

Status: accepted, 2026-08-22. Session the six-hundred-and-seventy-first, a clause round under
`doc/todo/01`'s binding rule, continuing the seventh step of its technique and finishing the
sweep that step opened. Amends §7.11.4.2's, §8.11.1's, §8.11.4.5's, §10.7.2's, §12.2's, §12.5.6.21's,
§12.6.3's, §12.6.4.7's, §12.7.5.5's, §12.9's, §12.9.2's, §12.11.1's and §14.11.6.2's ledger rows, and
gives `examples/absence_audit` a population flag, eight new claims and a nested walk. Extends
ADRs 0490 and 0493; changes nothing ADR 0101, 0403, 0405 or 0406 decided.

## 1. What this decides

ADR 0490 established that **a ledger negative measured before `CC-MAIN-2021-31` arrived is a negative
nobody has measured**. ADR 0493 found the reason four of its six rows had never been checked — the
census built *for* absence claims had its own population hard-coded — and put `--crawl` on
`examples/witness_census`.

**That repair was half a repair, and this round is the other half.** The sixteenth sweep has two
instruments and ADR 0403 is emphatic that both must be run: `witness_census` asks whether a *name*
appears, and `absence_audit` asks whether the *structure* is there, through the reader that would act
on it. Only the first got the flag. So the round after ADR 0493 could re-derive any claim about a
name and no claim about a construction — and a construction is what most of the remaining negatives
are about.

`absence_audit` has `--crawl` now, eight more claims, and one correction to how it walks.

## 2. Ten negatives, five false

Each was run over the curated population and over the crawl, separately, because ADR 0490's
control-and-growth pair needs the two answered apart. `65 703` is the number of the 65 944 crawled
files this tree opens; `1251` is the curated population.

| row | curated | crawl | verdict |
|---|---|---|---|
| **§12.2** Table 147's four boundary entries | 0 | **96** name them, **95** inside a `/ViewerPreferences` | **false** |
| **§10.7.2** `/FL` in a graphics state parameter dictionary | 0 | **88** | **false** |
| **§12.6.3** `/PV` and `/PI` on an annotation's `/AA` | 0 | **5** | **false** |
| **§12.7.5.5** `/Lock` on a *signed* signature field | 0 | **90** | **false** |
| **§12.9.2** a viewport whose `/Measure` is Table 267's rectilinear one | 0 | **127** of 277 stating a `/VP` | **false** |
| **§7.11.4.2** `/RF` on a file specification | 0 | **0** | holds |
| **§12.11.1** a `/Requirements` array in the catalog | 0 | **0** | holds |
| **§12.5.6.21**, **§14.11.6.2** a `/Subtype /TrapNet` annotation | 0 | **0** | holds |
| **§12.6.4.7** an action whose `/S` is `Thread` | 0 | **0** | holds |
| **§8.11.1**, **§8.11.4.5** a usage application naming `Zoom`, `User` or `Language` | 0 | **0** of the 475 stating one | holds |

The last of those is the rule ADR 0475's round wrote down and this one obeyed: **a note whose count
already has a gate owes the gate's name, not a new census.** §8.11.4.5's row already names
`examples/oc_usage_census`, so the re-derivation is that program over a wider argument list —
`find corpus-cache/safedocs/cc-main-2021-31 -name '*.pdf' -print0 | xargs -0 -P 8 -n 400`, four
minutes — and the answer is `View` 758, `Print` 648, `Export` 530, and the three processor-facing
categories not once. §12.6.3's count has a gate too
(`actions.rs::the_corpus_states_these_page_scoped_triggers`), which asserts six numbers over the
curated population and is why the new block asks that gate's question rather than a new one.

## 3. What the round adds, and the first is a defect it wrote itself

**Plant the witness against the census, not against the reader.** `doc/habits.md` already says a zero
owes a planted witness, and ADR 0493 planted one through `interpret`. This round built a file stating
all seven new constructs, dropped it into `doc/corpora-own` for one run and deleted it — and the
first draft of the example scored it **zero for §12.6.4.7's thread action**, because the file writes
its action inline inside the annotation's `/AA` and the draft asked only the objects the
cross-reference table names. That is the six-hundred-and-forty-eighth session's finding, reproduced
in code written by a round that had read it that morning. §10.7.2's resource route was invisible for
the same reason, being one level under `/Resources`. `visit` recurses into each object's own nested
structure now, bounded like `witness_census`'s and following no reference. **Two of seven blocks
would have written a false zero into the ledger**, and nothing but the planted file could have said
so.

**A negative can be false with its sharper half intact, and that is a third row rather than either.**
§12.2's sentence was two claims wearing one: "none states any of the four boundary entries", and "the
half of the clause that can change a pixel has no corpus witness". The first is false — 96 crawled
documents state all four. The second **survives, and now on a measurement**: every one of the 96
states `/ViewArea` and `/ViewClip` as `/CropBox`, which is Table 147's own default for both, so
`Page::display_box` and `Page::clip_box` answer what they would have answered from silence; and the
one document naming a box the table does not default to — `cc-main-2021-31/3498/3498998.pdf` — names
it as `/PrintArea /MediaBox` and `/PrintClip /MediaBox`, the pair nothing here prints. A round that
wrote only *false* would have deleted a true sentence.

**Where the two instruments disagree, the direction is the finding.** ADR 0403's rule was that a name
being present is not the structure being present, and ADR 0493 paid it on `/CL`. This round paid it
twice more, harder:

- **§7.11.4.2's `/RF`.** 55 710 crawled documents' raw bytes contain the token, 32 192 documents'
  decoded streams do, **one** states it as a name in an object, and **not one** carries it on a file
  specification. The gap between the first number and the last is four orders of magnitude.
- **§12.11.1's `/Requirements`.** 411 documents' decoded streams contain the word and no catalog
  states the array. That is a census of English prose.

A byte search would have called both clauses well witnessed and both rows wrong.

## 4. What each false one costs, said out loud

`doc/todo/01`'s third step: a refusal with witnesses is a different row from a refusal with none.

- **§10.7.2** is `implemented` and stays there, because the clause's normative content is a
  *permission* — "PDF processors may choose to ignore any flatness tolerance specified within a PDF
  file" — and a permission is not weakened by producers exercising the entry. What changed is that
  the row's second defence, the absence of files, is gone: 88 documents state `/FL` and honouring
  their number could still only make a curve worse, which is now an argument rather than a
  convenience.
- **§12.6.3** keeps its derivation. `/PV` still coincides with `/PO` here because this layout shows
  one page, and what five witnesses would decide is a multi-page layout — §7.7.2's `/PageLayout`, not
  this clause.
- **§12.7.5.5** gains real producers for Table 236's three actions, so trap 8's hand-built fixture is
  a second witness rather than the only one. The shapes are workflows: an `Include` naming
  `USGPOSignature` or `Signature1`, then `All`.
- **§12.9.2**'s worked example is likewise a second witness now, against 127 documents.
- **§12.9** keeps its `partial` and its reason — nothing here measures, because there is no tool a
  person drags with — and the reason now has a size: 277 documents, not one.

No status moved. That is right, and it is the same answer ADR 0493 gave: five rows lost a false
sentence about the world and five gained the evidence they were resting on. What a clause requires
did not change.

## 5. The instrument, before and after

ADR 0485's habit; thirteen sweeps run before the edit and after it.

**Two hit lists moved and the rest did not.** `--bin overstated` 8 contradicted with 7 marked (its
corroborations 55 → 56, which is §12.9's row now asserting a term §12.9.2's corroborates — the
direction a parent and child should move in); `counts` 4 places counting one family twice;
`quotations` 2 diverging in the ledger and 34 in the documents; `entries` 177 over 49 rows;
`pointers` 118 absent and 13 undefined; `owed` 181 unnamed terms over 114 rows; and `blockers`,
`capabilities`, `inapplicable`, `callers` and `overtaken` at their standing populations.

**`--bin tables`' absent list went 99 → 100, and the hit is this round's own prose.** §12.2's finding
turns on a default, so the sentence carrying it names a page-boundary *value* next to the number of
the table whose **entry** takes that value — and the sweep reads the pair as a key citation, prints
the right answer itself (`stated by: Table 31, Table 396`) and marks the hit `[correction]`, which
demotes it. That is the second of the three noise shapes its own closing paragraph names, and it is
charged to any round that writes down what a default resolves to.

Two things about it are worth more than the count. **It was found only because the sweeps were run a
third time, on the committed tree** — not on the ledger after the ledger edit. An ADR, a history file
and a todo edit are `SOURCE_ROOTS` too, so a round that stops measuring when the ledger is finished
has measured half of what it wrote. **And the level briefly read 102**, because the first draft of
this section and of the history file each repeated the pairing they were describing: documenting this
shape instantiates it, once per place, which is what a sweep over two adjacent words does. The
finding's own sentence is not rewritten to dodge that — writing around an instrument is what ADR
0490 §6 refused — but its description gives the example once rather than three times, which is the
ordinary reason to give an example once, and the level settles at 100.

**The one that moved is `--bin unread`, and it is ADR 0493's noise shape one sweep over.** Confirmed
46 → 44, quoted 136 → 138, both on the single key **`/FL`** under §8.4.5's row and §10.7.2's — because
the census this round wrote *to measure how many documents state `/FL`* names the string `"FL"`, and
that sweep asks whether any source quotes a key a row calls unread. So **obeying `CLAUDE.md`'s "write
down the command" rule makes a row that says an entry is unread look wrong to the sweep that checks
exactly that**. Neither repair is right — dropping the census is the instrument deciding what the
ledger may say (ADR 0490 §6), and teaching the sweep about examples is a special case pleading — and
neither is needed, because the sweep's own discriminator already handles it: its read-first list is
the keys named by *the row's own `code` array*, and that number did not move (68 both runs). Know the
shape; read the witness path.

**And `owed` gained no phantom this round, which is worth the sentence.** ADR 0493 found that a
citation like `examples/luminosity_mask_census` yields the phantom key `luminosity`. This round cited
`examples/absence_audit` and `examples/witness_census`, whose leading segments are `absence` and
`witness` — ordinary English words that occur in those very files — so the phantom is *named by a
source* and never reaches the unnamed list. The shape costs a round one phantom only when the
invented noun is invented.

## 6. `spec-errata emit` before writing

Over all fourteen documents, for every family touched. **§12.2 carries Issue #14**, which strikes the
cross-reference `7.7.3, "Page tree"` out of all four boundary rows and writes `"Table 31 - entries in
a page object"` in its place, and re-sets `CropBox` in each — a pointer and a typeface, moving no
requirement, and the row's new sentence quotes the entry's own words rather than the struck
reference. **§12.11.1 carries Issue #187**, already in that row, and Issue #656's EDITOR NOTE that
Tables 269–274 and 276 head their third column *Description* where every other dictionary table says
*Value*; it changes nothing this tree reads. **§7.11.4.2 carries Issue #382 and Issue #598**, the
latter adding a NOTE about `/CreationDate` and `/ModDate` reflecting file system dates — Table 45's
subject and not `/RF`'s.

**And §10.7.2 carries a live erratum its row had never mentioned, which running `emit` before writing
is the whole reason anybody looked.** As printed, a flatness value "shall be a positive number";
Issue #371, state Review/Completed, strikes that sentence outright and writes in its place that it
shall be a number in the range 0 to 100 inclusive, where 0 selects the output device's own default
and the value is a maximum error tolerance in device pixels. **It moves no requirement here**, and
the reason is the clause's own shape: a processor exercising §10.7.2's permission never reads the
number, so a rule about which numbers are valid binds a writer and a processor that honours one.

**The finding is not the erratum, it is where the erratum already was.** `doc/errata-read.md` carries
Issue #371 under §10.7.2, read correctly and disposed of correctly — "moved here from Table 58 … the
permission this row rests on is untouched" — and the *row* never heard. So the seventeenth sweep's
premise has a mirror image nobody had named: that sweep asks whether a place that records an erratum
has applied it, and this is a place that recorded it, applied it in reasoning, and left the
conclusion in the document it was written in. Two homes, one of them gated by
`cargo test -p conformance`, and the ungated one was right first. It is ADR 0101's "a retired claim
is a string" pointed the other way: a *read* claim is a string too, and nothing carried it across.
§12.6.3, §12.6.4.7, §12.7.5.5, §12.9, §12.9.2, §12.5.6.21, §14.11.6.2 and §8.11.4.5 get no heading at
all.

## 7. Consequences

- `absence_audit` grows `--crawl` and eight claims, so the sixteenth sweep's structural half now
  reaches the population its name half does.
- `doc/todo/01` states where the negatives sweep stands **as a command**, and splits what is left
  into the three groups an instrument divides them into: a content-stream census nobody has written
  (five rows), a structural block of the kind this round added eight of (nine rows), and the rows
  that are not claims about a corpus at all (five, two of which owe an existing census a `--crawl`
  argument rather than a reading).
- The sweep is **not** finished, and the honest form of that is the list above rather than a number.
  What *is* finished is the part a name census can answer.
