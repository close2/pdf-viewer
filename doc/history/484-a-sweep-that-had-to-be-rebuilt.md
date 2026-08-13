# 484 — A sweep that had to be rebuilt, and the entry behind a capability

**Finding.** `doc/todo/01`'s fifteenth sweep — the only one that can see the refusal shape where a
row retires itself by naming a capability that arrived — had produced two findings in twenty-five
rounds and **had never been committed as a program**. The round before last rebuilt it from its
thirty-line description before it could run it, which is `CLAUDE.md`'s own "write down the command,
not the answer" failing in the direction it was written for. It is `conformance::entries` now, and
its first run as a program found **§12.6.4.2's `/SD`**: Table 202 gives a go-to action a structure
destination beside its page destination and says "[i]f present, the structure destination should
take precedence over destination in the D entry", `grep '"SD"' crates/` found nothing at all, and
§12.3.2.3's row has read `implemented` — with two tests — since the algorithm that walks a structure
element back to a page landed. The capability was built, tested and cited; the entry that turns it
on was wired to nothing; and the ledger row *asserted the connection that was missing*. Third
instance of that shape in twenty-five rounds (ADRs 0295, 0315), and the sharpest, because the other
two were entries with no reader anywhere.

**This is also the closing round of the block of thirty**, 455 to 484, and `doc/history.md` carries
its summary.

**Date.** 2026-08-13.
**ADR.** [0319](../adr/0319-a-sweep-that-has-to-be-rebuilt-and-the-entry-behind-a-capability.md).
**Touched.** `tools/conformance/src/entries.rs` (new), `tools/conformance/src/bin/entries.rs`
(new), `tools/conformance/src/clause.rs` (`ClauseIndex::text_in`),
`tools/conformance/src/lib.rs` (one module line),
`crates/pdf-model/src/destination.rs` (`Destination::of_go_to`, `open_action`'s call site, one
test), `crates/pdf-model/src/action.rs` (the `GoTo` arm),
`crates/pdf-model/examples/structure_destination_census.rs` (new),
`crates/pdf-model/examples/signature_algorithm_census.rs` and `crates/pdf-model/src/dsa.rs` (eleven
clippy warnings, below), `doc/conformance/ledger.toml` (§12.3.2.3, §12.6.4.2, §12.6.4.3),
`doc/todo/01-ledger-partial-rows.md`, `doc/todo/02-every-round.md` §4,
`doc/todo/README.md` (two index lines), `doc/HANDOVER.md` (six claims),
`doc/crate-map.md` (a missing crate), `doc/environment.md` (two lines),
`doc/history.md` (the block summary), `doc/adr/0319-*`, this file.

## The sweep, and the one thing the description could not have said

Fourteen sweeps read what a row *says*. The fifteenth reads no reason at all: it takes the entries
the clause's own tables state, out of the standard, and asks whether any Rust source names each —
and then whether any file the row itself lists in `code = [...]` does. **The second question is the
sweep**: `/Open`, the entry the four-hundred-and-fifty-ninth found unread, *is* named under
`crates/`, by the popup reader under a different table.

The program adds the filter the second run's own lesson asked for and could not have. That run
ended by saying a hit is work only where the row's disposal of the entry is a claim about *the
entry* rather than about *the clause* — which no program can decide. What a program can decide is
the case with no claim at all: **the row's own note never writes the key**. First run as a program:
215 rows in the population, 41 stating an entry their own code does not name, 102 entries — 30
named nowhere in the tree, 72 only elsewhere — of which **43 are not named by the row's note
either**, and that is the list to read.

Its numbers are not the two earlier runs', and the ADR says why rather than tabulating them
together: a reconstruction is not the same instrument twice. Two choices the reconstruction had to
make and nothing recorded — a clause's tables come from its **own** text rather than from the span
that includes its subclauses, and an entry counts as named as `"Key"` or `/Key` and never as a bare
word — are in the module, with the reason.

## Counted before believed

`examples/structure_destination_census`, over every object the cross-reference table lists rather
than over every page's `/Annots`, because a go-to action hangs off a link, an outline item, an
`/OpenAction`, a widget's `/AA` and another action's `/Next`:

```text
1187 document(s) opened
  7838 /S /GoTo action dictionar(ies)
  10 state Table 202's /SD, 10 of which read as a destination
  0 name a page their /D does not and 0 a different view of the same page
  0 /S /GoToR action(s) state an /SD, which is Table 203's and excluded
```

Ten witnesses of the **entry** and none of the **rule**, which are different measurements and only
the second ranks work — ADR 0315's finding one clause along. So the fixture is trap 8's
prescription: three go-to actions differing in that single entry, one with `/D` alone, one whose
`/SD` names another page *and* another view, one whose `/SD` is no destination at all. All three
assertions were watched fail with their rule removed.

