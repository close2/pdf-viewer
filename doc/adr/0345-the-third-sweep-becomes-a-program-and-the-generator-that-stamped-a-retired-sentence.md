# 0345 — The third sweep becomes a program, and the generator that stamped a retired sentence back

**Status.** Accepted.

## Context

`doc/todo/01`'s binding rule for a sweep round: commit one more prose sweep as a program before
running any of them. Four of the fifteen were commands — `conformance --bin entries`,
`--bin quotations`, `--bin unread` and `--bin blockers` — and the capability-reason sweep was the
most-run of the eleven still living as descriptions: "this program has no ___", "no panel",
"which this is not", re-derived by hand on every run since the hundred-and-twenty-second session.
It is also the sweep with the longest list of paid findings behind it — §12.6.3's "this crate has
no events" forty-one sessions after `Command::Pointer`, §12.3.2.1's "a window with scrolling and
zoom, which this program does not have" sixty-nine sessions after both commands entered the
vocabulary (ADRs 0122, 0162) — which made it the natural fifth.

## Decision

**`cargo run --release -p conformance --bin capabilities`** is the third sweep as a program
(`tools/conformance/src/capabilities.rs`), and it carries the two judgements every by-hand run
redid:

- **The witness question.** The thing a sentence says is absent is extracted — after "has no"
  and its kin it follows the phrase, before "which this is not" and "does not have" behind a
  relative clause it precedes — and the tree is searched for it, whole-word, singular or plural,
  longest word first. A hit whose lacking noun the tree names prints the witness path and line.
  This is `doc/habits.md`'s "a capability recorded as absent is worth one grep of your own tree"
  as a program.
- **The subject tell.** A claim about *the program* is the population that decays, because a
  capability arriving anywhere expires it; a claim about *one crate* is usually a boundary that
  crate keeps on purpose — no clock, no filesystem, no toolkit — and stays true however much the
  program grows. Hits are printed program-first, witnessed-first.

The three noise shapes are printed rather than filtered, as with every sweep here: the
true-boundary statement (dominant), the correction quoting its retired wording (`[history]`),
and a witness that is one short identifier in two clauses (`form::Control` witnessing "no such
control"). Not a gate, on ADR 0249's ratio argument.

**First run as a program**: ledger 47 capability sentences — 34 witnessed by the tree, 40 about
the program, 7 about one crate; source 142 — 116 witnessed, 78 program, 64 crate. One ledger hit
was live: **§14.7.6.3** disposed of attribute revision numbers with "a validity mechanism for a
processor that edits, which this is not" — expired since the hundred-and-thirty-fifth session
made this program one that edits and saves, with a filled widget a content item wherever an
`OBJR` names it. The conclusion survives on the clause's own words, read now rather than
assumed: the subclause is "deprecated with PDF 2.0" and the increment is a *may* — a permission
declined, recorded as a choice. The fifth failure shape: right conclusion, expired argument.

## The generator that stamped a retired sentence back

The retired-claim sweep, run over this round's nouns, found the ledger header's own status
definition reading `writer-side — addresses a PDF writer; we do not create files`. The
hundred-and-thirty-seventh session amended exactly that sentence — `CLAUDE.md` excludes
*authoring*, not writing — and `doc/ledger-and-claims.md` has recorded ever since that the
header "now carries that definition rather than 'we do not create files'". It did not, and the
mechanism is worth more than the correction: **the header is generated, its vocabulary lives in
`tools/conformance` — `bin/ledger.rs`'s `PREAMBLE` and `ledger.rs`'s `Status` docs — and nobody
amended the generator**, so the next regeneration stamped the retired sentence back over the
corrected file. A correction to generated text is not a correction until it reaches the
template. The same file held a second standing claim `CLAUDE.md` had already disproved:
`Exclusion::Xfa`'s "deprecated by ISO 32000-2 itself and specified outside it", where Annex K is
normative and inside the standard. Both are corrected in the template now, and the header was
regenerated from it.

## The blame band: §12.8's four freed rows and the eight from commit 185

Twelve rows read, oldest first — §12.8.2.1, §12.8.4.1, §12.8.4.5, §12.8.5.3 (freed by the
signature round landing), then §12.5.6.23, §7.10.2, §8.6.4.4, §7.11.3, §12.7.8.3.4,
§12.7.8.3.1, §11.7.4.4 and §9.6. **Two were wrong, and reading §9.6 found two more beside it**:

- **§9.6, §9.6.5 and §9.6.5.1 all said a font naming `MacExpertEncoding` "is refused and
  reported"** — three rows, one mechanism, all false since the four-hundred-and-fifty-first
  session transcribed Table D.4's 165 assignments (ADR 0286) and corrected §D and §D.4 while the
  three §9.6 rows one clause family over kept the refusal. The fifth failure shape's third visit
  to the font rows. §9.6's `partial` rested on that sentence alone ("the one thing actually
  left"), and §9.6.5 and §9.6.5.1 named no other absence: all three are `implemented`, with
  `name_keyed.rs::an_encoding_name_the_table_does_not_permit_is_no_encoding_at_all` added as
  evidence — the test that asserts every name Table 112 permits is a name the crate has a table
  for.
- **§12.5.6.23** excluded redaction's second phase with "`CLAUDE.md` excludes writing files" —
  the exclusion's wording from before the amendment. The conclusion stands on the amended
  exclusion's own terms, which the row now argues: this program writes §7.5.6's incremental
  updates, which append and keep the producer's bytes byte for byte, and a phase whose verb is
  *destroy* is the one edit that construction cannot express.

The ten kept rows each record the evidence that kept them — the grep run, the function checked,
the neighbour row consulted — which is what moves the blame pointer without a stamp.

## Consequences

- Five of the fifteen sweeps are committed programs; ten remain descriptions, one per sweep
  round until the backlog is gone.
- The table-number sweep found one more of the four-hundred-and-eighty-ninth's block shape:
  `spec_annotation_census.rs` attributed `/MarkInfo` to Table 353 — the catalog's Table 29 entry
  given to the table its value points at. Corrected.
- The ledger moves three rows to `implemented`; `silent` stays zero and every gate is green.
- The evidence-append script misfired on its first run — a doubled backslash in a regex made
  five sentences land at the end of the *next* backslash-free note — and was caught by grepping
  the file rather than trusting the exit status, which is `doc/todo/02` §6's rule paying again.
  The strays were removed and the corrected script verified each landing row.
