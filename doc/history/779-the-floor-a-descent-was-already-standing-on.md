# 779 — The floor a descent was already standing on

The errata selection rule's tenth use. The live head moved for the first time in five uses —
by decay, the ninth use having read both members of the standing pair — and the full ranking
out-ranked it for the fifth time, so step 4 took the settled head: Annex L, `writer-side`,
whose two issues amend Table L.2 in two places, one of them a pair of cells whose published
value `c` the annex's own legend never defined. The settled head confirmed its row and paid
nothing; the walk downward paid three times on the live head §9.8.1. Issue #190 rewrites
`/Descent`'s "negative number" as *less than or equal to zero* — a one-word strike under
`check`'s four-word floor with three rustdoc blockquotes standing on it, and the zero-descent
acceptance `measured_extent` had argued as its own choice becomes the entry's own permission.
Issue #152 makes Table 257's `/P` an integer, closing a misread window no fixture could see: a
conforming `/P 1.0` was read as absent, level 2 in place of level 1, and only this round's new
test separates the integer read from a numeric one. Two of that issue's three strikes were
filed one clause late by the outline's page-straddle, the ninth use's coarseness met twice
inside one issue.

Date: 2026-08-28.
ADR: [0716](../adr/0716-the-floor-a-descent-was-already-standing-on.md), the number the
briefing reserved.

Touched: `crates/pdf-model/src/signature.rs` (doc comment on `modification`, one new test
calibrated per trap 13), `crates/pdf-font/src/metrics.rs`, `crates/pdf-font/src/substitute.rs`,
`crates/pdf-model/src/variable_text.rs`, `crates/pdf-model/tests/variable_text.rs` (comments
only — the amended wording recorded beside the published blockquotes), `doc/conformance/
ledger.toml` (§9.8.1, §12.8.2.2.1, §14.8.5.4.4, Annex L, reformatted by its own binary),
`doc/errata-read.md` (tenth-use section), `doc/todo/01`, the ADR and this file.

## What the rule gave

Under the recipe's own single-issue line parse, 302 issue numbers in
`doc/ISO_32000-2_sponsored_EC3.pdf` carry a strike or a caret and **104 were named nowhere** at
this round's base — the ninth use's closing arithmetic (111 less its seven verdicts) reproduced
exactly by the greps, the first time base and derived closing figure have agreed. A parse that
also reads the multi-issue annotation lines counts 310 and 106 and moves no head. Five issues
gain verdicts this round: the settled head's two, and the live head's three. The remaining
five-annotation plateau's rows were not read and their issues stay in the population.

## What paid

- **Issue #190 (§9.8.1, Table 120's `/Descent`)**: the floor becomes *less than or equal to
  zero* and an inserted NOTE says font programs write descenders in either sign while PDF
  always expects negative values. Three blockquotes stood on the struck sentence — the gated
  one in `metrics.rs::measured_extent`, `variable_text::Metrics::read`'s and a test's — and
  the code's acceptance of a zero descent, argued in place as this program's reading, is the
  clause's own rule now. The NOTE names the mechanism of the corpus's 42 positive descents,
  corroborating ADR 0216's repair without legalising the form. Behaviour unchanged everywhere.
- **Issue #152 (Table 120, Table 257, Table 380 — one type cell each, `number` → *integer*)**:
  `signature::modification`'s `as_integer` was the amended read before the amendment; under the
  published cell a conforming real `/P 1.0` was read as absent — the default level 2 in place
  of the stated level 1, a permission-widening misread no existing fixture could see.
  `a_docmdp_level_written_as_a_real_takes_the_tables_default` pins the recovery, calibrated per
  trap 13: the numeric-read plant passes all 36 pre-existing signature tests and fails only the
  new one, run both ways and reverted. Table 257's strike prints under §12.8.2.3 and Table
  380's under §14.8.5.4.5 — both one clause late, the outline's page-straddle, recorded in the
  mis-filed rows' notes.
- **Issue #474 (§9.8.1, `/FontWeight`)**: the value becomes an integer between 1 and 1000
  inclusive (`shall`) with the published nine hundreds a `should`. `substitute.rs`'s bold
  threshold at 600 reads every conforming value under either printing; its `as_number` read is
  now stated as a reader's tolerance.
