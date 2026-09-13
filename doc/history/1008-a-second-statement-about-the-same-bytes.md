# 1008 — A second statement about the same bytes, and a `/ToUnicode` ten files did write

Date: 2026-09-12/13. Branch `batch-1006-1011`, worktree `/home/AI/pdf-viewer-rounds`, from `main` at
6c6a1d3d. ADR 1027. Topic: the robustness denominator — the oracle's unread heads, and the one
defect session 1002 left owed.

## What the round was asked for, and what it did

Two things. **Build the corroboration 1002 left owed** — `whole_program` reading Table 125's
`/Length1` on `Damage::CheckValue` as it does on `Damage::Truncated` — and **read another head** off
`doc/todo/00`'s rankings. Both are in ADR 1027; what is here is how each was reached and what was
run.

## Half one, which was specified

`pdf_font::program::whole_program` asked `stated_extent` only on a truncation, because ADR 0836's
paragraph said there was "no shortfall against Table 125's extent" on a check-value failure. That is
a sentence about *deciding*. It now asks on both damages and for two different jobs — decide on a
truncation, report on a check-value failure — and prints three answers: the extents differ (the
document corroborates the checksum and localises it), the extents agree (Table 125 was asked and had
nothing to add), or no extent is stated at all (`/FontFile3`, by §9.9's own sentence, and nothing is
printed). `bug1050040.pdf` prints the first; `issue13316_reduced.pdf` the second.

Three unit tests in `program.rs`, one per answer, on a fixture that writes a whole zlib stream and
flips its Adler-32. **Calibrated both ways** (trap 13): making `extent_corroboration` return the
empty string fails the two positive rows and correctly leaves the `/FontFile3` row passing;
exchanging the "short of"/"over" branch fails the direction assertion with "0 bytes short of".

## Half two, which was not

**The ink sweep's whole head is read**, which took a run to establish rather than a document.
`cargo run -p pdfref --bin undrawn` over the oracle log: 839 listed, 839 measured, 18 at or past the
alarm and 15 of them reported on — and every one of the 18 has a clause beside it already (eleven
ADR 0433, two ADR 0836, `checkbox_no_appearance.pdf`, `issue5954.pdf`, and the three complete names
`issue16038`/`issue12295`/`issue14297`). The gate's three rankings are the same story: its
`undiagnosed` list prints **empty**, and every name on the other two is held by an `AMBIGUOUS_*` or
`CONTRADICTED_*` group with an argument under it.

So the name taken was the first one *below* the alarm that is named nowhere in `oracle.rs`:
**`issue11915.pdf` page 1 at −0.636**. It is a font specimen — five lines naming five faces — and
§9.7.5.2 decides it exactly as ADR 0433 says. We are right. What it also turned out to be is the
witness for a report that was false: its five Type 0 dictionaries each state `/ToUnicode
/Identity-H`, a name where §9.10.1 requires a stream, and `collection_gap` ended every one of its
four facts with "it states no `/ToUnicode`".

**One command settled how much that was worth.** `examples/to_unicode_kind_census`, written for it:
16 font dictionaries over 10 documents in 1 239 opened files, every one the same name — and one of
the ten is `issue12418_reduced.pdf`, the head of the very ranking the round was reading. The report
on the most-looked-at page in this bucket had been saying the false half of that sentence since the
sentence existed.

**And the recovery was declined rather than not considered.** ADR 1027 §"Why the construction is not
recovered" has the clauses. The short of it: §9.10.3 defines a `/ToUnicode` `CMap` as a stream's
contents, Table 116's predefined `CMap`s map codes to CIDs rather than to Unicode, and that this
page's codes happen to be UTF-16BE is a fact about one producer. Three references landing on the
right letters is not a reason.

## What the four-panel strip said, because it is the clause's own argument

`mupdf` and `hayro` set all five lines correctly, `poppler` garbles the *Calibri* line into
`/1Ʈ⁄₆ρA`, `ghostscript` draws five lines of `^¸®±`. Same machine, same file, three pictures: what
each reference draws is an index into whatever face it found, so the page is correct exactly where a
substituted face's glyph order happens to match the producer's. That is the non-determinism
§9.7.5.2's `shall not` forbids, and a stronger statement than "four programs, four readings".

