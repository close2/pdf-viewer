# ADR 0788 — A row ranked against its siblings: §7.6.4.4 is `implemented`

Status: accepted. Session 864.
Clauses: ISO 32000-2 §7.6.4.4, §7.6.4.4.2, and the family's other eleven rows.
Code: `crates/pdf-syntax/src/crypt.rs` (unchanged).

## The question

`doc/todo/01`'s blame list has been led by §7.6.4.4 and §11.3.4 since the seven-hundred-and-first
session, and the rule ADR 0455's successors added is to read the **family** rather than the rank.
This is that reading of §7.6.4.4 — Algorithms 3 to 13, thirteen rows — and its finding is not
about the standard or about the code. It is about the row beside the row.

## What the family says

Three of the eleven algorithms are neither wholly the writer's nor wholly the reader's, because a
reader's authentication re-runs part of what a writer stored:

| row | algorithm | what a reader runs | what is left | status before |
|---|---|---|---|---|
| §7.6.4.4.2 | 3, the `/O` entry | steps (a) to (d), through §7.6.4.4.6's Algorithm 7 | (e) to (h) | **`partial`** |
| §7.6.4.4.3 | 4, the `/U` entry at R 2 | steps (a) and (b), through Algorithm 6 | (c) | `implemented` |
| §7.6.4.4.4 | 5, the `/U` entry at R 3 or 4 | steps (a) to (e), through Algorithm 6 | (f) | `implemented` |

One shape, two answers. And the difference was not argued: §7.6.4.4's own note gave as the reason
for the odd one out that "Algorithm 7 begins with Algorithm 3's steps (a) to (d), **which is why
§7.6.4.4.2 is `partial` rather than `writer-side`**" — a choice between two statuses that leaves
out the one its two siblings hold.

## The decision

**§7.6.4.4.2 is `implemented`, and §7.6.4.4 with it.**

Of Algorithm 3's four unexecuted steps, the two that *compute* are executed inverted one clause
along: §7.6.4.4.6's Algorithm 7 step (b) undoes step (f)'s and step (g)'s RC4 invocations, "from
19 to 0" against Algorithm 3's 1 to 19, and `crypt.rs`'s `unwrap_owner_entry` is where. What is
left is step (e), which pads the *user* password that a reader recovers rather than supplies, and
step (h) — "Store the output from the final invocation of the RC4 function as the value of the O
entry in the encryption dictionary" — which is a generator's requirement and therefore inside
`CLAUDE.md`'s closed exclusion list, exactly as §7.6.4.4.3's step (c) and §7.6.4.4.4's step (f)
are.

The parent then has no unsettled child. Every one of its twelve rows is `implemented` or
`writer-side`, and a `partial` aggregate over settled children is the ledger's own arithmetic
sweep firing on itself.

## Why this is not status inflation

The ledger's `partial` is for a row that "says which requirements are unexecuted, which are not,
and what is reported". §7.6.4.4.2's note named no requirement a *reader* owes — it named the
writer's steps and stopped, which is `doc/todo/01`'s fourteenth sweep's whole subject: a `partial`
row with no debt in it. Moving it does not claim more of the standard than the code does; it stops
claiming a debt the code does not have, in a place where the project's own blame list is ordered
by how long a debt has stood.

The alternative — moving §7.6.4.4.3 and §7.6.4.4.4 *down* to `partial` — was weighed and refused.
It would put every clause with a writer's step in it into permanent debt, which is the opposite of
what the exclusion list decides, and it would leave the ledger unable to distinguish a reader's
gap from a producer's requirement anywhere in clause 7.6.

## What the reading did not move

Nothing in `crypt.rs`. The four steps were read against the code again — the fifty whole-digest
re-hashes of step (c), the revision-keyed key length of step (d), the counter running 19 down to 0
in Algorithm 7 against 1 up to 19 in Algorithm 3 — and each is what the clause states.
`an_owner_entry_unwraps_to_the_padded_user_password` builds an `/O` by running steps (e) to (h)
forward at revisions 2, 3 and 4 and asserts this reader unwraps it, which is still the only thing
in the tree that reaches `unwrap_owner_entry`.

## Consequences

- `doc/todo/01`'s blame list loses its rank 1 and rank 2. §11.3.4 is what is left at the top, and
  its debt is named and real: the one-component blending spaces and `ICCBased` 'CMYK', both
  reported by name.
- The rule the family gave ADR 0455's ordering is worth restating, because this round is its
  second instance after ADR 0560's §14.6: **where the top of the blame list holds several rows of
  one clause family, the comparison that finds something is the row against its siblings.** An
  ordering by age cannot produce it, and neither can an ordering by what the clause says.
