# 720 — The report that asked the style, and two errata that cannot both be applied

The ledger's `partial` rows read as a family for the fourth round running, on ADR 0538's method for
the seventh block, with the pair chosen by ADR 0567's search under 0593's third rule — **take the
strongest pair the previous round named and did not read.** That rule took this round out of §12.5
for the first time in four, into §12.4.4 ~ §12.4.4.1, where the two rows contradict themselves in
the same sentence; the reading found a report keyed on less than what decides the drawing, and
`spec-errata emit` over the pages found two accepted errata that deny each other.

Date: 2026-08-25.
ADRs: [0600](../adr/0600-the-report-that-asked-the-style-and-the-frame-that-asked-the-direction.md),
[0601](../adr/0601-two-accepted-errata-that-cannot-both-be-applied.md).

Touched: `doc/conformance/ledger.toml` (§12.4.2, §12.4.4 and §12.4.4.1),
`crates/viewer-core/src/transition.rs` (`note` widened, `askew` new, one test added and one widened),
`crates/viewer-core/src/viewer.rs` (two call sites), `crates/viewer-host/src/clock.rs` (one doc
comment), `crates/viewer-core/tests/headless.rs` (one test), `crates/pdf-model/src/page_label.rs`
(one doc comment), `crates/pdf-model/examples/presentation_census.rs` (a `/Di` tally),
`doc/errata-read.md`, `doc/environment.md`, `doc/todo/01`, the two ADRs and this file.
**No status moves and no pixel moves; one report is widened and it fires on nothing that exists.**

## Why the ranking left the family it had been in

The search was run rather than read out of a document, with 710's two rules and 716's third. **The
family order did not move** — §12.5 heads it, §12.8 second, §12.7 third — and the third rule sends a
round somewhere else entirely: ADR 0593 §1 named two pairs stronger than the one it took and left
both. The stronger is §12.4.4 ~ §12.4.4.1, the strongest pair below any clause-level parent in the
whole ledger.

**That is the rule working rather than an exception to it.** Self-reinforcement is a property of the
family a round has just written in, so a rule that only reordered pairs *inside* the head family
could never escape it.

## The three findings

- **Both rows contradict themselves about `R`, and the correction had been made twice above them.**
  Each says seven of Table 164's twelve styles are shaped and **four** of the other five reported,
  `R` being the cut the table defines — and then, three lines later, that the clause is "`partial`
  for the five styles". §12.4's parent row was corrected to four in the
  three-hundred-and-eighty-eighth session and the *middle* sentence of each of the pair in the
  six-hundred-and-sixty-third; both times the closing tally was left standing. A correction reaching
  the sentence that states a mechanism and not the sentence that counts it, twice, in the same
  family.
- **The report asked the style; the frame asks the style and the direction.**
  `viewer_core::transition::note` took a `&Style`. `frame` takes a `&Transition`, and for `Wipe`,
  `Cover`, `Uncover` and `Push` it calls `quarter(transition.direction)?` — so a `/Di` outside the
  four quarter turns shaped **no frame and produced no sentence**: a cut in silence, which is the
  one outcome trap 5 exists to prevent and the one `note` was written for. `Clock::shapes`'s doc
  comment asserted the property that had failed, and
  `only_the_four_quarter_turns_name_a_sweep`'s own doc comment had claimed the report existed since
  the three-hundred-and-ninety-third session. One question answered in two expressions — 701's
  shape, 716's, and now three rounds running. `askew` asks `quarter`, the same expression `frame`
  refuses on, rather than restating the list; the property test holds `note` against `frame` over
  thirteen styles crossed with seven directions.
- **Two accepted errata on Table 161 that cannot both be applied.** Issue **#432**
  (`Review`/`Accepted`, 2024-06-17) strikes `ZZ` for `AZ` in "A to Z for the first 26 pages, AA to
  ZZ for the next 26" — the odometer. Issue **#593** (`Review`/`Accepted`, 2026-05-21) strikes
  nothing and inserts "AAA to ZZZ for the next 26, AAAA to ZZZZ for the next 26," after the same
  clause — the repeat. #432's strike rectangle matches `pdftotext -bbox`'s box for the word `ZZ` on
  physical page 474 to six decimal places; #593's caret sits on the same line just past the `26,`
  that ends the clause #432 rewrites. `page_label::letters` produces the repeat, on the published
  sentence's own count, and is unchanged: **an erratum is evidence about the standard in the way
  another renderer is evidence about our reading**, and where two disagree the clause and its
  arithmetic decide. Neither could have been printed by `check` — one is a caret with no strikeout,
  the other a one-word strike.

**And `emit` against §12.4.4 itself found nothing new**, which is the answer the round wanted:
Issues #36 and #75 and no others, both already recorded in §12.4.4.1's row.

