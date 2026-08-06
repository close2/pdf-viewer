# ADR 0205 — The status nobody swept, and two rows for every mechanism

Status: accepted, 2026-08-06 (session 359).

## Context

`doc/todo/01` has six sweeps over the conformance ledger, and every one of them walks the `partial`,
`reported` and `unreviewed` rows — the rows that owe something. **`inapplicable` was never swept.**
It is the status a row goes to when nobody expects to come back to it, which is precisely the
property that lets a wrong reason live there undisturbed.

ADR 0204 is what made the omission visible. §10.5's transfer function was `inapplicable` on the
phrase *marking device*, which occurs zero times in ISO 32000-2, and it decided what a screen showed
on a corpus page. The project owner's instruction after that round was to re-read the rest: clause
10's remaining `inapplicable` rows, `CLAUDE.md`'s closed exclusion list, and the 87 `out-of-scope`
rows. This ADR is the first of those, generalised into a sweep.

## Decision

### The sweep, and it is a grep of a row against the source

For each `inapplicable` row, take the capitalised identifiers and `/Key` names out of **its own
title and note**, and grep `crates/*/src` for each one. A row claiming this program does not do a
thing, whose own vocabulary the source names, is a row to read.

49 of 81 rows hit. Most hits are noise — `DeviceCMYK`, `XObject`, and the sweep's own English
(`Nothing`, `Whether`, `There`) — and the signal is a **rare** word: `GoToDp` in three files under a
§14.12 row, `DPart` in four, and the two prepress annotation subtypes.

### What it found: five wrong rows, and all five are one shape

| row | said | the clause says |
|---|---|---|
| §14.11.3 printer's marks | "by construction outside what this viewer draws … a screen is not a printer" | "[t]he Print and ReadOnly flags in the F entry shall be set and **all others clear**" |
| §14.11.6.2 trap networks | "drawing it on a screen would paint the artefact-hiding overlaps *as* artefacts" | the same sentence, verbatim |
| §14.12.4, §14.12.4.1 document parts | "[n]either is read, and neither reaches a screen" | Table 409's `/Start` is the page §12.6.4.5's `GoToDp` shows |
| §14.9.6 pronunciation hints | `inapplicable`, "the same reading §10.7.2's flatness permission gets" | §10.7.2 is `implemented`, on `CLAUDE.md`'s own rule |

**`PrinterMark` and `TrapNet` are in `annotation.rs`'s `STANDARD_SUBTYPES` and always have been**,
with no special-casing anywhere, so this tree draws both. §12.5.6.20 and §12.5.6.21 said so in their
own notes — §12.5.6.20's even ended "[w]hether such a mark belongs on a screen at all is a §14.11.3
question this row does not settle", and §14.11.3's row had settled it the other way. **The ledger
held both answers at once, in two clause families, for about a hundred sessions.**

And the clause settles it against the `§14` rows twice over, in one sentence each family uses
verbatim: the flags "shall be set and all others clear" puts `NoView` among *all others*, so a
conforming printer's mark or trap network annotation is one whose flags say **display me**. That is
the same reading this ledger already applies at `CONTRADICTED_LINK_BORDER`.

### The seventh failure shape

`doc/todo/01`'s sixth sweep compares a parent row with its children and cannot see any of this,
because these pairs are **cousins**: §12.5.6.20 and §14.11.3 are two clauses about one mechanism,
written in different sessions by different reasoning.

> **Shape 7 — two rows about one mechanism, disagreeing.** The tell is that one of them gives a
> **capability** reason ("a screen is not a printer") while the other names **code**. When a row's
> reason is about what this program *is* rather than about what the clause *says*, go and find the
> other row.

### One distinction the run had to make rather than blur

`inapplicable` is doing two jobs in this ledger:

- **a clause about a thing this program is not** — §14.10's web capture, §10.6's halftones (and that
  one is `inapplicable` on the standard's own condition, per ADR 0204);
- **a permission this program declines** — §14.11.2.2's page-boundary guidelines, "[i]nteractive PDF
  processors **may** offer the ability to display guidelines".

`CLAUDE.md` says the second is the stronger answer — "a clause that permits is a clause that has been
read, and it is a stronger answer than one that does not apply" — and §10.7.2's flatness permission
is `implemented` for exactly that. But §10.7.2 *earns* the status by naming code: the `i` operator is
parsed and discarded, and there is a test. §14.11.2.2 has no code to name, and `implemented` rows must
name evidence that exists — the checker enforces it, and refused the row when this session tried.

So §14.9.6 became `implemented`, because the alternative its own NOTE names ("text provided in Unicode
encoding combined with an indication of the language that applies to that text") is what
`accessibility.rs` answers `Query::AccessibilityTree` with, and there is a test over it. §14.11.2.2
stayed `inapplicable` **with its reason stated precisely instead of borrowing a status it has no
evidence for**. The status vocabulary has one word for two situations, and until that is worth a
status of its own, the defence is that every such note says which one it means.

## Consequences

- **`inapplicable` 84 → 79; `partial` 228 → 232; `implemented` 389 → 390.** Nothing else moved:
  **no corpus document states a `/PrinterMark`, a `/TrapNet`, a `/BoxColorInfo` or a `/DPartRoot`**,
  which was checked before any row was touched. This round changed what the ledger says, not what any
  page looks like, and saying so is the point — a ledger correction that also moved a picture would
  have been two changes.
- **One reader-side sentence came out of it unread**, and it is recorded rather than quietly dropped:
  if a page's `/LastModified` is more recent than its trap network annotation's, §14.11.6.2 says the
  trap networks "are invalid and shall be regenerated", and a reader that cannot regenerate them is
  drawing traps the clause has called invalid. `doc/todo/01` carries it.
- **The same sweep over the 87 `out-of-scope` rows is clean.** 26 name something the source names
  and every one is a refusal its own row describes — §12.5.6.25 already says a `RichMedia`
  annotation's appearances "are drawn where they exist, like any other annotation's, because nothing
  in the placement path switches on subtype", which is the sentence §14.11.3's row lacked. That is
  the third of the owner's three re-reads answered on this instrument, and only on this instrument:
  a grep says the rows do not contradict the code, not that the *exclusions* are still right.
- **The sweep is now the seventh in `doc/todo/01`** and runs over a population of 79 that had never
  been read since the sessions that wrote them.
- **The instruction that produced it was the owner's, and one of the three re-reads remains**:
  `CLAUDE.md`'s closed exclusion list. That one no grep can do, because an exclusion is a *decision*
  rather than a claim about the code — `out-of-scope` says this project decided not to, where
  `inapplicable` says the clause does not reach us, and only the second is falsifiable by a sweep.
