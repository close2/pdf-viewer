# 676 — The entry a row declared unread

Two more of the sweep's negatives re-derived over `CC-MAIN-2021-31` and **both false**, and the
second one turned out to be a different decay shape from the one this round was sent for: not a
count that grew, but a **clause nobody had finished reading**, whose one-sentence disposal had also
kept an erratum on the same entry out of `doc/errata-read.md` for as long as it stood.

Date: 2026-08-23.
ADR: [0502](../adr/0502-the-entry-a-row-declared-unread-and-an-erratum-nobody-recorded.md).

Touched: `crates/pdf-model/examples/absence_audit.rs`, `doc/conformance/ledger.toml` (§12.7.5.5,
§12.8.2.2, §12.8.2.2.1), `doc/errata-read.md`, `doc/todo/01-ledger-partial-rows.md`,
`doc/traps/instruments-and-reports.md`, `doc/HANDOVER.md`, the ADR and this file.

## What the briefing said, and what the instrument said

The briefing named §12.8.2.2.1 as next with "144 crawled `/DocMDP` documents", said about 24 of 45
rows were left, and told this round to verify all of it. Three of those numbers are worth reporting
back separately from the work.

**The row was right and the shape of it was right.** 143 of the 65 944 `CC-MAIN-2021-31` documents
bind a `/DocMDP` through §12.8.6's `/Perms`, against the curated one — the 144th is in
`corpus-cache/openpreserve`, which is 267 documents beside the crawl and not part of it.

**The 24 was not.** `doc/todo/01`'s own script, run against the six-hundred-and-seventy-first
session's commit, prints **7 done and 38 owed before that round and 17 and 28 after** — so the
sentence saying "10 … and 11 more … leaving 24" is wrong in all three numbers, and the four-group
breakdown under it names 24 rows where the instrument names 28. The four it misses are §8.6.5.6,
§8.11.4, §9.7.4.2 and §9.10.2. Three of those are the grep's own sentence boundary — the regex ends
a sentence at any full stop, so a clause number or a file name inside one splits it — and the
fourth, §9.7.4.2, is real and joins §8.4.3.5 and §12.5.4 as a row whose census owes a `--crawl`
argument.

`CLAUDE.md`'s rule was obeyed here and did not save it: the command **is** printed in the file,
directly above the sentence, and the round carried its number forward instead of running it. A
command in a document does not make the number beside it measured.

## §12.8.2.2.1 — the majority level is the one no file exercised

| | curated (1251) | crawl (65 944) |
|---|---|---|
| `/Perms /DocMDP` binding a level | **1** | **143** |
| `/P` 1 — `Modification::None` | 0 | **122** |
| `/P` 2 | 1 | 16 |
| `/P` 3 | 0 | 5 |
| a `/Perms /DocMDP` with no readable level | 0 | **0** |

The old sentence — one corpus document, `/P 2` — is true and names the *middle* level. In the world
the majority is 1, the level that withholds both operations this program has, and until now it was a
hand-built fixture standing in for nothing.

**What a reader owes here was asked before anything was assumed**, because the briefing was right to
flag it: §12.8.2.2's `shall`s about creating a certification signature fall on a producer, and the
one that reaches a reader is §12.8.2.2.1's parenthesis about the permissions dictionary. It has been
honoured since the hundred-and-ninety-first session in exactly the shape `CLAUDE.md` §3 asks for —
`restriction::asserted` answers with a reason naming its clause and its level rather than with a
refusal, `viewer_core` asks once per operation, and a host can turn it off. So nothing was owed, and
the 143 documents change only that the code is now known to be exercised rather than assumed to be.

**The negative that survived is worth as much as the two that fell.** Not one document in either
population states a `/Perms /DocMDP` whose level this tree cannot read — and the standard describes
that `/Reference` twice and differently, Table 255 as an array and Table 263 in the singular, where
`modification` accepts only the array. A planted file of exactly that shape was run through the new
block, scored the answer that names it, and was deleted before the zero was believed.

## §12.7.5.5 — one sentence of seven, and the erratum that followed it out

The row said Table 236's `/P` "is deliberately not read here", quoting the entry's sentence about
absence having no effect on signature validation rules. The entry has seven sentences and three of
the others are addressed to a processor that *changes* the file: it opens "The access permissions
granted for this document", and later states "permissions can be denied but not added", a default of
3 where there is no author signature, and that "[t]he new permission applies to any incremental
changes to the document following the signature of which this key is part".

**28 of the 65 944 crawled documents state it, 0 of 1251 curated, and every one of the 28 is `/P` 1
on an `Action` of `All`.** What that costs today is exactly one operation: `All` already withholds
every field from form filling, and `asserted` consults a field lock only for that operation, so
annotating one of those 28 is accepted while the level they state says no change at all is
permitted.

**And the disposal had a second cost nothing could have printed.** Errata issue #131 amends this very
entry — carving out a DSS-only or timestamp-only incremental update, the same exception §12.8.2.2.1
states for `/DocMDP` — and `doc/errata-read.md` has never carried it, because a row saying the entry
is not read is a reason not to open its page. An erratum that carves an exception out of a permission
is evidence the entry states a permission.

