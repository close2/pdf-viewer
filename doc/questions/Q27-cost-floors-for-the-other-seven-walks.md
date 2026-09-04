# Q27 — Which of the other seven corpus walks should get a cost floor, and does a walk owe one?

Asked by round 927, which built the first one. `Q26` was taken by round 924; rounds 925 and 926 ran
beside this one and may have taken numbers after it.

## The question

This project gates correctness thoroughly and cost almost not at all, and it has now been paid for
once: **a hundredfold cost regression lived in `pdf-vfs` for four sessions with the entire gate
sequence green** (ADR 0886). Round 927 gave that crate's two walks a floor — a *count* of how often
a generator runs, held to what the cache forgot, so that no clock and no band is involved (ADR
0894). It found a second defect of the same shape on its first run (ADR 0895 §1).

Seven other corpus-scale gates have no cost floor at all, and ADR 0895 §3 costs each of them. Two
things need the owner:

1. **Which of the seven is worth a round**, and in what order. They are not equal: three need no
   library change, one needs a single counter in `pdf_syntax::Document::open_with_password` that
   would serve seven gates at once, and one (the GPU device) needs a counter inside a backend.
2. **Does a corpus walk *owe* a cost floor the way it owes a population?** That is a rule for
   `doc/todo/02` §2, not a technical matter: if the answer is yes, then a round adding a walk adds
   a counted floor with it, and the seven become a debt with a todo file rather than a list in an
   ADR.

## Why this cannot be settled without the owner

The first half is a **priority** question, and priorities are the owner's — each row of ADR 0895
§3's table is a round's worth of work with a defect to check it against, and this round has no
standing to spend seven rounds of the project's time on its own finding.

The second half amends how every round works. `doc/todo/02` §2 is the file that says what a round
does, and a new standing obligation in it is the same kind of change as the memory rule or the
one-walk-at-a-time rule — the owner has stated each of those personally.

There is also a real argument *against* both, and it should be on the record: every gate costs
wall clock in a sequence that is already long, and a floor that is only a ratchet against a defect
nobody has hit is a tax. The counter-argument is the one this round exists because of — the defect
had already been hit, twice, and neither time did a gate see it.

## What the tree does meanwhile

- **`pdf-vfs` has the floor**, in `tests/a_face.rs` (every round, through
  `cargo nextest run --workspace`) and in `tests/read_corpus.rs` and `tests/write_corpus.rs` (when
  the crate changes, and every fifth round). ADR 0894.
- **The other seven print a wall clock nothing compares to anything**, exactly as before. Nothing
  regressed and nothing is blocked.
- ADR 0895 §3 records the survey so that whichever answer comes back, the next round does not have
  to re-derive it — though `doc/habits.md`'s *Measuring* section says a price decays, so a round
  taking one of these re-derives it rather than believing the table.

## Recommendation

1. **Take `accessibility_census` first** — one interpretation per page visited, where each page is
   asked two separate queries that both derive from it. It is the sharpest invariant of the seven
   and the census already computes the right-hand side.
2. **Then the `Document::open` counter**, because it is one function and it lets seven gates state
   a relation none of them can state today.
3. **Replace `pdf-transform`'s 40-pages-a-second floor with the font-cache count rather than
   keeping both.** That gate is the one with a recorded false failure (33.3 against 40 where the
   same tree quiet reads 198.3), and the property it is proxying — one shared font cache — is
   exactly countable.
4. **Yes to the standing obligation, worded as a default rather than a rule**: a walk that can name
   a counted cost property carries one, and a walk that cannot says why in its module comment. That
   keeps the tax proportional to what is actually countable.
