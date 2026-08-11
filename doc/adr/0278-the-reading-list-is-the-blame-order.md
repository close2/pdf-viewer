# ADR 0278 — The reading list is the blame order, and two rows it turned into decisions

Date: 2026-08-11 (session 442)
Status: accepted

## Context

`doc/todo/01`'s headline is *"Read the ledger's `partial` rows against the code"*, and the
fourteen sweeps that grew out of it have been run fifteen times while the reading itself keeps
being deferred. The reason is not laziness: the sweeps produce a *list*, and the reading does
not. Session 437 could say "~80 of 248 rows have not been re-read" and could not say **which**,
so every round that wanted to do the headline job began by choosing rows, which is the part a
round is worst at — the rows that look interesting are the rows recent work has touched, and
those are exactly the rows recent work has maintained.

The population this file exists for is the opposite one: a note that is quietly no longer true,
in a clause nothing has had cause to open. No sweep here can find it, because every sweep reads
a row's *reason* — a blocker, a capability, a retired string, an unread key — and a note whose
reason simply expired states none of those.

## Decision

**Order the `partial` rows by the commit that last wrote each row's `note = ` line, and read
from the top.** Twenty lines over `git blame --line-porcelain doc/conformance/ledger.toml`:
attribute each line to its commit, group the lines into rows, take the row's note line, and
index its commit into `git log --reverse`.

The instrument rests on a convention this project already keeps rather than on anything new.
Every re-reading session in `doc/todo/01` records itself **in the note it re-read** — "this row
said X until session N", the shape the file's sixth failure mode taught it to write — so a note
nobody has rewritten is a note nobody has re-read *and corrected*. That is weaker than "nobody
has read it": a row read and found right leaves no trace. It is still the strongest signal
available, and unlike the running estimate it is checkable, per row, by anybody.

On the tree at `b45b77b`: 590 commits, 248 `partial` rows, and **40 of them with notes last
written before commit 110**. This session read 32 of the 40 and **fourteen were wrong**, with a
fifteenth found beside them.

## What the order is *not*

It is not a priority ranking. A row's age says nothing about whether the clause matters, and the
oldest rows in this ledger are old partly because the early sessions wrote good notes about
small clauses. What the order buys is that it is **not chosen by the reader**, which is the
whole complaint against the previous method.

It is also not a gate, for the reason ADR 0249 gives about the eleventh sweep: the output is a
reading list whose entries need a person, and turning "this note is old" into a build failure
would make the project's own record-keeping convention load-bearing on CI.

## The two rows that turned into decisions

Reading produced two questions the standard does not answer, and both are recorded as choices
rather than as findings.

### 1. Table 20's `/EFF`, and a reason that expired under a verb

§7.6.6's row was `partial` because "the /EFF path itself is unexercised — no embedded file's
bytes are ever read". True when written. False since `Command::Extract` and §7.11.4's
attachments panel: a host reads an embedded file's bytes now, and `Document::stream_method` was
choosing that stream's crypt filter with `/StmF`, so a document writing

```
/StmF /Identity /StrF /Identity /EFF /StdCF
```

handed its attachment back as ciphertext with no report at all — the failure trap 5 exists to
prevent, reached through a door the round that opened it had no reason to look at.

Table 20 states the rule and its fallback in one entry, and the reader's order falls out of the
entry's own words: the filter is for embedded file streams "that do not have their own crypt
filter specifier", so a `/Crypt` specifier is asked first; and "[i]f this entry is not present,
and the embedded file stream does not contain a crypt filter specifier, the stream shall be
encrypted using the default stream crypt filter specified by StmF", so its absence is `/StmF`
and *not* `Identity`. §7.6.6's neighbouring rule — "related files ( RF ) shall use the same
crypt filter as the embedded file ( EF )" — holds by construction, both being `/Type
/EmbeddedFile` streams reached by no other route.

**The decision is the fixture.** No document exists that exercises `/EFF` alone: the corpus's
two candidates state *both* routes to `StdCF`, so the reader's answer is identical whether or
not it reads the entry, and building a synthetic encrypted document would mean encrypting with
the algorithms under test and comparing this code with itself — which
`crates/pdf-syntax/tests/encryption.rs` opens by refusing to do. So the fixture is
`encrypted-attachment.pdf` **with one entry blanked out**: the `/Crypt` specifier replaced by
spaces, which §7.2.3 makes white space, leaving every byte offset where the cross-reference
table says it is. What is opened is a real producer's file minus one entry. Both halves of Table
20's sentence are then assertable — with `/EFF` the stream takes `StdCF` and §7.6.6 refuses it
for want of a key, and with `/EFF` blanked as well it falls back to `/Identity` and the bytes
pass through — and only the pair distinguishes *reading the entry* from *refusing every
attachment*.

### 2. `/Path` over `/InkList`, which Table 185 does not order

§12.5.6.13's row said PDF 2.0's `/Path` "supersedes it as the table requires". **Table 185
requires nothing of the kind.** It marks `/InkList` "(Required)" flatly and `/Path` "(Optional;
PDF 2.0)" beside it, with no sentence ordering the two. §12.5.6.9's Table 181 *does* say it, of
`/Vertices` — "(Required unless a Path key is present, in which case it shall be ignored)" — and
the row had borrowed the neighbour's rule along with the neighbour's code, which is one function
serving both clauses.

The tree's behaviour is not changed, because there is nothing to change it to that is better: a
file stating both entries describes one scribble twice, so drawing both marks the page twice and
drawing the older one throws away curves the newer can carry. **The choice stands and is now
labelled as one**, in `appearance::ink`'s doc comment, in the ledger row, and in the test that
holds it. Presenting a de-facto convention as though it were derived is the failure `CLAUDE.md`
principle 5 names, and this row had been doing it for four hundred sessions.

## Consequences

- `doc/todo/01` carries the blame order as its reading list, and the standing count is now two
  numbers rather than one: **~48 of 244 unread**, of which **8 have notes older than commit
  110** and 55 more fall between there and commit 300.
- **§11.6's note is the longest-lived stale claim this project has recorded: 424 sessions.** It
  said a graphics-state soft mask was reported and transparency groups were "the silence", and
  both were built in the two sessions *after* it was written — the seventeenth and the
  eighteenth — each recorded in its own row ever since. It beats §9.3's 365 (ADR 0273) and
  §12.5.6.19's 364.
- Clause 11's `partial` rows are clean, and the blame order says why they had not been: §11.3.7,
  §11.6 and §11.4.1 were the only three whose notes predated commit 90, and every other row in
  the clause has been rewritten in the last ninety commits by the rounds that built transparency
  groups, the knockout shape, the page group and the press. **A family under work is a family
  whose rows are maintained**; the rows that go stale are the ones nothing since has had cause to
  open, which is what an age-ordered list finds and a subject-ordered one cannot.
- Four rows moved to `implemented` — §8.9.3, §12.5.6.13, §14.11.2 and §14.11.2.1 — taking the
  ledger 406/248 → **410/244**.