## What the census says it costs

`examples/presentation_census` gains a `/Di` tally on the four styles that travel along one. Over
`CC-MAIN-2021-31` it reproduces the row's three figures exactly — 65 703 documents opening, **276**
stating a `/Trans`, 86 a `/Dur`, 1 a `/PresSteps` — and of the **464** transitions on those four
styles, **every one states 0, 90, 180 or 270.** Not one states 315, the name `None` or a fractional
angle. So the widened report fires on nothing in the crawl and can fire on no conforming file
either; what was wrong was the shape of the decision rather than a picture.

The same run settled a smaller thing: the **only** unrecognised `/S` anywhere in the crawl is the
**empty name**, on 106 pages of seven documents, beside the private keys `/Curve` and `/Directional`.
The sentence it produced read `transition: / is not one of Table 164's styles`; an empty name is
described now instead of printed as a bare slash.

## A worktree lesson, and it cost the round twenty minutes

**`git checkout -- doc` takes a parallel worktree's submodule symlinks away.** The before-and-after
sweep needs the tree at `HEAD`, and a *directory* argument restores `doc/arlington-pdf-model`,
`doc/pdf.js` and their two neighbours as **empty directories** — the links into the main worktree
are gone, `git status` says nothing, and what a round sees is `pdf-spec`'s build script panicking
with *It is a git submodule; run: `git submodule update --init`*, whose advice would clone a second
copy rather than restore the link. Named paths do what the directory argument was meant to do.
`doc/environment.md` carries it beside the `git add -A` rule it belongs with. Re-running every sweep
with the links intact gave **byte-identical output** to the run taken while they were broken, so no
number here is affected.

## Gates and sweeps

`PDFREF_CACHE` pointed at the shared warm cache, `/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`.
`tools/round.sh` says this is a fifth round and the change is in `viewer-core`, so §2 ran whole and
§5's binaries were rebuilt and installed — `round.sh` had flagged `target/` as holding none of them.

`fmt`, `clippy -D warnings`, `nextest`, the doctests, the fuzz `check`, the sandbox worker, corpus,
`pdfref-hayro`, oracle, text extraction, selection, accessibility, dates, XMP, JPEG 2000, quorra,
`fixed_documents` and `cargo test -p conformance` all green, the last of them after the final edit.
**The lines that spawn a reference renderer were held until the load fell**, which is §2's own rule:
the oracle ran at a one-minute load average of 9 on 24 cores, down from 33, and the text-extraction
and quorra lines at 19 as three parallel rounds picked up again — the oracle reported 32 seconds and
its verdicts, and the extraction cache took 958 hits and no misses, both of which are what an
unloaded run looks like here. The only clippy output was `viewer-qt`'s cold-build gcc
`-Wmaybe-uninitialized` lines, which §2 documents as not lints. `--bin quoted` was run against the
oracle's own log and is a level rather than a delta, because this round touches no page-list note.

Thirteen sweeps run before the edits and after them, with the three errata commands beside them.
**One level moved into a defect bucket on this round's own prose and it was put back**: `--bin
counts` went 58 "no such way" to 59 on a sentence of this round's in `doc/todo/01` reading "these
two rows' *middle* sentences" — a cardinal governing one of the ledger's words for a row, 691's
noise shape exactly. It says "the *middle* sentence of each of the pair" now and the level is back
at 58.

Everything else moved by what the new prose contains and nothing landed in a defect bucket. Final
levels, after ← before: `counts` 7684 ← 7663 sentences with 410 ← 409 attributed counts, **58 "no
such way" and 4 places counting one family twice both times**; `quotations` 6052 ← 6037 document
quotations over 911 ← 908 documents with **diverging unchanged at 36**, and 1918 ← 1915 ledger
quotations with **diverging unchanged at 2**; `tables` 6401 ← 6375 sentences and 2390 ← 2387 key
citations with **absent unchanged at 100 and contradicted denials at 6**; `pointers` 7971 ← 7949
with **absent unchanged at 131 and undefined at 13**; `owed` 3802 ← 3797 terms over 223 `partial`
rows with **179 unnamed over 112 rows unchanged**; `overtaken` 544 ← 542 decision records with
**44 overtaken unchanged**;
`blockers`, `entries`, `unread`, `inapplicable`, `overstated`, `capabilities` and `callers` all
unmoved. `spec-errata check` is unchanged but for the line numbers this round's own insertion into
`doc/errata-read.md` shifted, and `applied`'s three comparison counts — 90 quoting a replacement, 10
matching both sides, 171 quoting what an erratum struck — are unchanged over a population that grew
by 18 places naming an erratum, which is this round's own writing.
