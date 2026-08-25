# 739 — The entry that says which part of a validation matters

The successor selection rule's second use. It ran, it corrected its own second step, and the head
it then named had never moved off the head.

Date: 2026-08-25.
ADR: [0637](../adr/0637-the-entry-that-says-which-part-of-a-validation-matters.md).

Touched: `crates/pdf-model/src/signature.rs` (one struct field, one method, one test, five test
literals), `crates/pdf-model/examples/signature_algorithm_census.rs` (one counter, one report),
`crates/viewer-core/src/notes.rs` (one note, one test), `doc/conformance/ledger.toml` (§12.8 and
§12.8.1), `doc/errata-read.md`, `doc/todo/01`, `doc/state-of-play.md`, the ADR and this file.
**No pixel moves.** One entry of Table 255 gains a reader and one sentence is added to what a
signed document is told about itself.

## What the rule gave

`spec-errata emit` over `doc/ISO_32000-2_sponsored_EC3.pdf`, the issue numbers this tree names, the
attribution to the nearest live ledger row — the recipe in `doc/todo/01`, run rather than read.

**Run as written it named §12.7.5.5, and that was the recipe's own defect.** Step 2 asks for issue
numbers carrying the `Issue #` prefix, for the `&#124;` reason the round before recorded; but
`doc/errata-read.md` — the tree's record of every erratum it has read — writes its numbers **bare,
in a table column**, so two of §12.7.5.5's four "unnamed" issues carry a verdict there already. The
prefixed grep found 113 of the 351 issues carrying an annotation while that one file records 159.

**A bare-number grep is not the repair**, and this is the second collision family:
`doc/HAYRO_ISSUES.md` and `doc/HAYRO_ISSUES_FOR_QUORRA.md` list another project's GitHub issues and
name `#54`, `#55`, `#680` and `#681` — four of the five errata this round read. A number-only search
answers *recorded* from a document about a different tracker. `doc/todo/01`'s step 2 is two greps
now, unioned, with the numeric character references stripped from the second.

**With step 2 repaired the head is §12.8.1**, which ADR 0627's reconstruction across nine bases said
had been in the top six at every one of them with nobody on it. Nine annotations, five issue
numbers, none recorded: #54, #55, #117, #121 and #219 — and #219 had to be reassembled across two
clause headings, seven annotations over Table 20 and Table 255.

## What the issues said

