# ADR 1165 — Twelve departures re-derived, and the one whose premise expired

Status: accepted, 2026-09-21.

Answers the owner's note `doc/adr_revisit/1119-a-word-for-a-departure-inside-an-implemented-clause.md`,
which asks for the sweep that ADR 1119's check cannot be: a `departed` row owes nothing, joins the
settled statuses, and its argument becomes a dated document that nothing re-reads. The instrument
half is ADR 1166.

## What was asked of each row

`departed` says every requirement of a clause is executed except the one the note's first sentence
names, "decided against with its cost recorded". The question a re-reading asks is not whether the
departure was right when it was taken but whether the **premise** it was taken on is still true.
Four shapes of expiry were looked for in each, and they are the four the owner's own sweep of 963
ADRs named:

- a capability the argument says this program does not have, that has since been built;
- a property of the output device — this device is a screen, nothing here prints — where printing
  is on no closed exclusion list in `CLAUDE.md`;
- a decision taken under the "we do not create files" exclusion, which has been amended four times;
- an inconsistency on the argument's own terms.

Twelve rows were read: §7.4.2, §7.4.8, §7.5.5, §7.10.2, §8.5.3.3.1, §8.6.5.7, §10.4.2.5, §12.4.4,
§12.4.4.1, §12.5.5, §12.5.6.19, §12.11.6. §10.7.5 and §12.5.2 are the other two `departed` rows and
belong to other rounds of this batch. Each was read as the clause in `doc/md/`, then as the deciding
ADR, then as the tree today; each row's note now carries one sentence saying what was re-derived
against what.

## The one that expired: §12.11.6

**ADR 0460's premise was that the four levels come later, and they came without it.** That ADR
computed §12.11.3's penalty total, declined §12.11.6's "the processing of the document shall not
continue", and gave three reasons. The second is the one that has expired, in its own words: this is
a document asserting a restriction over its reader, so the policy is asked once in a place a host can
supply, the number is computed in `pdf-model`, "the decision is taken nowhere", and *adding the four
levels later — off, on, ask, warn — is a change in the host and nothing below it*.

That change was made. `pdf_model::restriction::Level` is the four levels; `RestrictionPolicy` carries
one per `restriction::Operation`; `Event::Refused`, `Event::Asking` and `Event::Warned` are the three
answers; all three windows supply a value and a menu offers it (ADRs 0803, 0814, 1144, 1145;
`doc/todo/38`). §12.11.6's threshold was attached to none of it. So the one restriction in this tree
that a reader cannot turn on, be asked about, or be warned about is this one — which is the shape
`CLAUDE.md` principle 3 names as the thing to avoid: *a refusal that cannot become an "ask"*.

The row is `partial`, and what it is `partial` for is named: `requirements::penalty_total` past
`PENALTY_LIMIT` reaching the vocabulary the six operations reach, with `Off` staying the default the
owner's words require. The refusal itself is **not** owed — declining a `should` costs conformance
nothing, and that half of ADR 0460 stands untouched. What is owed is the policy question.

This is not a rendering change and was not taken here; `doc/todo/65`'s host-surface bucket names it.

## The eleven that hold, and what each was re-derived against

| row | the premise | what it was re-derived against |
|---|---|---|
| §7.4.2 | a recovery: refusing a hex stream over one stray byte loses the whole stream | the clause's four sentences in `doc/md/`, and `filter.rs` still skipping |
| §7.4.8 | Table 13's sentence has exactly one witness and it contradicts itself | **dated, not expired** — see below |
| §7.5.5 | obeying Table 15's `/Size` sentence costs 66 documents their page tree, and the rule protects nothing here | Table 15 in `doc/md/`, unchanged |
| §7.10.2 | Table 39 names a cubic spline and states none | Table 39, plus the clause's only other sentence about the entry — `/Order 3` ignored where `/Size` is under 4, satisfied here by construction |
| §8.5.3.3.1 | the clause calls the single pixel device-dependent and not generally useful in the breath that states it; §10.7.4 defines a clip by the fill | both sentences, and `collapsed.rs` deciding it once for every backend |
| §8.6.5.7 | this processor's native space is a screen's three components | **rests on the device** — see below |
| §10.4.2.5 | §10.4.2.1 offers the family to a less-capable processor and ranks §10.3 above it | the clause, and `ColourSpace::device_family` as the one place the ranking is applied |
| §12.4.4, §12.4.4.1 | Table 164 describes four effects and states no quantity for any | the table's own rows, four against the seven that are drawn |
| §12.5.5 | three of the standard's sentences conflict and two of them win | §12.5.5, Table 166's `/ca` and `/CA`, §12.5.2's list as Errata Collection 3 leaves it |
| §12.5.6.19 | Table 192 gives codes 2 to 5 a side and no proportion | the table, and no other clause stating one |

Two of the eleven are worth more than a row of a table.

**§7.4.8's premise is dated rather than expired.** ADR 0036's ground names no capability, no device
and no writer, so none of the four shapes bites it. What it does rest on is a count — exactly one
witness for Table 13's `/ColorTransform` sentence among the 974, and that witness contradicting
itself — and the ADR states its own revisit condition: *the next file that writes `/ColorTransform 0`
and means it will be a different question with the same clause*. The crawled population arrived after
the decision and has never been asked this entry. That is a census, not a reading, and it is named in
`doc/todo/65` rather than taken here.

**§8.6.5.7's premise is the device's, and it is the one premise among the twelve that would expire if
this program acquired a path to a device with other colourants.** Declining the clause's `should` in
the general case is grounded on this processor's native space being a screen's three components,
which is true while every raster this tree produces is one — `pdf-transform`'s page rasteriser
included. Nothing else in the decision rests on it: the passthrough on a page that composites in a
press *is* performed (ADR 0272), and ADR 1001's price is measured on the ICC's own profile of this
device rather than assumed. The row says so, so that the next reader is looking at a named condition
rather than at an absence.

## What the sweep cost, and what it says about the word

Eleven of twelve held and one had expired, which is the ratio worth recording: the status is not
rotten, and it is not self-maintaining either. Finding the one took reading twelve clauses and eleven
ADRs by hand, which is exactly the work ADR 1166's instrument makes repeatable — it cannot judge a
premise, and it can hand the next reader the documents that came after the argument, which is what
made this sweep take a day rather than a week.

Three of the twelve rows carry their own evidence that the shape is real: §12.5.6.19 refused four
entries on a capability premise and three of them were built (ADR 1090); §12.4.4 named a presentation
mode, a window and a clock as missing and all three arrived; §12.11.6 named the four levels. In every
case the row was corrected by a round that happened to be working on the clause. Nothing else was
watching.
