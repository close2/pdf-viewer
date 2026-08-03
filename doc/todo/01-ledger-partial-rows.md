# Read the ledger's `partial` rows against the code

Status: **standing task.** ~185 of the 239 rows have not been re-read.
Priority: 01 — the population with no gate, and it has paid on every session that touched it
Code: `doc/conformance/ledger.toml`, checked by `cargo test -p conformance`

## Why

All 823 subclauses of the eight technical clauses have been read against this code since the
fifty-sixth session, and the statuses are gated: `silent` is **zero**, `REVIEW_OWED` is empty and
fails the build the moment a cited-but-unread clause appears, and `FILE_ONLY_EVIDENCE_CEILING` is
zero and asserted with `==`.

What no gate can watch is a **note that has gone stale**, and the 239 `partial` rows are where
those live. Five failure shapes, in the order they were found:

1. A note that *understates* what the code does (five in session 115).
2. A note whose **reason** has expired — "while §X does not exist", "needs §Y" (117, 118).
3. A note claiming an entry is *unread* where the tree reads it (three in 122, five more in 159).
4. A note whose "what IS done" half is wrong — **the class that resists a grep**, because the
   name being present is what a grep looks for.
5. A note that is *stale about its neighbour*: §7.7.2 listed eighteen catalog entries as unread
   that were read, most of them by the session that built their clause. **A family's parent row
   is not maintained by the sessions that implement its members**, because the clauses do not
   cite each other. Three instances so far (§12.3's parent, §14.8.5.1's, §7.7.2's).

## The three sweeps

Twenty lines of Python apiece, each of which paid on its first run. Run all three after any round
that adds a verb.

| sweep | looks for | first catch |
|---|---|---|
| expired blocker | `while §X does not exist`, `needs §Y`, `until §Z` | session 118; found §9.7.5.2's "a licensing decision" 150 sessions after the decision |
| entry claimed unread | every `/Key` in a "Not read:" list, grepped against the tree | six of ten lists had a live entry; §7.7.3.3's had eleven of eighteen |
| capability | `this program has no ___`, `no panel`, `which this is not` | §12.6.3's "this crate has no events", 41 sessions after `Command::Pointer` |

Three false-positive shapes on the second, all seen: a note *quoting* its own retired wording
(§9.6), a key named in a sentence about something else (§12.7.5.3), and a key that is a string in
an unrelated list (`/Metadata` in `thumbnail.rs`). Read the hit before believing it.

## The shape the sweeps found last, and it is the strongest

The hundred-and-ninety-first session ran all three. §12.8.6 said a usage-rights signature grants
"features of a PDF processor that are not available by default" and that **"this program has no
feature behind such a gate"**; §12.8.2.3 said the same. Both were true when written and both
stopped being true in the hundred-and-thirty-fifth and -sixth sessions, when this program learned
to fill in a field and save a file — which are exactly the rights Table 258 grants and exactly
the changes Table 257's `/P` restricts.

And the requirement was not new. §12.8.2.2.1 has always carried, in a parenthesis:

> (These changes to the document shall also be prevented if the signature dictionary is referred
> from the DocMDP entry in the permissions dictionary.)

A `shall`, addressed to a processor that modifies, unread for fifty-six sessions after this one
became one. `ViewState::set_field` obeys it now.

**So: after a round that gives the program a verb, re-read the rows whose reason is about what
the program *is*, not only the ones about what a clause needs.** The same shape as §7.6.3.2's
random initialisation vector, which sat in an `implemented` row for a hundred and twenty sessions
because a reader only ever *reads* one (ADR 0129).

## What is still owed, named

- **§12.8.2.3's `should`**, opened by the session above and not closed: "A PDF processor that
  modifies a PDF, with a UR signature in excess of the rights that are granted by that signature,
  should remove that signature prior to writing the newly modified PDF." `pdf_syntax::write`
  appends §7.5.6's incremental update and removes nothing.
- **~185 `partial` rows** not yet re-read against the code.
- **The 33 `reported` rows are worked out** — all read in the hundred-and-twenty-first and
  -second, and none is of the two known failure classes (a true observation about the wrong half
  of a sentence, ADR 0109; a clause with two populations where the row names one, ADR 0110). 17
  are cryptographic validation needing a trust store, 5 need a second file or a network, 3 are
  icon clauses whose own verb is *should*, and the rest name a device or a user control this
  program does not have.
