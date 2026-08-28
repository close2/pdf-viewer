# 789 — The recovery one entry states

The errata selection rule's eleventh use. Both rankings topped out in plateaus of one-issue
rows: the full ranking's head was a settled pair tied at five — §7.5.4, whose real content was
one strike the outline had filed a clause late (§7.5.2's binary-marker sentence, a writer's
rule this tree cannot break), and §13.6.3.1, inside the multimedia exclusion — and the live
head a four-way tie at five that confirmed four times over. The payment sat one rank below
both: §7.7.2 at four annotations under four distinct issues. Issue #105 inserts *or invalid
(see 14.9.2)* into Table 29's `/Lang`, so an invalid catalog language identifier is unknown
exactly as an absent one — and this reader was carrying an invalid tag to every consumer as if
it named a language, because the published entry stated the recovery for absence alone.
`structure::document_language` now answers `None` for a tag that fails BCP 47's grammar —
well-formedness, self-contained, deliberately not the registry judgement — and §14.9.2.2's
reason of record, one sentence that had conflated those two judgements, is retired with the
half of it that was right still standing. Fourteen issues left the population, the rule's
largest single decay.

Date: 2026-08-28.
ADR: [0724](../adr/0724-the-recovery-one-entry-states.md), the number the briefing reserved.

Touched: `crates/pdf-model/src/structure.rs` (`well_formed_language_tag` and its grammar
walker, `document_language` filtering the catalog tag, one unit test),
`crates/pdf-model/tests/accessibility.rs` (one end-to-end test, calibrated per trap 13),
`doc/conformance/ledger.toml` (§7.5.2, §7.5.5, §7.5.7, §7.7.2, §12.5.6.5, §12.7.5.5, §14.7.2,
§14.9.2.2, reformatted by its own binary), `doc/errata-read.md` (eleventh-use section),
`doc/todo/01`, `tools/worktree.sh` (root resolution — see below), the ADR and this file.

## What the rule gave

Under the recipe's own single-issue line parse, 302 issue numbers in
`doc/ISO_32000-2_sponsored_EC3.pdf` carry a strike or a caret and **99 were named nowhere** at
this round's base — the tenth use's closing arithmetic (104 less its five verdicts) reproduced
by the greps, the second consecutive use at which base and derived closing figure agree. The
multi-issue parse counts 310 and 101, the tenth's own figures less the same five. Fourteen
issues gain verdicts this round; the closing population is 85, re-derived by the same greps
after the records were written.

## What paid

- **Issue #105 (§7.7.2, Table 29's `/Lang`)**: one caret, and the entry's last sentence
  becomes: if this entry is absent or invalid, the language shall be considered unknown.
  Implemented as BCP 47 well-formedness (RFC 5646 section 2.1's grammar, grandfathered tags and
  private tails included; the registry deliberately not consulted), applied to the catalog
  entry alone because it is the only place the standard states the recovery — an element's
  invalid tag treated as unknown would also cancel §14.9.2.3's inheritance. Calibrated per
  trap 13 above the round's commit, both ways: the no-validation plant passes all 19
  pre-existing accessibility tests and fails only the new one; the reject-everything plant
  fails the three older tests that assert a valid tag is carried, plus the grammar test.
- **§14.9.2.2's reason of record retired**: "a BCP 47 grammar would be a judgement about a
  registry this program does not hold" conflated well-formedness with validity — the cheap
  judgement declined at the dear one's price. No sweep reads a reason's internal logic; only
  the ranking put a round in front of the sentence.
