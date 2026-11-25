# 3634 — The accessibility census's whole-population floors count H.264 among the specifications

Session 2233. Status: accepted. Context: ADRs 1075, 1151, 3537; trap 43.

## Decision

The whole-population floors of `accessibility_census.rs` are restated to the population the run
reads with the two specifications that joined the owner's `doc/` on 2026-11-23 and 2026-11-24,
and both join `RATCHETED_SPECIFICATIONS`, so a disk without them skips the tier by name rather
than failing it:

- `ITU-T-H.264-202606.pdf` (ADR 3537) is tagged, 854 pages: documents with structure 114 → 115,
  pages answering 2659 → 3513 (exactly its 854), and with it elements reached, `/Alt` or `/E`
  carried, the three placement counts, cells with headers and their associations, elements that
  are annotations, elements a caret reaches, lines and characters.
- `ISO-IEC-14496-12-2012.pdf` (ADR 3460) is untagged: untagged pages answering honestly 901 → 902.

The other seven whole-population floors and every tracked-set ratchet read the same as before.

## How the figures were taken

`gate_ratchet` has no update mode: a floor's bound is source, the population is the run's
output, and a floor stops the run at the first bound out of step. So the figures are the run's
own report lines — the census prints every whole-population population above the ratchets — and
each bound is that number, held at zero slack; a run of the restated gate is the check.

## Left

Nothing; the next specification that joins `doc/` moves these floors the same way (trap 43).
