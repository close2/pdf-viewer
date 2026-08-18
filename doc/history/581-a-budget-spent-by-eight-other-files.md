# 581 — The survey that answered differently twice, and the bound that decided it

Date: 2026-08-18. ADR: [0416](../adr/0416-a-budget-spent-by-eight-other-files.md).

Touched `crates/pdf-model/src/colour.rs`, `content.rs`, `content/report.rs`,
`content/transparency.rs`, `action.rs`, `crates/pdf-model/examples/press_census.rs`,
`tools/safedocs/src/survey.rs` and `main.rs`, `doc/conformance/ledger.toml`,
`doc/todo/03-more-corpora.md`, `doc/todo/49-restrictions-worth-re-examining.md` and
`doc/todo/README.md`.

## The demand half — the instrument, not a population

Session 580 reported that re-running `tools/safedocs survey` moved about ten documents in and out
of §11.4.7's report, and left it. Reproduced on the 287 crawled documents whose page-one press is
an ICC profile this tree evaluates: three quiet runs of one unchanged binary printed **30, 36 and
33** press refusals, and a fourth under twelve spinning cores printed 35. So it is
**nondeterminism** — quiet runs already disagree — and load only changes the interleaving.

Attributed by removing the suspect, which is `doc/habits.md`'s rule: with `colour::MAX_PRESSES`
raised to 256 in a scratch build, two runs were **byte-identical** and printed 19 incomplete and no
press refusal at all. The whole flipping population is that one bound — a `static` table of eight
sampled blending spaces, filled from the front, never evicted, so the ninth *distinct* press a
process meets is refused and which document that falls on is rayon's answer.

What changed: the refusal now says whose reason it is (`BeyondPress { why, this_process }`), a
file-stated reason is reported ahead of the process's, `Interpretation` carries the two counts
beside the report, and the survey marks each such document and prints the file-decided count. Three
runs now print **19, 19, 19** — the same figure the bound-removed build gave.

## The population, re-established with something that shares nothing

`examples/press_census` over all 65 703 crawled documents that open, one process per archive, run
twice and byte-identical over all 145: **2296 state §11.4.7's condition** (3.49%), **287** name
their press through a four-component ICC profile, and those name **28 distinct presses** against a
table of eight. The 974 name **0**, which is why no gate ever moved for this — checked by running
the corpus gate twice, every count identical.

## The spec half — §12.6.4.3, a `reported` row

`GoToR` is refused for want of a filesystem, and `action.rs` said that about *this program*. Every
host has written files since ADR 0244. The refusal stands; its reason is now the smaller and
nameable one — a second `Document` in the vocabulary and a host's decision about which files a
document may name.

## What is owed

The bound itself. `doc/todo/49`'s new third-bound section prices three roads and rejects two of
them; the one that works makes the press budget per-interpretation, which is what every other
budget in this tree already is.