- **The settled head (Annex L, Issues #83 and #440)**: `Table` becomes a valid child of `P`
  (0..n, both directions), and the `WP`/`Figure` pair's two cells go from `c` — a value Table
  L.1 never defined, so a normative constraint with no meaning — to 0..n. Nothing here reads a
  cell; the row's note now carries the amended matrix for the checker it promises.

## Gates

Full §2 sequence, `PDFREF_CACHE` at the shared warm cache, all green. `fmt` and
`clippy -D warnings` silent; nextest 2718 passed, 18 skipped — one more test than the
seven-hundred-and-seventy-sixth's merge count, which is this round's; doctests clean; the fuzz
`check` clean; both trap-10 workers built. Corpus: 974 documents in 3.3 s — 0 unopenable,
8 locked, 2 encrypted beyond us, 6 pageless, 67 incomplete, 0 slow. Oracle: 1945 pages in
70.1 s at a 100.0% cache hit rate — 983 agree, 61 contradicted, 836 ambiguous, 3 our geometry,
2 reference geometry, 42 not comparable, 18 no render — exit 0, no ratchet moved. The three
text gates and the selection spread: 4 passed in 35 s. Both censuses green (selection 13.7 s,
accessibility 27.8 s). Dates, XMP, JPEG 2000 green. Quorra: 957 pages in 113.9 s — 932 agree,
22 differ, 3 refused, 17 not comparable. `fixed_documents`: 40 checked, 0 absent.
`cargo test -p conformance`: 23 result lines, all ok.

**The launch test the briefing flagged passed**, inside the workspace run, at a load that began
near 30 with sibling rounds building — the seven-hundred-and-seventy-sixth's load-robustness
holding, where the seven-hundred-and-seventy-fourth saw it fail twice under less.

The warm reference cache still carries mupdf failure messages quoting a sibling worktree's path
(r707) — trap 10a's shape in the message text only, the cached verdict being path-independent,
as the two rounds before the ninth use also noted.

## Sweeps

Sixteen sweeps against the pristine tree before the edits and again after them — the baseline
taken first, before any file moved, per the ninth use's contamination lesson; `quoted` and
`unpriced` not run, no page-list note touched. Every delta is the round's own work:
`pointers` +12 paths (+3 live, +7 unrooted, +2 not carried) and +3 symbols, absent unchanged at
98 and undefined at 13; `tables` +32 sentences and +9 key citations, all agreeing, absent
unchanged at 101; `inapplicable` +3 terms, all from §14.8.5.4.4's amended note; `counts` +32
sentences, every attribution bucket unchanged; `owed` +4 terms with both row counts unchanged;
`quotations` +5 document quotations over +2 documents with **diverging unchanged at 38 and 2**;
`overstated` corroborated 61 to 62, contradicted unchanged; `overtaken` +1 decision record,
overtaken unchanged at 45; `applied` +47 places read and +25 naming an erratum, with the
comparison counts, the read-first list (10) and the correction-shaped count (91) all unchanged
— the round's new erratum-naming places quote amended text in italics, which is why they add no
comparisons; `check` byte-identical, the round's quoted strikes all being under its four-word
floor. `entries`, `unread`, `blockers`, `capabilities`, `callers` and `parts` unchanged.

## What contradicts the briefing

- **The briefing's "population 104 after it" is confirmed by the greps rather than trusted**:
  302 issues carry a strike or a caret under the recipe's own single-issue line parse and 104
  are named nowhere at this round's base, which is the ninth use's 111 less its seven verdicts
  — the first use at which base and derived closing figure agree.
- **The briefing described the last five uses as each finding "a settled row whose evidence
  covered fewer written forms than the clause states"; the tree records that as one of five
  distinct mechanisms** (ADR 0712's list: a round trip that could not fail, a sentence about a
  sibling row, a set with no closure check, a two-written-forms claim with a test of one, a
  fixture too small to exercise a rule) — and this round's settled head paid nothing at all,
  while the fixture-too-small shape appeared on a *live* family instead, §12.8.2.2.1's `/P`.
  The briefing's "expect the mechanism, do not assume it" was the right instruction.
- The launch test in `viewer-host` passed under load rather than failing, so there is nothing
  to record against round 776's repair. Main's CI failure `round.sh` flags (run 33121581297) is
  the pre-existing owner-arc one the briefing names; this round's own clippy under
  `-D warnings` is silent, so it does not reproduce here.
