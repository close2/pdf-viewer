# 961 — The edition that reached two routes and not the third

Date: 2026-09-11. Branch `round-945/the-fifth-round`, from `44c77ef5`. Stream: the PDF/A validator,
`crates/pdf-archive`. Two sibling rounds were live in the same tree throughout (the viewer, and the
converter in `crates/pdf-transform/src/archive/`).

## What the round was asked to do, and what it did

A sweep of every `Check::Unchecked` and `Check::Processor` reason, asking of each whether it is
still true — and, where a text this project holds had quietly made a row checkable, closing it.

- **All twenty-three `Processor` reasons were read against ISO 19005-2 clause 6 and ISO 19005-4
  clause 6.** Every one is accurate. Nothing moved there.
- **All twenty `Unchecked` reasons were re-read.** Session 958 had read the same twenty; one of them
  is sharpened here rather than corrected, and the rest stand.
- **The crate's ordinary doc comments were swept for the same class of claim**, which is where the
  three findings were. ADR 0964 has them: a capability that reached two of three call sites, a
  ground that was never true for part 2, and a comment that cited the note disproving it.

## The figures, off the run

The corpus harness, before and after the change, identical:

| target | agreed | missed | over | settled | elsewhere | unreadable |
|---|---|---|---|---|---|---|
| PDF/A-4 | 473 | 0 | 0 | 8 | 6 | 0 |
| PDF/A-4f | 9 | 0 | 0 | 0 | 2 | 0 |
| PDF/A-4e | 17 | 0 | 0 | 1 | 1 | 0 |
| PDF/A-2b | 971 | 1 | 0 | 7 | 7 | 0 |
| PDF/A-2u | 21 | 0 | 0 | 1 | 0 | 0 |
| PDF/A-2a | 27 | 0 | 0 | 0 | 0 | 0 |

`over` is 0 on all six. The one miss is `6-6-2-3-3-t03-fail-b`, where errata A029 makes our answer
the right one. The coverage census (`--example targets`) is unchanged on every target.

## Files touched

- `crates/pdf-archive/src/table/graphics.rs`
- `crates/pdf-archive/src/table/interaction.rs`
- `crates/pdf-archive/src/table/metadata.rs`
- `doc/adr/0964-the-edition-that-reached-two-routes-and-not-the-third.md`
- this file

**No requirement identifier was added, removed or re-keyed**, so
`crates/pdf-transform/tests/archive_unconsidered.txt` is untouched by this round.

## What the next round should know

- `cargo fmt --all --check` was red in this tree at the end of the round, in
  `crates/pdf-transform/src/archive/prepare.rs` and `sites.rs` — the converter round's live files,
  not this one's. `cargo fmt -p pdf-archive --check` is clean.
- The part 2 / part 4 split found here is general, not local to ICC: a rule ISO 19005-2 delegates to
  ISO 32000-1:2008 may read differently from the same-numbered clause of ISO 32000-2, and every part
  2 row citing a `§` is a row that has not been checked against the edition part 2 names.
  `doc/questions/A49` calls that tree work; ADR 0964 section 2 is the first instance met in the
  concrete, and it was not found by looking for it.
