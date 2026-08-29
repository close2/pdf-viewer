# 824 — The field list a reference could not open, and a tie-break that heads into an exclusion

Finding: **§12.7.6.3's Table 241 gives a reset action's `/Fields` two spellings and this tree could
follow only one of them to the bottom.** The *name* form was a prefix test over §12.7.4.2's
qualified names and right; the *reference* form took the referenced object identity for a widget
identity, on an analogy with §12.6.4.11's hide action that does not hold — Table 214's `/T` names an
annotation, Table 241's `/Fields` names a *field*, and §12.7.4.1 merges a field with a widget only
where it has exactly one. So a reference to a non-terminal field reset nothing at all and none of
its descendants, and a reference to a terminal field with separate `/Kids` widgets reset nothing
either. Beside it, and about the rule rather than the standard: **the tie-break's first preference
and `CLAUDE.md`'s largest exclusion select the same ground**, measured.

Date: 2026-08-29. Argued in ADR 0753. (`main` carries through 0752 at this round's base, so 0753 is
the tip; sibling rounds hold the numbers above it.)

Touched: `crates/pdf-model/src/view.rs`, `crates/pdf-model/src/action.rs`,
`crates/pdf-model/examples/reset_form_census.rs` (new), `doc/conformance/ledger.toml` (§12.7.6.3),
`doc/errata-read.md`, `doc/todo/01-ledger-partial-rows.md`, `doc/adr/0753-…`.

## What the rule was asked and what it answered

The errata selection rule's eighteenth use. `spec-errata emit` over
`doc/ISO_32000-2_sponsored_EC3.pdf`, step 2's two greps, step 3's attribution with the family guard,
step 4's two rankings, step 5's ranking by issue.

The base population reproduces the seventeenth use's closing arithmetic exactly — **302 issue
numbers carry a strike or a caret under the recipe's own single-issue line parse and 54 were named
nowhere**, which is 819's 56 less its two verdicts; the multi-issue parse's 310 and 56 reproduce the
same way over the same eight numbers that appear only as the second number of a two-issue line.
Ninth consecutive use, third that needed no re-derivation of the parse.

Of the 54, **37 touch only a settled row, 10 touch a live one and seven land on no row at all**,
against 39, 10 and seven at 819's base — a plain subtraction of its two verdicts, for the second use
running.

## All three rankings flat again, and what a third flat use could tell

Over live rows six rows tie at two annotations with three at one; over every row 28 tie at two with
17 at one; by issue, 31 tie at two with 23 at one. That is 819's field less its verdicts. ADR 0749
had already concluded that the rankings are a filter and the tie-break the selector, and declined a
step 6 on the argument that a third *cardinal* would answer a question the rule has not been failing
at. **That argument stands.** What a third flat use adds is a measurement of where the tie-break
sends the head:

- seven of the 54 unread issues land only on `out-of-scope` rows, and **every one of the seven is
  clause 13's**;
- **three of those seven are the whole of the population's requirement-level substitutions** —
  §13.2.4.2's `should`→`shall`, §13.2.6.1's `should`→ a qualified `shall`, §13.7.2.3.5's `is`→*may
  be*. The one other modal edit in the field is §7.5.7's `might`→`may`, ISO house style over a
  possibility rather than a level.

ADR 0653's tie-break prefers a requirement level first, so its first preference and the project's
largest exclusion select the same ground — and the sixteenth use's head landing inside that
exclusion was not the accident it was recorded as. **The recipe gains step 6**: count the issues
whose every landing is inside a closed exclusion apart, the way step 3's family guard counts an
informative annex's annotations apart, and rank the rest. They are not dropped — a round with
minutes disposes of the column, which is how one leaves the population at all.

**Verdict on the rule: not retired.** 811 stated the condition — "the day an issue read whole stops
changing anything" — and that day has not come; this use's reading moved a line of behaviour, a
wrong comment beside it, a ledger row and two documents. The honest summary is narrower than
retirement: the counts have stopped ranking, the tie-break selects, and the tie-break needed a guard
that the counts never did.

## The head confirmed twice and the walk downward paid twice

**Issue #257 and Issue #662 — §13.2.4.2 and §13.2.6.1.** The head under the tie-break, and both are
inside the clause-13 exclusion, so both confirm and cost minutes. One is worth carrying out of the
exclusion as a caution: §13.2.6.1's `should be honoured.` becomes `shall only be honoured in a 'best
effort' sense.`, so **a requirement-level change is not always a strengthening**.

**Issue #683 — §12.7.6.3, Table 241's `/Flags`.** A strikeout over `; inheritable`, leaving
*(Optional)*. `action::reset_form` reads the entry off the action dictionary and follows no chain,
which the published cell called wrong — and there was never a chain to follow, since inheritance is
§7.7.3.4's page tree and §12.7.4.1's field tree and an action dictionary is in neither. Cites.

**Issue #174 — §12.7.6.3, Table 242's Include/Exclude row.** A bare caret appending to the *set*
branch the descendant parenthesis the *clear* branch has always printed, so the two branches are one
subtree question. Reading the pair whole is what found the reference-form defect above.
`view::widgets_under` walks the referenced field's subtree to its leaves now, under the same depth
bound and cycle guard the name table's walk carries, and the name form is unchanged.

Calibrated per trap 13 above the commit that makes the change, in two directions, each failing a
different assertion: the old `vec![*id]` fails the non-terminal case, and a `descend` that pushes
every node rather than only a leaf fails the assertion that the field dictionary is not itself a
widget.

**Counted before believed.** `crates/pdf-model/examples/reset_form_census` reproduces §12.7.6.3's
row's own figure and adds the question that matters: every `/Fields` element in the corpus is a
name, and not one is a reference. No gate could have seen the defect and none can see the fix, which
is trap 8 and the answer §12.6.4.2's `/SD` row already gives.

## A record defect in this rule's own file

`doc/errata-read.md` is where the blindnesses live and it has been numbering **two** lists under one
word: the instruments' (what `check` and `emit` cannot see — six, the sixth closed by `renumbered`)
and the rule's own wider list, which reached eight by the seventh use. Read in order the file says
*first, second, third, third, eighth, fifth, sixth*, and the ADRs already cross-reference between
the two numberings. Nothing is renumbered; what is added is the sentence saying they are two, and
the rule that a new blindness is numbered on the instruments' list because that is the one a command
can be written against.

## Gates and sweeps

The full §2 sequence — a change in `pdf-model` is under everything by the change→gate map. §4's
sweeps before and after, against a pristine checkout at the base commit with its own build
directory, closed with it afterwards.
