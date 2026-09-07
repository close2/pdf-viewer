# Q46 — Is PDF/A on, and does the validator come first?

Source: `doc/rfc/0006` §10 question 1, unanswered; and `doc/questions/A17`, which put the whole
feature **on hold for now**.
Status: **open** — answered when `A46-the-shape-and-the-go-ahead-for-pdf-a.md` exists beside this file.

## Why it needs the owner

Two things have changed since A17 and neither is on the record as a decision. The owner said on
2026-09-07 that they intend to build a PDF-to-PDF/A converter, which reverses "on hold" in intent;
and RFC 0006 is still **Status: draft**, never ratified, unlike RFC 0002 and 0003 which the owner
approved by name. Every other question below is downstream of this one.

The second half is the shape. RFC 0006 recommends **validator first, converter after a
measurement**, and §6 of that document argues it at length. Nothing since has weakened the
argument, and one thing has strengthened it: the validator needs **no bundled resource at all**,
so it is the half that can be started before Q18 and Q47 are answered.

## What the tree does meanwhile

Nothing built. Parts 2 and 4 are readable in `doc/pdfa/`;
`doc/pdf-a-conversion-limits.md` is the limitation catalogue written from them.

## The third part: one part at a time, or both at once?

Raised by the owner on 2026-09-07 — *isn't it cheaper to validate all the levels at the same
time?* — and the measurement says yes. `python3 tools/pdfa-text.py --overlap` counts the
`shall` sentences of each part's clause 6 and how many of them are the same rule; run it rather
than believing a number written here.

Two things the count and the clauses settle between them:

- **Levels within a part are nearly free.** ISO 19005-2 §6.2.11.7.1 and §6.7.1 state their own
  applicability — Level B may ignore the Unicode subclause, Level B and U may ignore logical
  structure — so 2b, 2u and 2a are **one implementation with an applicability column**, not three.
  ISO 19005-4's Annexes A and B are the same shape: modifications to clause 6 that mostly
  *relax* it, so 4, 4f and 4e are one implementation and two flags.
- **Parts 2 and 4 share most of their predicates.** The union is far smaller than the sum, and the
  rules that differ are the ones already catalogued in `doc/pdf-a-conversion-limits.md`. Building
  part 4 alone and part 2 later would mean re-reading part 2's clause 6 to rediscover which
  requirements differ — work already done once.

Two precisions on "all levels", though:

- It can only mean **parts 2 and 4 and their flavours**. Parts 1 and 3 are not owned, and
  principle 5 does not permit implementing a requirement from somebody else's reading (A16, A17).
- **Building broad and certifying broad are different acts.** A validator that claims six targets
  while some of their requirements are unimplemented is worse than one that claims one — unless
  Q20's discipline holds, which is exactly what it is for: every target's report names which of
  its requirements were checked. **Q20 is therefore a prerequisite of the wide build, not a
  detail beside it.**

## Recommendation

**Yes, validator first; and one requirement table covering parts 2 and 4 with a per-part,
per-level applicability column** — which is RFC 0006 §7.1's own profile-data design, asked to
carry both parts from the first row rather than one.

What stays sequenced is *completion*, not code: **finish and certify PDF/A-4 first.** Its base
document is ISO 32000-2, the edition this tree owns and cites, where part 2's is ISO 32000-1:2008
which it does not (Q49) — so part 2's report will carry a "read from the wrong edition" note that
part 4's will not. And PDF/A-4 is what the owner's own archive requires. This amends RFC 0006 §11,
which proposed part 2 level b first and part 3 straight after, on facts that did not exist when it
was written.
