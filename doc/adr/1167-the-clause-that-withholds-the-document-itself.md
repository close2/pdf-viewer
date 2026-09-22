# ADR 1167 — The clause that withholds the document itself

Status: accepted, 2026-09-22. Session 1165, the batch's host-UI round.

§12.11.6's requirements processing is a `restriction::Operation` of its own, so that `off`, `on`,
ask and warn reach it the way they reach every other restriction a document asserts over its
reader. Supersedes ADR 0460's departure, whose premise ADR 1165 had already found expired.

## 1. What the clause asks, and what was missing

> If requirements cannot be met, as determined by the computation of the penalty value as
> described in 12.11.3, "Requirement penalty values", then the processing of the document shall
> not continue.

The computation has been performed since session 626 — `requirements::penalty_total` sums the
penalties of the requirements `Kind::unmet` answers, and §12.11.3's last paragraph states the
threshold it is compared against. What was missing was any way for a reader to act on it.
`CLAUDE.md` names the shape: *a refusal that cannot become an "ask" is the thing to avoid*, and
this was the only restriction in the tree that a reader could not turn on, be asked about or be
warned of.

## 2. Why an `Operation` and not a refusal at the point of the open

The alternative was a boolean in `viewer-core`: open the document, or do not. It fails the same
test `pdf_model::restriction` was built against — a function answering `false` has thrown away
everything the question would need — and it would have put a second policy beside
`RestrictionPolicy`, with its own default, its own command and its own wire byte. `Operation` is
the vocabulary that already carries a level per restriction, a per-document departure, a command
line word, a menu row, a wire byte and a C constant; adding an arm gets all seven for nothing.

Three readings had to be taken to make the arm honest, and each is written beside the code:

- **Table 22 states no position for it.** `Operation::bit` answers `Option<Bit>` now, and `None`
  is a statement about the table rather than a hole: its eight positions are about what a reader
  does *to* a document that is open, and this clause is about whether it is opened at all.
- **Table 257's certification says nothing about it either.** Every level of `/DocMDP` is about
  "changes to the document", and processing one changes nothing.
- **It is asked of the processing and of nothing else.** NOTE 1 is what makes that safe rather
  than narrow — "there is no formal connection between the requirement type and the operation of
  the associated feature(s)" — so an unmet requirement is never a reason to refuse a particular
  verb, and `asserted` consults the penalty only for `Operation::Process`.

## 3. Where it is asked, and what each level does

`Viewer::open`, after the host-supplied values are placed on the new `Open` and before anything
else opening a document does. That ordering is the clause's own: the requirements are evaluated
first, so *not continuing* is a document that was read and put down again rather than one half
processed. `Viewer::process` is the rest of the open, split out for exactly this.

| level | what happens |
|---|---|
| `off` | the document opens, and `notes::about` still names what could not be promised |
| `on` | `Event::Refused`, no `Event::Opened` at all |
| `ask` | `Event::Asking`, the document held unfocused until `Command::Answer` |
| `warn` | the document opens, and `Event::Warned` follows every event the open raised |

The window's levels rather than the focused document's, because a departure belongs to a document
that is open and this one is not (ADR 1145).

**The `no` to this question is the first that has something to undo.** An edit declined leaves a
document that is open either way; an *open* declined would leave `viewer-core` holding a document
nobody can see, query or close, so `answer` forgets it. That is the one asymmetry in `Held`, and
it is why `Held::Process` exists rather than a flag.

## 4. The one cost that had to be paid back

Asking the policy at the open moves `restriction::asserted` onto the launch path, where it had
never been: every previous caller was a gesture. Two of its sources cost a walk —
`pdf_signature::signature::field_lock_permissions` visits every signed field in §12.7.4's field tree —
and neither can withhold `Process`, because every level of Table 257 is about "changes to the
document" and processing one changes nothing. So `asserted` asks `certification_binds(operation)`
first, which is a pure question about the operation and answers `false` for the four operations
`certification_permits` permits at every level. `no_operation_is_asked_of_a_level_that_permits_it`
is what keeps the two from drifting apart. `CLAUDE.md` principle 2, at the one place in this change
where it bites.

## 5. What it cost

`RestrictionPolicy` is seven levels rather than six, which the wire, the command line and the menu
all enumerate from `OPERATIONS` and therefore needed no edit. `QUORRA_RESTRICTED_PROCESS` is a
number an old C caller never passes, so `QUORRA_ABI_VERSION` does not move (ADR 0576 section 4).
Two sibling matches over `Operation` gained an arm apiece (`pdf-vfs`'s wire, `viewer-confined`'s),
and two over `Restriction` gained a sentence apiece (`pdf-transform`'s stderr wording,
`save_round_trip`'s census line) — neither crate can produce the new arm, and both word it because
a variant a match cannot word is a sentence waiting to be missing.

The ledger row is `implemented`, on §7.6.4.1's own reading of a `shall` addressed to a reader: one
that the reader can turn off is one command away rather than unread, and `CLAUDE.md` requires that
it always be possible to turn it off.
