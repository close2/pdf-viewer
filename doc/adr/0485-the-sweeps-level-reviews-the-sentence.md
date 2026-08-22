# ADR 0485 — A sweep's level reviews the sentence a round is about to write

Status: accepted.
Session: the six-hundred-and-fifty-seventh, a clause round under `doc/todo/01`'s binding rule.

## 1. What this decides

1. **A round that edits `ledger.toml` runs the sweeps before *and* after its own edit, and accounts
   for every number that moved.** Not to find a defect in the tree — to find one in the sentence it
   just added.
2. **A number that moves without a reason the round can state is an edit to reconsider**, and the
   first thing to reconsider is whether the sentence duplicates something a neighbouring row already
   says.
3. This generalises the six-hundred-and-fifty-second's finding from a sweep's *vocabulary* to its
   *level*, and it is cheaper than the thing it replaces, which is a reviewer.

## 2. Where it comes from

652 found that `--bin overstated` went from 8 contradictions to 12 while it was drafting a
correction, because the draft wrote a denial in words the sweep scores as an assertion. Its lesson
was about the idiom: write a denial in the ledger's own `unread` vocabulary. That is true and it is
narrower than what happened. What actually caught the draft was not the vocabulary — it was the
**count changing for a reason the round had not intended**.

This round hit the same instrument from the other side. The correction to §8.11.4.1 named Table 98's
`/Configs` and a configuration's `/Name` and `/Creator` as the entries §8.11.4.3 owes. Every word of
that was true. `--bin unread` went from its standing **69 rows / 182 keys** to **70 / 185**, and the
three new keys were the three §8.11.4.3's row already carries — a `/Configs` whose only witness is
`examples/oc_usage_census`, a `/Name` that collides with an optional content *group*'s, a `/Creator`
that is `metadata.rs`'s §14.3.3 entry. The sweep's dominant noise shape, doubled, by a sentence
written to correct a row.

The edit was rewritten to point at §8.11.4.3's row rather than repeat its list, and the sweep
returned to 69 / 182. Nothing about the correction's *content* changed.

## 3. Why the level and not the hits

A sweep's hit list is read by the round that runs it, and a hit is explicitly "a reading list rather
than a verdict" in every one of these programs. That framing is right and it has a cost: a round
that reads a hit, judges it noise and moves on has no way to notice that the *number of hits it just
created* is its own.

The level does not have that problem. It is one integer per sweep, it is stable across rounds that
change nothing, and it is produced by a program that does not know what this round is trying to say.
Three properties follow:

- **It is a review with no reviewer.** The round states what it expects each number to do before it
  edits; a number that does something else is a question.
- **It cannot be argued with.** A round can talk itself past a hit. It cannot talk `unread` from 185
  keys back to 182.
- **It is symmetric.** 652's failure raised a count by widening a vocabulary's match; this round's
  raised one by duplicating a true fact. Neither is a defect a reader would see and both are visible
  as a moved integer.

## 4. What a round owes

After editing `ledger.toml`, and before committing:

```sh
for b in overstated counts quotations tables unread owed blockers capabilities \
         entries inapplicable pointers callers; do
    cargo run -q --release -p conformance --bin "$b" 2>/dev/null | tail -2
done
```

against the same list taken before the edit, and one sentence per moved number in the round's own
history file. Nothing new is built: the twelve already run every round the ledger moves, and
`doc/todo/02` §4 already says so. What is new is that the run *before* the edit is not optional and
the deltas are accounted for rather than reported.

This round's, as the worked example. The reading is taken with only `ledger.toml` edited, which
matters: several of these sweeps also read `doc/`, so a round's own ADR and history file move them
afterwards for reasons that are not about the ledger at all.

| sweep | before → after | why |
|---|---|---|
| `overstated` | 125 → 127 terms asserted, 52 → 54 corroborated, **8 contradicted, unchanged** | §11.7 and §9.7 now name a child's entry where they named none, and both are corroborated |
| `counts` | 359 → 360 attributed counts, **4 places counting one family twice, unchanged** | §9.7's debt is stated as **one** clause where it was stated as two |
| `quotations` (ledger) | 1727 → 1733 quotations, 1311 → 1313 verbatim, **1 diverging, unchanged** | §10.7 quotes §10.7.3's "internal limits" sentence and §8.11.4.4's "otherwise OFF" |
| `unread` | **69 rows / 182 keys, unchanged** | *after* the rewrite in §2; the first draft made it 70 / 185, and that is the finding |
| `entries` | 276 → 277 rows explaining themselves by an arrival | §10.7 now dates `/SM`'s reader |
| `owed` | 3399 → 3414 terms stated, **174 named by no source over 111 rows, unchanged** | four longer notes, none of which names a new debt |
| `pointers` | 6902 → 6903 paths, **118 absent and 13 undefined symbols, both unchanged** | one prose mention of `Ramp::resolution_for`; the four files added to `code` and `test` all exist |
| `tables`, `blockers`, `capabilities`, `inapplicable`, `callers` | unchanged | nothing the ledger gained names a new table, blocker or capability |

The row that matters is the fourth: a sweep whose level *did not* move, because the round noticed
that it had.

## 5. What this is not

It is not a gate. The levels are not thresholds, several of them move for entirely good reasons
every round, and pinning them would turn twelve reading lists into twelve ratchets somebody has to
re-baseline — which `doc/habits.md` already records going wrong once, in the text floor that stayed
red for ten sessions after a round improved it.

It is a habit, and it belongs where the other reading habits are: `doc/todo/01`, beside the sweeps
themselves.

## 6. Consequences

- `doc/todo/01` gains one bullet naming this and pointing here.
- A round that finds a moved number it cannot explain has found either a defect in its own sentence
  or a defect in a sweep, and both are worth the minute.
- The cost is one extra run of twelve programs that take, between them, a few seconds — measured
  this round rather than assumed: `overstated` alone is a fifth of a second and opens no source
  file.
