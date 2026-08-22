# 645 — The claim a child denies

The failure shape 641 found by reading and said no sweep could print, built as one. `--bin
overstated` is the eighteenth sweep in `doc/todo/01` and the thirteenth of them to be a committed
program, and it is the first that opens no source file at all.

Date: 2026-08-22.
ADR: [0475](../adr/0475-the-eighteenth-sweep-and-the-claim-a-child-denies.md).

Touched: `tools/conformance/src/overstated.rs` (new), `tools/conformance/src/bin/overstated.rs`
(new), `tools/conformance/src/lib.rs`, `doc/conformance/ledger.toml` (§9.7.6, §9.9.1),
`doc/todo/01-ledger-partial-rows.md`, `doc/todo/02-every-round.md`, the ADR and this file.

## Which discriminator, and why

Two were on the table. **A row against its own children** — a parent asserting what no child
asserts, or the opposite of one — and **a row against the tree**, which is `--bin capabilities`
with the sign flipped. The first was taken, for three reasons that are ADR 0475 §5:

- Its answer side is exact. Both halves are this project's own sentences about its own code, so a
  contradiction is a contradiction whatever the standard says. The other discriminator's dominant
  noise is precisely the half a program cannot settle — a row describing what a *clause* requires
  rather than what this tree does — and the two are written in the same words.
- Its answer side is not already built. `--bin owed` already measures whether the tree names a
  term; the tree-facing sweep would be that measurement over a different population, which is a
  second population inside an existing binary rather than a new question.
- **It would not have found §12.11.** The parent's term was "Table 276" and source comments cite
  table numbers freely, so a tree-facing matcher would have found a witness and reported the claim
  corroborated. The contradiction was only ever visible from the child.

The cost of the one not taken is written down so a later round can take it: the code is small
because the extraction and the reach count already exist, and the whole price is reading — every
asserting row rather than the nine a family disagreement produces, each needing a judgement about
whether the sentence is about the standard or about us.

## Three judgements inside the instrument

**The denial vocabulary is `unread::CLAIMS` unchanged**, so this sweep and the second cannot drift
apart about what a denial is. **The assertion vocabulary is five words on a word boundary** —
"read" as a whole word is this ledger's verb, and the boundary keeps "unread", "reader", "reading"
and "already" out — against one idiom excluded by name: "**Read and kept** in the
five-hundred-and-sixty-fifth" says a *round read the row* and was two of the first run's hits.

**Stance is a property of a clause rather than of a sentence**, so `unread::sentences` could not be
reused: §14.12.4's row holds both stances inside one full stop — "Table 409's `/Start` and
`/DParts` are read; Table 408 is not" — and read whole it asserts the opposite of what it says.

The one noise shape a program can mark is *a table read in part*, and marking it needed the ninth
sweep's attribution rule rather than a plain key comparison. §12.11 is why: its row enumerates
"Table 273's `/S`, `/V` and `/Penalty`, Table 275's twenty-five types, Table 276's handlers", so a
mark counting every key in the part as the asserted table's would have demoted the Table 276 claim
on Table 273's keys — **the one defect the sweep was built for, printed as noise.**

## The first run: nine contradictions, two defects

170 rows have descendants and assert 118 terms between them, 49 of them corroborated by a child.
Four hits carry a demoting mark, two sit on the third rung, one is §14.9.2's partitive against
§14.9.2.2's fourth `/Lang` — both true — and two were defects.

**§9.9.1 said Table 125's `/Length1`, `/Length2` and `/Length3` were "read by nobody", and §9.9's
own row had contradicted it for twenty sessions.** `program::stated_extent` reads all three since
ADR 0459: `/Length1` alone for a `/FontFile2`, since Table 125 makes it "the entire TrueType font
program", and the sum of the three for a `/FontFile`, each stated in bytes "after it has been
decoded using the filters specified by the stream's Filter entry" — which is what makes it a fact
a reader can check. What the lengths are *not* used for is what the sentence was written about:
`read-fonts` finds the eexec boundary in the bytes, so no outline depends on them. The row carried
"**Read and kept in the five-hundred-and-forty-fifth session**", true when written and false
twenty sessions later. The fifth failure shape with the sign reversed inside one family: **a
parent that had outgrown its child.** `partial` is unchanged and is the `/Length3`-of-zero
requirement, still not executed, and the row now cites
`a_font_program_that_reaches_its_stated_length_survives_a_truncated_filter` for the half it does.