## What the reconciliation found

The round's other half was reading the documents against the tree after thirty rounds, eleven of
them merged from parallel worktrees by somebody outside the round.

- **`cargo clippy --workspace --all-targets` was not silent, and had not been for five rounds.**
  Eleven `clippy::pedantic` warnings, all of them in files the four-hundred-and-seventy-ninth added
  — nine in `examples/signature_algorithm_census.rs` and two in `src/dsa.rs`. The four rounds after
  it each recorded the lint run silent, and each was telling the truth about **its own worktree**,
  which branched before that file existed. `doc/todo/02` §2 calls this run "must be silent of
  lints" and CI makes warnings errors, so it was a broken gate rather than a nit. Fixed.
- **`doc/crate-map.md` says "one row per crate" and had no row for `render-quorra`** — the third
  rasteriser, and **the one the window actually presents with**. Nineteen crates, eighteen rows,
  since the second of August.
- **`doc/HANDOVER.md`'s "Where we are" said "[t]wo backends (CPU and GPU)"**, which stopped being
  the shape of this tree when a person's frames started going through quorra. Corrected there too,
  with `render-gpu` named as the comparison backend rather than the shipped one.
- **`doc/todo/README.md` said the sixth population of quotation is one "which nothing reads at
  all"**, ten rounds after the four-hundred-and-seventy-fourth built the instrument that reads it
  (ADR 0309) — and `doc/todo/48`'s own header has said so since. The index warns about exactly this
  in its opening paragraph.
- **Four more claims in `doc/HANDOVER.md`** were true when written: that no consumer had needed a
  new message (a *clause* has, ADR 0316); that the volume of a font's partial silence was not
  measured (ADRs 0311, 0318); that a click on the files panel is the only way to an embedded file
  (ADR 0295); and that where a closing round's block summary goes "is not decided", which
  `doc/history.md`'s own preamble decided.
- **`doc/environment.md` gained two lines**, both about the shared build directory: a build
  script's `env!("CARGO_MANIFEST_DIR")` outliving the checkout it was compiled in, which bit two
  rounds of the four-hundred-and-fifties; and `sccache`, which the owner has just activated and
  which reads **0.17% on Rust** over 6530 compilations, because this workspace's crates change on
  nearly every round.

## The sweeps

All of `doc/todo/01`'s, over `ledger.toml`, `crates/`, `tools/`, `fuzz/` and `doc/adr/`.

- **Expired blockers**: 13 over the ledger, 14 over the source roots, every one a row or a comment
  naming a clause it genuinely waits on.
- **Entries claimed unread**: 26 raw hits in the ledger's four phrasings, the known
  one-short-key-three-clauses population and the corrections quoting their own retired wording.
- **Capability reasons**: 11 over the ledger and 43 over the source roots, each a true statement
  about a boundary a crate keeps.
- **Retired claim**, over this round's own nouns — `/SD`, `structure destination`, `of_go_to`:
  clean once §12.3.2.3, §12.6.4.2 and §12.6.4.3 were written, and ADR 0054 makes no claim about the
  entry.
- **The prose quotation sweep** (`bin/quotations`): 2850 quotations in 420 documents, 1361 verbatim,
  **21 suspects and every one of them the known correct-writing class** — a document recording the
  wording a correction retired. Nine of the 21 are the four-hundred-and-seventy-fourth's own record
  of its thirteen corrections, which is what that round predicted about itself.
- **`spec-errata check`**: 151 struck passages `doc/md/` still carries as current, 73 quotations
  landing on one in the clause they cite, none in this round's files.
- **The fifteenth**: above.

## Gates

`tools/state.sh` after the last edit, and every ratchet held. The whole of `doc/todo/02` §2, and
§5's binaries rebuilt and installed.

**`doc/todo/00` step 7's ink sweep is not owed and was not run.** The only change under `crates/`
that can reach a raster is a go-to action's destination, which decides which page is *shown* and
never what is drawn on it; the corpus and oracle gates are identical, which is the check rather
than the claim.

## What the next round should know

- **Fourteen of the fifteen sweeps are still prose.** The argument in ADR 0319 applies to every one
  of them, and the cheapest moment to commit one is the round that next has to run it.
- **The sweep's list has 42 entries on it that were read and not taken**, each a refusal
  `CLAUDE.md` closes, a `reported` row's own subject, or `doc/md/` shifting a table's columns. The
  next run should start from the ones whose *note* does not name the entry.
- **A round green in a worktree has established nothing about `main`.** The clippy finding above is
  the proof, and the block summary in `doc/history.md` says what else eleven parallel rounds cost.
