# 1535 — An `inapplicable` row quotes its condition, and a clause stating no requirement is a heading

Session 1350. Status: **accepted**. Amends ADR 1461's class (a).
Context: `CLAUDE.md` principle 5 ("an entry here that says a clause does not apply is a claim about
the specification, and it decays"; "Quotation marks mean verbatim"); `doc/PLAN.md` §5a's
`implemented` and `inapplicable`; ADRs 0205, 1035, 1437, 1461; ISO 32000-2 §3.15, §10.1, §10.6,
§10.8, §14.8.3, §14.8.5.4, §14.10, §14.11, §14.12.

## 1. The instrument

`conformance::ledger::grounding` asks of each `inapplicable` row whether its note quotes, in double
quotation marks and at least `quote::MIN_WORDS` words long, a sentence that `ClauseIndex::holds_quotation`
finds in the row's own clause, in its parent below the top level, or in a clause the note cites
with a `§`. The last is how a condition stated once is quoted where it applies: §3.15's definition
of *deprecated* under each web-capture row, §10.6.1's exemption under §10.6.2 to §10.6.4. Before
this, a ledger note's quotations were checked by nothing that gates — `--bin quotations` is a
reading list — so an `inapplicable` row's condition was prose that no run re-read.

A row that fails is `Problem::ConditionUnquoted`, named with its `Grounding` (`Unverified`: it
quotes nothing those clauses hold; `ProseOnly`: it quotes nothing long enough). The gate admits them
up to `CONDITION_UNQUOTED_CEILING` with `==`, so the count can only fall; it is 0. `--bin ledger`
prints the three counts, and the `out-of-scope` rows that name their exclusion beside their total.

What it cannot see is whether the quoted sentence is the condition rather than another sentence of
the same clause. That stays a reading; the quotation is what lets the reading be repeated.

## 2. Before and after

Of 48 `inapplicable` rows, 15 carried a verified quotation, 31 prose only, and 2 a quotation their
clauses do not hold: §14.11.1's, which reworded its clause's "features of PDF that support
prepress production workflows", and §14.10.5.4's, a paraphrase in quotation marks. Of the 15,
six quoted a sentence that is not a condition (a version number, a NOTE, a listing order). After: 34 rows, every one quoting its condition.
The 94 `out-of-scope` rows all name an exclusion; nothing to do there.

## 3. Fourteen rows were not `inapplicable`

- **A clause that states no requirement is a heading, not a condition.** §8.4.3.1, §8.6.6.1,
  §11.6.1, §11.7.1, §12.7.1, §14.8.3.1, §14.11.1 and §14.12.1 have no `shall` and introduce
  subclauses; §5a's `implemented` holds vacuously and this ledger's convention already says so for
  §6.1, §8.6.1, §10.4.1 and §11.1. Each now names its family's code and test and quotes its own first
  sentence. A clause inside a deprecated family (§14.10.4.1) stays `inapplicable` on §3.15.
- **A heading takes its subclauses' status.** §10.6, §10.8, §14.8.3 and §14.11.6 have no text of
  their own and at least one subclause `implemented`; "the requirement cannot reach this program" is
  false of the family, so each is `implemented` as §14.11 and §14.8.5.4 already were. §10.6's note
  says first that the screen is `inapplicable` in §10.6.1 to §10.6.4.
- **A permission exercised.** §10.8.1: "Whether separations are produced is up to the processing
  software", and `pdf_render::separation` produces one plane per colourant under §10.8.3's
  simulation. The old reason, that this program produces only a screen, was false.
- **A condition false of the tree (ADR 1461's (d)).** §14.8.5.4.2 said none of Table 378's
  attributes is read. `Tree::writing_mode` reads `/WritingMode` through `Tree::inherited_attribute`,
  as "[t]he specified layout directions shall apply to the given structure element and all of its
  descendants" requires, and §14.8.5.4.5's table equalisation uses it, as "[f]or tables, the writing
  mode controls the layout of rows and columns" says. `implemented`, the other seven on §14.8.5.4.1's
  condition. The same reading found §14.8.5.4.3 calling `/SpaceBefore` and `/SpaceAfter`
  unexecuted while `Tree::allocation` reads them; that note is corrected.

## 4. Consequences

A new `inapplicable` row arrives with its sentence or fails the build. A later round that re-reads
one asks first whether it is a heading (no `shall`, or no text of its own) and whether the clause's
permission is exercised somewhere, before asking whether its condition holds.
