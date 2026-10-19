# 2031 — A count of the tree's own items is written as the rule that makes it, and no gate reads it by shape

Session 1597, slot 6 of batch ninety-four, under the project owner's "switch to code improvements …
until I say otherwise" of 2026-10-14. Status: **accepted**. Context: `CLAUDE.md` "Where knowledge
lives", ADR 1023, ADR 1547; `tools/conformance/tests/spelled_ordinals.rs`. Code:
`crates/pdf-model/src/content.rs` (`Interpreter::checkpoint`'s `#[expect]`). Cites no clause and
moves no ledger row.

## 1. What was found

`checkpoint`'s `#[expect(clippy::too_many_lines)]` gave as its reason "one line per field of an
interpreter that has fifty-nine of them". The interpreter has eighty (`struct Interpreter` counted
on this tree), so the reason had been wrong for every field added since it was written, and the
function went on compiling, because the exhaustive destructure it explains is the guard and the
count in the sentence beside it guards nothing. It is now the rule: one line per field, so a field
added is a line added.

## 2. Why `spelled_ordinals` did not see it, and why it is not taught to

That gate's population is a **round's** ordinal — `hundred-and-`, `Session <n>` — a string no
identifier, clause or noun of the standard contains, so every hit is a debt. A count is a different
class, and it has no shape of its own: the same spelled compounds are, in this tree, the standard's
own figures (Table 171's twenty-eight names, ICC.1's forty-five tags, sixty-five thousand code
points), constants with their derivations, and test fixtures — 336 lines under `crates`, `tools`
and `raster` today, of which a reader decides which are the tree counting itself. A gate that held
them at a ceiling would hold mostly the standard's facts, and one that tried to tell them apart
would be the reading list `CLAUDE.md`'s second grep already is.

So no gate is built for it. The rule is applied where the sentence is written: a reason or a
comment that would carry a count of this tree's own items — fields, arms, variants, files — states
the rule that produces the count instead, and where the count is load-bearing it is a test's
assertion, not prose (`CLAUDE.md`: what is written down is the command that counts it).