**And the first draft of this paragraph named the wrong two renderers**, because it read the panel
order off the strip instead of opening the four PNGs. Trap 1's own sentence, earned again.

## The golden, which found a page the round had written off

`raster_golden` over 974 tracked documents: **held 969, moved 5, unheld 0, left 0**, every one of
the five labelled `reports only (a change in the diagnosis, trap 37)`. The `.tsv` diff is the proof
that nothing drew differently — on all five rows the two raster-digest columns are **byte-identical**
and only the reports digest changed:

```text
-bug1050040.pdf p1  drawn 200x50  e2ce7238a89a97ff 2be2a4a410cc2a53 f2f98d6c390fd3d0
+bug1050040.pdf p1  drawn 200x50  e2ce7238a89a97ff 2be2a4a410cc2a53 f908f187a22f4387
```

Four of the five were predicted. **The fifth was not**: `issue5801.pdf` reached `to_unicode_gap`'s
third arm — a `/ToUnicode` that *is* a stream and states no `bf` mapping — which that arm's own doc
comment had said no corpus document reached. It is a copy of the `Identity-H` **CID** `CMap` in the
`/ToUnicode` slot, all `begincidrange`, and extending the census found **55 such streams over 27
documents** against sixteen names over ten. The comment is corrected with the number in it.

Each of the five was looked at (trap 1). `issue5801.pdf`'s four-panel strip is the one worth
recording: ours is blank and **all four references draw the same mojibake** — `SLĆWHN`,
`SRQLHGJLDãHN` — because all four take the CIDs as glyph indices into a substituted ArialBlack.
Four references agreeing, on nonsense. `issue12418_reduced.pdf` is ADR 0433's own four panels, four
strings.

## Files touched

- `crates/pdf-font/src/program.rs` — `stated`, `extent_corroboration`, the doc comment's corrected
  paragraph, three tests and a `zlib_stored_wrong_check` fixture.
- `crates/pdf-font/src/composite.rs` — `collection_gap`'s fifth fact, `to_unicode_gap`, one test and
  an assertion added to the existing four-fact test.
- `crates/pdf-font/src/loading.rs` — the one call site.
- `crates/pdf-model/tests/raster_golden.tsv` — the five reports digests, regenerated with the change.
- `crates/pdf-font/examples/to_unicode_kind_census.rs` — new.
- `doc/adr/1027-…` — new. `doc/adr/0433-…` — one clause corrected at the end.
- `doc/todo/00-ambiguous-bucket.md` §7 — the owed item struck and paid, the re-run recorded, the
  next name read.
- `doc/conformance/ledger.toml` — §9.9 and §9.10.1's `test` lists, and §9.10.1's `code`.

## Gates

**The worktree is shared with five siblings and their work-in-flight is in it**, so the
whole-workspace lines measure their crates as much as this one: `cargo fmt --all --check` and
`RUSTFLAGS="-D warnings" cargo clippy --workspace` were **red on `crates/pdf-transform/` and
`tools/conformance/`, neither of them this round's**. Scoped to what this round is under, and run
one walk at a time under `tools/bounded.sh --data 12 --tree 12`:

- `cargo fmt -p pdf-font --check`, and clippy over `pdf-font`, `pdf-model`, `pdf-render`,
  `render-cpu`, `pdf-syntax` with `RUSTFLAGS="-D warnings"` — clean.
- `cargo nextest run` over those five — 2074 passed, 17 skipped. Doctests: 0 and 0.
- `corpus`, `raster_golden`, `oracle`, `text_extraction`, `selection_census`,
  `accessibility_census`, `render-raster`'s corpus, `fixed_documents`, `pdf-syntax`'s `on_disk`,
  `cargo test -p conformance` — every one exit 0.
- The oracle's **verdict lines are identical before and after, page for page**, which is the
  statement a reports-only change owes.

Numbers are in the runs rather than here; the gate that this round's change is *about* is the
golden, whose movements are above. **A merge round owns the whole sequence on `main`**, and the two
red workspace lines above are what it will meet.
