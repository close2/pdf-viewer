# 951 — Six targets, and a table for the refusals that are not debts

Date: 2026-09-10. ADR: 0951.
Files: `crates/pdf-transform/src/archive/` (six files, from one of 3 523 lines),
`tests/archive_corpus.rs`, `doc/pdf-a-conversion-limits.md` §4.3, §5.1, §9.4.

The converter runs against all six targets, and the split that made room for the rest of the work
is proved by its tests: 1 806 before, 1 806 immediately after, 1 812 with the slice's own six.

**One structural change came out of the split rather than being tidying.** `decide`'s special case
for the identification schema became `Prepared::obstacle`, asked of every mechanical rewrite — and
three of this slice's rewrites needed exactly that gate. A special case that turns out to be one
instance of a general question is what a refactor is for.

**What each level adds.** 2u derives a `/ToUnicode` by ISO 32000-2 §9.10.2's *second* method only,
over the codes the content streams actually showed, keeping the producer's usable values and
deriving only placeholders and absences — and **a font with one underivable code produces no CMap
at all**, because a partial `/ToUnicode` is a claim about the codes it does not cover. 2a writes
`/MarkInfo` `/Marked true` only where the catalog already states a `/StructTreeRoot`; the tree is
never invented, which is §5.1 and the owner's correction of it. 4f and 4e get the associated-file
names and a relationship of `/Unspecified`, which is Table 43's own stated default and therefore
asserts nothing.

**4e needs no new rewrite, and that is a finding rather than an omission**: Annex B is nearly all
relaxation, and its one file-binding addition is clause-13 artwork, which this project excludes.

**A third table, `REFUSED_BY_NAME`, because "a later slice owes this" was false for most of what
sat under it.** Ten rows separated into three kinds — an edit the fence forbids, a requirement
that is not this target's, and the genuinely not-yet-built — with `refused_by_name()` tested
against `pdf_archive`'s own identifiers, so a typo cannot silently demote an argument into a debt.

**An honest limit on 2u that will not move.** All eight veraPDF 2u failures are underivable by
construction: the clause's second exemption excuses the fonts whose glyph names are listed, so a
font that fails has names that are not. The corpus number cannot improve, and three hand-built
fixtures with a real embedded font program are the witnesses instead.