**It is measured and argued and deliberately not implemented**, with the cost written down. The entry
is two-voiced: it states permissions and gives no route that makes them binding, where §12.8.2.2.1
gives `/DocMDP` exactly such a route and says so in a parenthesis written for the purpose. Reading it
as a permission adds a default refusal on 28 real documents for a sentence the standard may not be
addressing to us, and `CLAUDE.md` §3 makes that the error worth avoiding in a hurry. The row is
`partial` now and names what is not executed; when it is settled it goes through
`restriction::asserted` like the other five reasons, so the four policy levels reach it without
revisiting anything.

## Two instrument findings

**A truncated witness list is a count with the finding cut off it.** `absence_audit` printed twelve
names and "… and N more", and §12.2's ninety-six witnesses were retired by a *distribution* rather
than a count. `report` tallies the distinct answers now wherever it truncates — four lines, and it
paid for itself on this round's own blocks (122/16/5, and 28-of-28 `/P 1 on /All`) and
retrospectively on §12.7.5.5's, where 90 locks against 89 `FieldMDP` transforms now says which single
document holds the lock without the copy.

**A sweep binary carries its tree with it** — `root()` is `env!("CARGO_MANIFEST_DIR")`, compiled in
and unaffected by the working directory. This round's entire before-sweep was run from the *main*
tree's build directory, printed thirteen plausible summary lines, and had measured a tree it had not
edited; the tell is that nothing moves after an edit, which reads as "my change touched no sweep".
That is **trap 15** now, with the recipe for a real before: `git archive HEAD` into a directory,
symlink `doc/md`, the PDFs, the submodules and `corpus-cache` back in, and build the sweeps inside
it. Without `doc/md` three sweeps refuse; without the rest `pointers` invents deltas.

## The instrument, before and after

Thirteen sweeps, both runs from binaries built from the tree each was measuring. **Seven moved and
every delta is this round's own prose**: `counts` 6909 → 6936 sentences, `overstated` corroborations
56 → 58 with contradictions unmoved at 8 (7 marked, and §12.7.5.5's `/P` denial still among them —
correctly, since the row still states it, now as a struck sentence with its argument beside it),
`overtaken` 497 → 498 decision records, `owed` 224 → 225 rows and 3553 → 3597 terms with 181 → 184
unnamed over 114 → 115 rows (all of it §12.7.5.5 joining the `partial` population), `pointers` 7215 →
7228 with **absent unmoved at 123**, `quotations` 1773 → 1779 in the ledger with all six new ones
verbatim and diverging unmoved at 2, and `tables` 5863 → 5881 sentences and 2217 → 2227 key citations
with **absent unmoved at 100**. `blockers`, `callers`, `capabilities`, `entries`, `inapplicable` and
`unread` are at their standing populations.

Run a third time on the finished tree (ADR 0485), because an ADR, a history file and a trap edit are
`SOURCE_ROOTS` too: only `pointers` moves again, 7228 → 7235, and **absent stays 123** — the new
pointers are this file's and the trap's, and every one of them resolves.

## Gates

The change reaches `crates/pdf-model` (one example), so the map asks for everything, and `round.sh`
called this a fifth round besides. The whole of `doc/todo/02` §2 was run.

- `cargo fmt --all --check` — exit 0.
- `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` — exit 0.
- `cargo nextest run --workspace` — **2437 passed, 17 skipped**, 55 s.
- `cargo test --workspace --doc` — exit 0.
- `RUSTFLAGS="-D warnings" cargo check --manifest-path fuzz/Cargo.toml --bins` — exit 0.
- `cargo build --profile gates -p pdf-sandbox --bins` and `-p hayro-compare --bin pdfref-hayro` —
  both exit 0 (trap 10).
- **corpus** — exit 0: 974 documents in 3.5 s, 0 unopenable, 8 locked, 2 encrypted beyond us, 6
  pageless, 68 incomplete, 0 slow.
- **oracle** — exit 0: agrees 908 (863 on pages called complete), contradicted 65, ambiguous 786, our
  geometry 2, reference geometry 2, not comparable 13, no render 18 — every one identical to the
  previous clause round's, which is what a round that moved no drawing code should see.
- **text extraction** — exit 0: 99.8% (14257/14281 words) against PDFBox in both orders with 4 below
  90%, and the position gate 10969/11163 in bounds (98.26%), 486 of 508 documents fully in bounds.
- **selection census** — exit 0. **accessibility census** — exit 0: 102 853 elements reached, 57 116
  a caret can move through.
- **dates**, **xmp**, **jpeg2000** — exit 0. **fixed documents** — exit 0: 40 checked, 0 absent.
- **quorra corpus** — exit 0: 957 pages compared, 933 agree, 22 differ, 2 refused, 17 not comparable;
  median page 2.35× the CPU backend.
- `cargo test -p conformance` — exit 0. **875 rows**: 435 implemented, 225 partial, 18 reported, 76
  inapplicable, 8 writer-side, 113 out-of-scope, 0 unreviewed, no `silent` row. **One status moved**,
  §12.7.5.5 `implemented` → `partial`, and it is the round's finding rather than a bookkeeping change.

The reference cache was **copied** into this worktree's own target directory rather than shared, so
the oracle's 908 agreements are not a read of a directory three neighbours are writing.

**§5's binaries were deliberately not installed**: this is a parallel round told not to push or
merge, `target/` is the *main* tree's, and putting an unmerged branch's binaries where a person runs
them is what §5 exists to prevent. The merge round owns it.

## Overlap with the parallel rounds

Three other rounds ran beside this one. Nothing written here is outside the three ledger rows named
above, `doc/errata-read.md`'s new line, `doc/todo/01`'s corrected paragraph, and the new trap; no
other row was reflowed.
