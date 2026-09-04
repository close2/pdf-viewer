# 0872 — Two streams write into one clause, and the merge is where the count is wrong

Session 915. Status: **accepted**. The first of this round's two records, and the only one that is
about a *merge* rather than about the code either side of it: two long-lived branches — the
transform suite's and the file-system faces' — both wrote a page-label writer into §12.4.2's ledger
row, and the resolution is not the union of the two texts.

## Context

`round-867` carried session 910's `split --at-bookmarks` and the three document-level constructs a
piece now carries; `round-911` carried sessions 902, 906, 909 and 911, the whole of RFC 0003 from
the confined worker to a mount a shell has walked. Both landed on `main` in this round, in that
order.

`doc/todo/02` §2 states the rule the merge is under — "**A merge is a round of its own, and it runs
this sequence on `main`**" — and `CLAUDE.md` states the one this record is about: every clause a
change touches leaves its ledger row non-`unreviewed`. Sessions 905, 908 and 910 each read the
ledger **row by row** across a merge rather than off the diff, and each recorded that they found
nothing. This one found two things, which is the argument for keeping the habit.

Git's own verdict on the second merge was one conflict, in §12.4.2's row, and nothing else.

## Decision

### 1. Two appends to one row are not resolved by taking both appends

Session 906 added `pdf_transform::update` — the in-place page editor — as a §12.4.2 writer, and
session 910 added `split`. Each was a pure append: one entry on the row's `code` list, one on its
`test` list, one sentence at the end of its note. Neither side saw the other, because neither
branch had it.

The union compiles and reads well and is **wrong about a number**. Session 910's sentence opens
"[a] piece of a document is the **third** caller of that construction", counting `merge` (session
888) and `pages` (session 893) before it. On the merged tree there are four, because `update` is
one of them and arrived four sessions earlier. So the resolution is:

- both `code` entries and both `test` entries, in the order the writers were built;
- both sentences, in **session order** — 906's in-place edit before 910's piece, which is the order
  the rest of that note already runs in;
- and session 910's *third* becomes **fourth**.

The general shape is worth more than the instance: **a note that counts its own callers is a
cardinal about this tree**, which is the twenty-second sweep's subject (ADR 0709), and a merge is
exactly the event that invalidates one without either side being wrong when it was written. A
resolution that keeps both texts unchanged has produced a row that is false and that neither branch
could have made false on its own.

### 2. A clean auto-merge is not a read, and this one was carrying a wrong table number

Session 906's sentence cites "Table 159's `/P`". Table 159 is *Entries in a folder dictionary*
(§12.3.5's collection); the page label dictionary is **Table 161**, which the same row's own first
sentence names — "Table 161's `/S`, `/P` and `/St`". The same number is in
`crates/pdf-transform/src/update.rs`'s doc comment above `rebuild_labels`.

Both are corrected here. Neither would have been caught by anything else in this tree, and the
reason is worth stating because it is a hole rather than an oversight: `tools/conformance` verifies
that a cited table **exists** and prints its title, so `Table 159` reads exactly like a correct
citation — the failure that ADR 0475's `--bin tables` sweep exists for, which compares a cited
table's `/Key` citations against the entries ISO 32000-2 puts in that table. This citation names no
key of Table 159 beyond `/P`, and `/P` is a key of neither table's own naming that the sweep can
key on.

So it was found by reading the two sides of a conflict against each other, which is what a row-by-row
check *is*. **The habit is the finding**: sessions 905, 908 and 910 each did this and reported
nothing, and it was tempting to record that as evidence the check is ceremony. It is not — a check
that has not yet failed is a check whose population has not yet contained a defect, and this merge's
did.

### 3. What a row-by-row check is, stated so the next merge does not re-derive it

Three questions, and the third is the one a diff cannot answer:

1. **Which rows did each side move against the common ancestor?** Parse the three versions into
   rows keyed by `clause`, and compare sets. Here: `main` had moved §7.3.7, §9.6.4, §11.5.3 (from
   rounds 907, 908 and 912) and later thirteen more; the branches had moved five and seven.
2. **Is every merged row identical to the side that wrote it, and did no third row move?** That is
   a set comparison, and it is what catches a resolution that silently dropped an unrelated row.
3. **Is a row that both sides wrote still *true* of the merged tree?** Only a reading answers this,
   and §12.4.2's count is the case.

The row count is the cheap invariant beside them — 875 on all four versions of the ledger in both
merges — and `tomllib` parsing the result is cheaper still. Neither is a substitute for the third
question.

## Consequences

- §12.4.2 names five writers and six tests, and its note counts four callers of `merge::page_labels`.
- `crates/pdf-transform/src/update.rs` cites Table 161.
- `doc/todo/02` §2's merge paragraph is unchanged: it already says a merge owns the sequence for the
  merged result. What this record adds is that it owns the *ledger* for the merged result too, and
  that "git found no conflict" is a statement about text.
- The `--bin tables` sweep does not see a table number that is wrong in prose without a `/Key`
  beside it. That is a gap rather than a defect, and it is recorded here rather than fixed, because
  the fix — checking every `Table NNN` mention against the title the standard gives it — is a
  different sweep from the one that exists and belongs to a round that can measure its noise.
