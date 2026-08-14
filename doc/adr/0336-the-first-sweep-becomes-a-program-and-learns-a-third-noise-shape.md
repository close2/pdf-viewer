# 0336 — The first sweep becomes a program, and learns a third noise shape

Status: accepted (session 501)

## Context

`doc/todo/01` binds a sweep round to commit one prose sweep as a program before running any of
them. Three of the fifteen were commands — `entries` (ADR 0319), `quotations` (ADR 0309) and
`unread` (ADR 0324) — and the expired-blocker grep, the oldest sweep of all (session 118, ADRs
0107 and 0108), was still a description re-derived on every run. It was chosen over the other
eleven because it is among the two most-run and because a program can carry the judgement each
run used to redo by hand: whether the thing a blocker names has arrived.

Six rounds had landed in one wave since the last full sweep (486–495), which is the condition the
sweeps exist for, and the retired-claim sweep's noun list was exactly their vocabulary.

## Decision 1: sweep 1 is `conformance --bin blockers`

The first sweep — "while §X does not exist", "needs §Y", "until §Z" — is now
`tools/conformance/src/blockers.rs` and `cargo run --release -p conformance --bin blockers`,
over the ledger's notes and, block by comment block, the comments under `SOURCE_ROOTS`. Four
choices shape it:

- **The claim is matched by shape, not wording** — seven phrases, `while §` to `blocked by` —
  which is `unread`'s own lesson applied to this sweep's vocabulary.
- **Where the blocker is a clause, the ledger itself judges it.** The sentence's `§` citations
  are parsed with the crate's own `ClauseNumber` and looked up: a blocker whose every named
  clause has a settled row has *expired on the ledger's own account* and prints first; one
  naming an owing clause holds; one naming no clause at all — a capability, a dependency, a
  decision — is printed as the reading's share. This is the judgement no grep could make, and it
  is why this sweep was worth a program more than the capability grep beside it.
- **Comments are read as blocks, not lines**, because a blocker sentence is prose and prose
  wraps at rustfmt's width — "needs" on one line and "§11.4.6" on the next would hide from a
  line grep. Only text after a `//` marker counts: a string literal quoting a phrase is data.
- **The checker's own directory is excluded** (`NOT_SCANNED`), because this module's
  documentation quotes the claim phrases as examples, and a sweep that reports itself is the
  fastest way to have one switched off. `unread` deliberately includes it — a key is not
  something the checker writes as an example — and the difference is stated in both places.

The known false-positive shapes are printed rather than filtered, marked where a program can
mark them: a sentence that also reads like history — "used to", "said", "retired" — carries
`[history]`, which is the correction-quoting-its-retired-wording shape; a past tense is
invisible to any grep and stays the reader's, as every prior run of this sweep recorded.

It is a reading list and not a gate, for ADR 0249's ratio reason.

## Decision 2: the first run's finding is about the instrument, and it is kept in the instrument

First run: 20 blocker sentences over the ledger (6 expired by the ledger's account, 9 holding,
5 naming no clause), 26 over the source roots (9, 10, 7), **0 defects** — and one hit that
taught the sweep a third noise shape. §12.10.2's "converting them needs §12.10.3's external
references" printed as expired, §12.10.3's row being `implemented` — and the wait was never on
the clause: it is on the EPSG registry ("administered by the International Association of Oil
and Gas Producers") and ISO 19162's WKT grammar that §12.10.3's *entries point at*. **A blocker
can name a clause as the route to something outside the standard, and the named row settling
does not settle the wait.** The module doc carries the shape beside the other two, and the row
now names the registry rather than the clause, so the next run does not re-litigate it.

## Decision 3: the band after 165, and what the other sweeps found

The blame list's next band — thirteen rows, commits 165 to 185, minus the four §12.8.\* rows a
parallel round owns (§12.8.2.1, §12.8.4.1, §12.8.4.5, §12.8.5.3) — was read row by row, kept
rows recording their evidence. Two corrections:

- **§12.3.5.2** said "`partial` for the panel" while `viewer_ui::chrome`'s files tab has drawn
  the clause's folder tree since session 352 (ADR 0202) — `doc/todo/01`'s second and fifth
  shapes. What is actually owed is named now: a folder's own `/Thumb`, `/CreationDate` and
  `/ModDate`, with Table 159's `/Free` named rather than owed because its `shall` fires "when a
  new folder is added" and no verb here adds one.
- **§14.7.2** (from the capability sweep, not the band) still said "nothing in this program yet
  hands a structure tree to anybody" four sentences above its own appended correction — failure
  shape 6, fixed by rewriting the paragraph rather than appending a third statement.

The retired-claim sweep over the wave's nouns paid twice, both one file away from a round's own
correction: `examples/free_text_census.rs` still called `/CL` "the callout line `doc/todo/33`
holds open" one round after 494 closed it, and `doc/todo/README.md`'s item-31 line still listed
the empty accessibility answer 490 had closed (ADR 0325). **An index row decays at its item's
pace, not its own.**

Every other sweep was clean, including the first fully clean table-numbers run over ledger and
source together (1159 citations, 90 suspects, 0 defects) and the errata census unchanged at 151
struck passages — with one instrument note recorded: run over a single PDF the census prints
150, because the population is the annotations', so the invocation's document list is part of
the count.

## Consequences

- Four of the fifteen sweeps are committed programs; eleven remain prose, one per sweep round.
- `doc/todo/02` §4 states the new invocation; `doc/todo/01` carries the run's record.
- The blame list's unread region starts at commit 185, plus §12.8.\*'s four rows in this band,
  which stay with the signature round.