- **Thirteen more issues to verdicts, none moving code**: §7.5.2's binary-marker tightening
  (writer's; the straddle placed before any verdict), example typography across four clause
  families, §13.6.3.1's pair inside the exclusion, Table 237's `/DigestMethod` shape behind
  the `/SV` refusal, the link `QuadPoints` NOTE that states `link.rs`'s own construction, two
  `/BS` version markers to PDF 1.3, the `/Size` NOTE, namespace completeness rules on the
  writer, and `/Extensions` direct / `/StructTreeRoot` indirect / encrypted-catalog-not-in-an-
  object-stream, each a writer's rule under a stated reader's tolerance.

## Gates

Full §2 sequence (pdf-model changed, so the map's first row applies), `PDFREF_CACHE` at the
shared warm cache, all green. `fmt` and `clippy -D warnings` silent; nextest 2730 passed,
18 skipped — the launch test passed inside the workspace run; doctests clean; the fuzz `check`
clean; both trap-10 workers built. Corpus: 974 documents in 5.2 s — 0 unopenable, 8 locked,
2 encrypted beyond us, 6 pageless, 67 incomplete, 0 slow. Oracle: 1945 pages in 62.5 s at a
100.0% cache hit rate — 983 agree, 61 contradicted, 836 ambiguous, 3 our geometry,
2 reference geometry, 42 not comparable, 18 no render — exit 0, no ratchet moved. The three
text gates and the selection spread: 4 passed in 34.2 s. Both censuses green (selection
25.6 s, drag 98.91%; accessibility 28.7 s — the language change moved no ratchet). Dates,
XMP, JPEG 2000 green. Quorra: 957 pages in 26.5 s — 932 agree, 22 differ, 3 refused, 17 not
comparable. `fixed_documents`: 40 checked, 0 absent. `cargo test -p conformance`: 23 result
lines, all ok — re-run after every document edit, and it is the gate that had earlier refused
`RFC 5646 §2.1` (a `§` reserved for ISO 32000-2), corrected to the section spelling.

The warm reference cache still carries mupdf failure messages quoting a sibling worktree's
path (r707) — trap 10a's shape in the message text only, as the three rounds before also
noted.

## Sweeps

Sixteen sweeps against the pristine tree before any edit and again after them; `quoted` and
`unpriced` not run, no page-list note touched. **The ninth sweep paid on the round's own
work**: its first after-run printed two wrong table numbers this round had just written —
the seed value dictionary attributed to Table 236 (the lock dictionary's) and the 3D stream
dictionary to Table 302 — in the ADR, the record and one ledger note; both corrected to
Table 237 and Table 311 and the re-run agrees. The `unread` sweep's first after-run also
flagged a phrasing of this round's §14.7.2 sentence that read as a `/NS` unread-claim;
reworded, and the row is back to its two pre-existing hits. Every remaining delta is the
round's own work: `pointers` +3 paths, all live, absent unchanged at 98 and undefined at 13;
`tables` +24 sentences and +23 key citations, all agreeing, absent unchanged at 101;
`counts` +34 sentences, every bucket unchanged; `owed` two rows gain terms with every one
named; `quotations` +8 over +1 document with diverging unchanged at 38 and 2; `overtaken`
+1 decision record, overtaken unchanged at 47 — and its note population counts one more,
because the grammar's `GRANDFATHERED` const reads to the sweep as a page-list note citing no
ADR; `entries` +1 row explaining itself by an arrival, reported counts unchanged;
`inapplicable` +1 RFC-naming file and a cousin-row shift to §7.7.2; `applied` +61 places and
+22 naming an erratum, read-first list unchanged at 10; `check` +1 in the known
struck-out-of-another-clause document bucket — the record's #522 row quotes Table 15's
caption, which sits inside a struck example paragraph, the file's own known false-positive
shape. `overstated`, `blockers`, `callers`, `parts` and (after the reword) `unread`
identical.

## What else the round touched, and why

`tools/worktree.sh` resolved its root with `--show-toplevel`, so every command in it run from
*inside* a worktree took the worktree as the main tree: `list` printed all four **live**
sibling build directories as "ORPHANED — its worktree is gone", and a `close` trusted the same
wrong root — a data-loss footgun one `rm -rf` wide. The root is now the parent of
`--git-common-dir`, the one path all worktrees share; verified from the main tree and from
inside this one, which now agree.

## What contradicts the briefing

- Nothing does. The briefing's population figure — 99 after the tenth use's five verdicts —
  is confirmed by the greps rather than trusted, and the parse-scope note it carried (the
  tenth's 310/106 against the ninth's 307/112 on multi-issue lines) reproduces here as
  310/101, which is the tenth's own scope less its five verdicts.
- The launch test passed inside the workspace run; per the briefing a failure would have been
  news, and there is none.
- Main's CI failure `round.sh` flags (run 33121581297) is the pre-existing one the briefing
  names; this round's clippy under `-D warnings` is silent, so it does not reproduce here.
