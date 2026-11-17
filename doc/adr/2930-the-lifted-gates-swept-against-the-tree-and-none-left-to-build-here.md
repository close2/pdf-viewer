# 2930 — The lifted gates swept against the tree, and none of them left to build here

Session 2058. Status: **accepted.** It moves no ledger row and no band. Context: ADR 2800 (A800's lift, which
`doc/todo/76` carries out), ADR 2564 (every answered question checked against the tree), the owner's answers
A254, A828 and A840.

## 1. The question

`doc/todo/76` lists the gates A800 lifted and the kind of each. Rounds were briefed from it after its rows had been
built under their own ADRs: row 7 cited ADR 2137, which ADR 2185 superseded, and row 2 still listed 1-Pass
ECMQV, which ADR 2904 built. Every row not marked built was checked against `doc/adr`, `doc/history`, the ledger
and the code, by its subject, its ADR and its witness.

## 2. What the tree says, row by row

- **Built, and the file said otherwise**: row 7 (ADR 2185 answers all five cards; its print card's printer and
  spool are `doc/todo/30`'s N4) and row 21 (ADRs 2356 and 2404: the oracle draws a fill's and a stroke's ramp from
  its own cells, and ADR 2341's two pages fell; the `alphatrans.pdf` tie is a stored alpha's, ADR 2367).
- **Built further than the file said**: row 2 (ADR 2904's 1-Pass ECMQV; brainpoolP512r1 alone is left, by its
  curve's name), row 3 (ADRs 2560, 2604, 2781, 2874, 2922 and 2923 enforce what the ledger row lists), row 28
  (ADR 2454 met ADR 2445's deferral, ADR 2700 met ADR 2519's, ADR 2642 is the restatement ADR 2630 put first; the
  Noto pack is not built) and row 29 (ADRs 2905 and 2927; clause 9.5 is the signatures lane's next).
- **Answered by the owner since**: row 10 (measured under ADR 2837; A840 lifted the exclusion as the fourth
  amendment, ADR 2876, built after the ledger's final states), rows 11 to 15, 22 and 23 (A828 kept each).
- **Unchanged**: rows 16 to 20 (`ring`, the curves, the three advisories and the measured declines all hold).

## 3. Why nothing was built after the sweep

The order's open, effort-only rows outside the other lanes are three, and none is buildable whole on this machine:

- **Row 6's WinUI half and row 7's remainder** are one dialogue, N4's printer selection and spooling. The WinUI
  window is compiled by CI alone (ADR 2136) and runs nowhere here; a print dialogue built blind and verified by no
  run is the placeholder principle 1 forbids.
- **Row 9**, a CMYK spool through the intent's `BToA`, is "after the spool path it names", and that path does not
  exist: `quorra-gtk` and `quorra-qt` print through their toolkits' RGB contexts, and RFC 0009 sections 2.7 and
  2.8's `ipp` submission and spool container — A254 item 2, placed in `doc/todo/65` section 1 by ADR 2564 — are
  unbuilt, with no `ipp` in `Cargo.lock` and no print system on this machine to receive a job. A254 adopted
  RFC 0009's answer 9, "no, until a spool path exists"; A800 lifted only the "and a document on it asks" half.
  The spool path is several slots on its own (ADR 2564) and is not this row.

## 4. Consequences

- `doc/todo/76`'s rows and status line say the above, with these ADRs; `doc/todo/72`'s sentence that the assumed
  inks were owed to a writer that does not exist now cites ADR 2700.
- The next round from this file takes A254 item 2's spool path from `doc/todo/65` section 1 before row 9, or a
  WinUI round with a Windows machine for N4.