`doc/errata-read.md` has all five with the rectangle that places each. Three are editorial or
informative: a struck `0` in `attrib0ute` (§12.7.9's, filed under §12.8.1 by the page-straddle),
PDF version markers on six `/SubFilter` values, and "These shall follow the certification signature
if one is present" extended to the document timestamp bullet — a rule about how a file is
assembled, which this program does not do.

Two had teeth:

- **#117 strikes `(Required)` from Table 256's `/DigestMethod` and writes *Optional; deprecated in
  PDF 2.0*.** §12.8.1's ledger row quoted the retired word. `spec-errata check` cannot see a
  one-word strike — it is under the four-word floor — so a quotation sat on retired text in the one
  population that has a gate. The correction is more than the words: an entry that was required and
  unread is a debt, and one PDF 2.0 deprecates is met on older files alone. The strike also leaves
  standing the NOTE below it, "[t]he DigestMethod key was also corrected to be required as no
  default value is defined", which now reports a correction the erratum has undone; the cell decides
  and the NOTE does not, which is the second place this collection disagrees with the document it
  amends.
- **#121 strikes `; inheritable` from Table 255's `/Filter`, and it vindicates the code.**
  `signature::read` asks `Document::get_key`, which resolves and inherits nothing, so `/Filter` was
  never looked for up a `/Parent` chain — and until this erratum the table said it should be.

## What reading them made this round look at

**§12.8.1's note said "Table 255 entire" and named thirteen of the table's eighteen entries.** #121
is on the `/Filter` row, so the table had to be read against the row. `/R`, `/V`, `/Prop_Build`,
`/Prop_AuthTime` and `/Prop_AuthType` had no reader at all. Four are declined in the row with the
entries' own words — `/R` withdraws itself, `/Prop_Build`'s use is defined by a document this
project does not hold, and the two `Prop_Auth*` entries are "used in claims of signature
repudiation".

**The fifth was work.** Table 255's `/V`: "[t]he value is 1 if the Reference dictionary shall be
considered critical to the validation of the signature." That is the one sentence of the entry
addressed to a validator, and this program evaluates no transform method — so a file writing `/V 1`
names the part of its own validation this program skips, and nothing here read the entry.
`Signature::format_version` reads it, `Signature::reference_is_critical` applies the entry's own
condition, and `viewer_core::notes` says one sentence beside the paragraph that already names the
questions which went unanswered. §12.8's row said "Table 255 whole" for the same reason and is
corrected with it.

**The population was measured before the report was written**, and the command is
`examples/signature_algorithm_census`, which now counts Table 255's `/V` beside the identifiers it
already counted. No curated document states the entry, so the witness is hand-built (trap 8); the
crawl holds the files that do, and the two writing `/V 1` each carry a `DocMDP` and a `FieldMDP`
signature reference dictionary and a `/SigFieldLock` — exactly the material the sentence is about.

Calibrated per trap 13, two ways from one fixture: with the guard forced false the test fails on the
signature stating `/V 1`; widened to *the entry is present* it fails on the signature stating `/V 0`,
which six crawled files write. Both plants removed.

## Gates and sweeps

`PDFREF_CACHE` pointed at the shared warm cache,
`/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`. `tools/round.sh` says this is not a fifth
round, but the change is in `pdf-model`, so §2's map asks for everything and everything was run.
§5's binaries were rebuilt and installed — `round.sh` had flagged `target/` as holding none of them.

`fmt`, `clippy -D warnings`, `nextest`, the doctests, the fuzz `check`, the sandbox worker, corpus,
`pdfref-hayro`, oracle, text extraction, selection, accessibility, dates, XMP, JPEG 2000, quorra,
`fixed_documents` and `cargo test -p conformance` all green. The only clippy output was
`viewer-qt`'s cold-build gcc `-Wmaybe-uninitialized` lines, which §2 documents as not lints.
`cargo nextest run --workspace` failed once and the failure was this round's:
`every_quotation_is_the_standards_own_words` named the new `/V` doc comment, whose blockquote opened
before the comment had cited a clause. The clause is named now. **The oracle says no pixel moved**,
which is what a change to what a document *says about itself* should look like.

The reference-spawning lines were run at a one-minute load average between 13 and 22 on 24 cores —
higher than the section would like, and the guard against reading a false failure is that both
passed with the verdicts and bounds a quiet machine gives; the oracle's own summary is in the run.

Thirteen sweeps run before the edits and after them, with the three errata commands beside them.
`quoted` and `unpriced` were not run: this round touches no page-list note and both take the
oracle's log as their right-hand side. `entries`, `overstated` and `unread` are byte-identical;
`spec-errata check` and `moved` are byte-identical; `blockers` and `capabilities` differ only in
line numbers this round's insertions shifted.

**One level moved and it moved for the right reason.** `--bin owed` went 179 unnamed terms over 112
rows to **183 over 113**, and the four are `/R`, `/Prop_Build`, `/Prop_AuthTime` and
`/Prop_AuthType` — Table 255 entries no source in this tree names, now named under a `partial` row.
That is not this round's prose tripping the sweep the way 734's census result did: it is the
ledger's own definition of `partial` being met, which is that the note says which requirements are
not executed. The sweep counting them is the sweep working, and §12.8.1 leaving the reading list is
the correct answer to the question it asks.

Everything else moved by what the new prose contains and nothing landed in a defect bucket. The
levels below are the **last** run, after the final edit, per §2's rule that a number is current only
for the round that ran the gate last; after ← before: `counts` 7927 ← 7896 sentences with **412
attributed counts, 58 "no such way" and 4 places counting one family twice, all three unchanged**;
`quotations` 6236 ← 6218 document quotations over 951 ← 949 documents with **diverging unchanged at
38**, and 1940 ← 1933 ledger quotations with **diverging unchanged at 2**; `tables` 6582 ← 6550
sentences and 2446 ← 2430 key citations with **absent unchanged at 100, contradicted denials at 6
and keyless at 58**; `pointers` 8233 ← 8209 with **absent unchanged at 131**, and 137 symbol
pointers with **13 undefined, both unchanged**; `callers` 327 ← 326 `pub fn` names in `pdf-model`
with **135 named by no crate, unchanged** — the new one is called by `viewer-core`; `overtaken` 564
← 563 decision records with **43 overtaken unchanged**; `inapplicable` unchanged at 55 / 233 / 224,
one cousin row swapping §12.8.3 for §12.8.1. `spec-errata applied` grew to 677 ← 660 places naming
an erratum over a population of 55 852 ← 55 787, with **the read-first list unchanged at 10, the
corrections quoting retired wording at 90 and the quotations of struck text at 172**.
