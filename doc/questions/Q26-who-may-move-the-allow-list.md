# Q26 — Who may move the seccomp allow-list, and one clause of Q24's proposed wording that this round makes fragile

Asked by session 924, which took `doc/todo/61` §3's decision and admitted one command of one system
call to the confined interpreter. **Provisional, not a blocker**: the decision is taken, argued in
ADR 0888, tested in three directions and calibrated against the policy made wrong in each of them.
Nothing waits on this answer; what it would change is who takes the *next* such decision.

## The question, in two parts

### 1. Is moving the allow-list a round's decision or the owner's?

`doc/todo/61` says of exactly this case that "this item is the place to decide deliberately", and the
owner accepted that item on 2026-09-04. This round read that as a delegation and decided. It is the
**first movement of the allow-list since ADR 0812** (session 883, forty-one sessions), and the first
ever that is not a resource crossing the boundary — `recvmsg` and `pread64` were admitted so that a
document could arrive; this one is admitted so that a document can be *let go of*.

Two readings are available and this round cannot choose between them:

- **The item delegated it**, and a round that argues the trade, prices the alternatives and probes
  both directions has done what the item asks. That is what happened.
- **The sandbox is the owner's sentence** and `CLAUDE.md` principle 3 calls it non-negotiable, so the
  list is frozen except by the owner's word, whatever an item says — and `doc/todo/61` would then be
  a place to *prepare* the decision rather than to take it.

**A standing rule either way would be worth more than a ruling on this instance.** If it is the
owner's, the honest form is a line in principle 3 or in `doc/todo/61` saying so, and this round's
change stands or is reverted on the owner's word.

### 2. One clause of `Q24`'s proposed wording is now fragile

`Q24` (session 920) proposes amending principle 3's sandbox bullet, and its draft contains:

> The renderer's system-call set **does not change**, no host may change it, and the renderer can
> still name no path.

Read as it was meant — *offering a face does not change the set* — that is exactly true and stays
true. Read as a standing claim about the tree it is now falsifiable, because the set did change one
round later. The second and third clauses are unaffected and are the ones carrying the weight: **no
host may change it** is still true, and no host can; **the renderer can still name no path** is still
true, and `F_GETFD` names none.

If the owner takes `Q24`'s wording, the suggested repair is to bound the first clause to the port:

> **No port changes the renderer's system-call set**, no host may change it, and the renderer can
> still name no path.

## Why it cannot be settled without the owner

The boundary is the owner's own statement, and the failure `doc/todo/61` exists to prevent is a round
widening it for its own convenience. A round that both widens the list *and* rules that rounds may
widen the list has decided its own case. That is the whole of the reason; the technical argument is
in ADR 0888 and does not need the owner at all.

## What the tree does meanwhile

**The change is in and the boundary is measured rather than described.**

- `fcntl` is permitted on `Profile::Interpreter` **only**, and only for `F_GETFD` — the command
  `OwnedFd::drop` asks before `close`. It reads the close-on-exec flag of a descriptor the process
  already holds: no path, opens nothing, creates nothing, changes nothing.
- `Profile::Decoder`'s list did not move, and
  `pdf-sandbox::a_confined_decoder_cannot_ask_about_a_descriptor_at_all` is what fails if it does.
- Every other command of the same call still kills:
  `a_confined_interpreter_cannot_set_a_descriptors_flags` and
  `a_confined_interpreter_cannot_duplicate_a_descriptor_it_holds`.
- The whole run's measured `fcntl` traffic is `F_GETFD`, seven times, and nothing else
  (`strace -f -e trace=fcntl`).
- Reverting is one `if` and three test failures, none of which is a corpus figure — so an owner who
  says no costs this tree a defect back, not a rewrite. The defect being carried again would be:
  a confined worker killed by `SIGSYS` when a document is closed, in every build with library-UB
  checks compiled in, and passing in every release build.

**Principle 3 is unamended**, as it was after session 920: this round did not touch `CLAUDE.md`
either, and `Q24` is still the open question about its wording.

## Recommendation

**Answer part 1 with a standing rule, and prefer "the owner's", written down.** The technical merits
of this instance are not what makes it worth a rule — they are as good as such a case gets, and that
is precisely the argument a weaker case would borrow. A line saying *the allow-list moves only by the
owner's word, and `doc/todo/61` is where a round prepares the case* costs a round one question file
and removes the failure mode that item was created for. If the owner would rather keep the delegation,
say so there too: an item that reads as a delegation and is not one is worse than either.

**Answer part 2 by taking the repair above** if `Q24`'s wording is accepted at all.