**§9.7.6 said "Table 119's entries are read" and its own child says one of the six is not.**
`/BaseFont` is deliberately unread for a Type 0 font on the clause's own NOTE — "an arbitrary
name, since there is no font program associated directly with a Type 0 font dictionary" — which
§9.7.6.1's row has said all along.

**And 641's own instance was planted back**, which is trap 13: with §12.11's pre-641 note restored
the sweep names it on rung 2, unmarked, quoting both sides; with the corrected note it names the
correction instead, marked as a row quoting the wording it retired. A sweep written over the wrong
side of a defect reports a clean tree.

`spec-errata emit` over all fourteen documents before writing: nothing at all between p. 364 and
p. 372, which is the whole of §9.9, and nothing over §9.7.6.1. The one erratum in the family is
§9.7.6.2's Issue #324, a cross-reference moved from §9.9.2 to §9.10, which touches neither row.

## A numbering collision, corrected on the way

The round was briefed as building "the thirteenth sweep", on `doc/todo/02` §4's count of twelve
committed programs, and that is right — this is the thirteenth of those. But `doc/todo/01`'s header
runs a different series, of *sweeps*, in which thirteen was already taken twice over: the ADR sweep
run once and declined (ADR 0265), and 637's proposal to check a note's prose against its own `code`
array. The two counts had been running together and one of them had two occupants. They are
separated rather than added to — **eighteen sweeps, thirteen of them committed programs**, both
derivable from the header, and 637's proposal is renumbered the nineteenth so it keeps its place in
the queue without owning an ordinal it never had.

## Gates

`tools/round.sh` called this a fifth round, so §2 ran whole — and it ran **twice**, because a
server-side overload cut the round in half at the sequence and the first half's figures were taken
while three neighbours were building. Everything below is from the re-run on an idle machine, which
is 626's rule applied without being made to: the oracle took 70.9s where the loaded run took 211.3s
and reported the identical verdicts.

`fmt` clean. `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` exit 0 — it caught
three things first, an `unused_mut` on a closure, two redundant method closures, and a doc link to
a private function, all fixed rather than allowed. `cargo nextest run --workspace` **2370 passed /
17 skipped**. Doctests clean. `RUSTFLAGS="-D warnings" cargo check --manifest-path fuzz/Cargo.toml
--bins` exit 0.

Corpus **974 documents, 68 incomplete**, 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless,
0 slow. Oracle **1794 pages: 907 agree, 66 contradicted, 786 ambiguous, 2 our geometry, 2 reference
geometry, 13 not comparable, 18 no render** — identical to 641's, verdict for verdict. `render-quorra`
957 pages: 932 agree, 23 differ, 2 refused, 17 not comparable. Text extraction 10969/11163 matched
words in bounds (98.26%) over 508 documents; PDFBox lane 99.8% (14257/14281); pdftotext lane 99.2%
(22834/23013). `selection_census` 1000/1011 words (98.91%) over 453 documents; `accessibility_census`
90 tagged documents, 1502 of 1558 pages answering, 876 of 876 untagged pages answering the honest
empty tree, 0 answering with structure they do not state; `dates` 1545 strings, 1514 conforming;
`xmp` 319 documents, 318 read; `jpeg2000` green; `fixed_documents` 33 checked, 0 absent.
`cargo test -p conformance` green — **875 rows**, and the status breakdown is unchanged at 436
implemented, 222 partial, 18 reported, 78 inapplicable, 8 writer-side, 113 out-of-scope. **No
`silent` row.**

Sweeps run because the ledger moved: `quotations` — 1716 ledger quotations, 1 diverging, and that
one is §8.9.5's and was there before; `pointers`, `counts` and `tables` printed their standing false
positives and no new hit, `tables` still at 6 denials, none of them this round's. §5's binaries
rebuilt and installed.

**One process failure worth recording.** The trap-13 proof plants §12.11's pre-641 note into
`ledger.toml` and restores with `git checkout --`, which is fine before the round's own ledger edits
and destroys them afterwards. It did: the second proof ran after the two corrections and took them
with it. They were reapplied from this file's own quotations and re-verified against `tomllib` and
the sweep. **Plant before you correct, or plant into a copy** — the restore is not a stash.
