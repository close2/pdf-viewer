# ADR 0105 — Five rows that said less than the code does

Status: accepted, 2026-08-01.

## Context

With the file-only evidence population at zero (ADR 0102), `doc/HANDOVER.md` names the next map:
the 41 `reported` rows and the notes on 236 `partial` ones. This session read the `reported`
population against the code — the first pass over it — and the population turned out to contain
a different defect from the one it was being read for.

`doc/HANDOVER.md` already states the rule this pass tests:

> **A stale row can understate as well as overstate, and only the overstatements have a gate.**
> Nothing fails when a row claims *less* than the code does, so the only defence is reading the
> row when you touch the family.

## What the pass found

**Five of the 41 rows understate, and one of them by eighty sessions.**

- **§12.5.6.10, text markup annotations.** The row said "[t]he four text markup subtypes are
  refused and named". `appearance.rs` has drawn all four since the thirty-fourth session (ADR
  0043) — highlight, underline, strikeout and squiggly, with the thickness taken as a fraction
  of the quadrilateral's own height and a highlight filled under `Multiply` for §11.3.5.2's
  reason. **`appearance.rs`'s own module comment said they were refused too**, three hundred
  lines above the function that draws them. `implemented`.
- **§12.5.6.6, free text annotations.** "[R]efused and named in `crate::appearance`. 4 corpus
  documents." `appearance::free_text` lays `/Contents` out through §12.7.4.3 and has since the
  twenty-third session. The four corpus documents are refused for a different reason — their
  `/DA` names a font the `/DR` does not define, which is the clause's own malformed-file case.
  `partial`, for `/CL`, `/IT` and `/BS`'s colour.
- **§12.7.5.4, choice fields.** The note describes the code exactly — a combo box draws, a list
  box is refused because the clause states no appearance for a selection — and then carries the
  status of a row where nothing is drawn. `partial`.
- **§9.7.5.2, predefined `CMap`s.** "Table 116's two Identity `CMap`s are built … and every
  other name is refused." Two of the table's entries implemented is not none. `partial`.
- **§12.7.8.3.4, FDF annotation dictionaries.** Table 254 is read into `FdfAnnotation`; what is
  missing is drawing one, which needs a second `Document` reaching the interpreter. `partial`.

None of the five is a *behaviour* change and none moved a gate. What changed is that the ledger
now says what the tree does.

## Decision

Correct the five rows, correct `appearance.rs`'s module comment, and write the one test the
corrections needed: §12.7.5.4's two halves are one flag bit apart and had nothing holding either,
so `a_combo_box_draws_its_value_and_a_list_box_says_it_cannot` draws both and checks the refusal
*by name* — a report that fired for some other reason would satisfy "something was reported".

`reported` falls **41 → 36**, `implemented` rises 364 → 365, `partial` 236 → 240.

## Consequences

**The `reported` population has the same failure mode as the file-only one, and no gate at
all.** `FILE_ONLY_EVIDENCE_CEILING` counts the rows where an *overstatement* can hide; a row
that understates fails nothing, ever, because a status of "not implemented" is never
contradicted by a passing test. The only instrument is reading the row against the code, which
is what this session did for 41 rows and what remains to be done for 240 `partial` ones.

**A comment that names a refusal outlives the refusal.** `appearance.rs`'s module header listed
the four markup subtypes among the things that "state no mark" while the same file drew all four
— for eighty sessions, through several reviews of that module. The header is where a reader
learns what the module refuses, so it is exactly where a stale refusal does most damage. The
check is one `grep` for the subtype in the same file, and it takes a minute.
