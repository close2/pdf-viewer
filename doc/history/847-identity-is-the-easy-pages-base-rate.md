# Session 847 — the identical raster is the easy page's base rate, not the pair's signature

2026-09-01. ADR 0774. An oracle round on `doc/todo/12`'s consensus half, handed on by round 846.

**Finding**: byte-identical reference rasters decide 176 of the 1044 pages the oracle judges by a
consensus, and they are a property of the **page** rather than of the pair — 0.4% of text
consensuses against 68.9% of vector ones, *depleted* in the contradicted pool rather than enriched,
and 68 of the 176 a three-way identity including the reference the row that raised the question
excludes. So no rule follows, the consensus stands as it is, and ADR 0773's mechanism is a
description of its own 95 pages rather than a test for anything.

## Files

- `crates/pdf-model/tests/oracle.rs` — `ConsensusIdentity`, `the_consensus_that_decided_it`,
  `what_the_consensus_was_made_of`, and a paragraph on `CONTRADICTED_SHARED_JBIG2_DECODER`'s note.
- `doc/adr/0774-identity-is-the-easy-pages-base-rate.md` — new.
- `doc/traps/oracle-and-references.md` — trap 9's identical-rasters bullet gains the base rate.
- `doc/todo/12-one-bound-two-jobs.md`, `doc/todo/README.md` — item 3's remaining question closed.
- `doc/conformance/ledger.toml` — §10.7.1, the second track.

## What the round did

The census is arithmetic over numbers the gate already had: `Triangulation::between_references`
carries every pair's `Comparison`, and `max_error` is the one field that separates identity from
closeness. So it costs nothing, prints every run, and could have been written at any time in the
last four hundred rounds. What made it worth writing is that ADR 0773 measured identity on 117
pages of one row and handed on a question about a rule; ADR 0771 had, two rounds earlier, retired a
different rule for exactly the reason that its control had only ever been run on the population it
was invented for. The same sentence answers both, and the remedy in both cases was to make the gate
count the thing over the whole population rather than to reason about the sample.

Nothing moved: 980 agrees / 60 contradicted / 836 ambiguous over 1945 pages, before and after. The
four contradicted pages a set of identical rasters convicts turned out to be three pages of
`CONTRADICTED_SHARED_JBIG2_DECODER` — the group already named for `jbig2dec` twice — and one page
this tree reports on. There was no page to move, which is the honest result and is why the gate
names those four rather than counting them (ADR 0772's rule, applied without being asked).

The second number is the one nobody had asked for and may outlast the first: on **629 of the 1044**
pages a consensus decides, the bound never left the class floor at all. `widened_to`'s relative
bound — the whole reason this gate judges the way it does — decides nothing on three pages in five,
and a round arguing about `doc/todo/12`'s floor should know that before it starts.

## Second track

`--bin parts`, the twenty-second sweep, over `doc/conformance/ledger.toml`. §10.7.1's row said the
`shall` — *"the final step of rendering shall be scan conversion"* — is met because "both backends
are that step", and this workspace states three rasterisers. Its `code` named one file and its
`test` one test, so the row understated who meets the requirement as well as miscounting them.
§10.7.4's row had the identical sentence corrected in the seven-hundred-and-ninety-seventh session
and the *parent* clause that states the requirement was not read beside it, which is ADR 0697's
shape recurring one clause up. Corrected, with `render-cpu`, `render-gpu` and `render-quorra` named
and `cpu_and_quorra_agree_on_the_basic_scene` beside its GPU twin.

## Housekeeping

The build root was 119 GB; §5a's sweep over this checkout's own `debug`, `release` and `gates`
took it to 90 GB, leaving `tmp/pdfref-cache` and the directories `tools/worktree.sh list` says are
not this script's. §5's binaries were rebuilt and installed before any measurement.
