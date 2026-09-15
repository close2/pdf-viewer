# ADR 1083 — A launch-path band re-centres with the change that needs it, and the header is found in one pass

## Status

Accepted, 2026-09-15. Session 1069, the measurement slot. Answers the third Consequence of
ADR 1081, which found three `open_kinstructions` figures near the high edge of their bands and
correctly moved nothing. Crates: `crates/pdf-syntax`. Gates: `viewer-ui --test launch_path`,
`pdf-syntax --test cross_references`. **Moves no band.**

`§N` is ISO 32000-2 and nothing else.

## Context

ADR 1081 made `gate_ratchet::band` print every figure's distance to the nearer edge of its band,
and on its first run three `open_kinstructions` figures stood 10–15% from the **high** edge of
bands whose own measured spread is 0.011%. The next round to add a little to an open would fail a
gate for a change nobody would call a regression. 1081 declined to move a band on that: a band is
a claim about a program, and moving one needs the measurement rather than the discovery
(ADR 1044).

## The measurement

Two arms, each its own checkout and its own build directory: `d9fd1906`, the batch carrying
session 1027, which is where the bands were last looked at — and `e168f02c`, where 1081 printed.
Counted the way this figure is always counted, under callgrind, on the gate's own
`phase_count-open` child with a cleared environment. Thousands of instructions:

| document | at 1027 | at 1067 | Δ | of which `pdf_syntax::xref::read` | with the fix below |
|---|---|---|---|---|---|
| `PDF20_AN001-BPC.pdf` | 4942.7 | 4955.1 | +12.4 | +12.243 | 4943.6 |
| `Well-Tagged-PDF-WTPDF-1.0.pdf` | 26665.3 | 26677.8 | +12.5 | +12.243 | 26665.8 |
| `ISO_32000-2_sponsored_EC3.pdf` | 185009.6 | 185019.6 | +10.0 | +12.242 | 185014.9 |
| `bug1815476.pdf` | 3394.7 | 3408.5 | +13.8 | +12.245 | 3396.6 |
| `xfa_filled_imm1344e.pdf` | 1801.4 | 1814.6 | +13.2 | +12.243 | 1802.0 |

**One cause, and it is the same number on every row.** `xref::read`'s self cost grew by 12 243
instructions whatever the document — a constant, where every other candidate the round was given
would have scaled with pages, objects or signatures. The 1027 column reproduces ADR 1044's own
figures (4942.3 and 3394.4) on a binary built forty sessions later, which is what
makes the pair comparable at all.

**What did not move, verified rather than assumed.** The signed row's profile contains not one
instruction of `pdf_signature`, `notes::about`, `submission` or trust — ADR 1044's deferral holds
exactly, and `notes::losses` costs 108 instructions. `Command::Trust` never reaches this path: the
harness sends `Restrict` and `Open`, and `Open` does not read the policy. The submission wiring
and the Table 197 constraints are on trigger paths a launch does not take.

## Decision

### 1. The header is the first window that is either marker

§7.5.2 makes the header the origin of every offset in the file, and ADR 1066 made it the *earlier*
of `%PDF-` and `%FDF-` — which is right, and was implemented as two searches over the same
kilobyte and a `min` of the results. The second search runs to the end of the window looking for a
marker that is not there, however early the header stands, and that is the 12 243 instructions.

"The earlier of the two markers" **is** the first window that is either of them, so `header_position`
asks that question of each window once and stops at the first hit. Same answer in every case,
including the file this tree writes that made ADR 1066 necessary —
`cross_references.rs::the_header_is_whichever_marker_stands_first_in_the_file` holds both orders and
is untouched.

### 2. This is (b), not (a)

`CLAUDE.md` principle 2's presumption is that work on the launch path which page one does not need
comes off it. Finding the header is not that — every offset in the file depends on it, and it
cannot be deferred. What could come off is the *second pass*, which answers nothing the first has
not, and no band moves for a cost that should not have been paid.

### 3. Two of ADR 1081's three figures did not move, and their bands do not re-centre here

`Well-Tagged` and `ISO_32000-2` read 26665.3 and 185009.6 at the 1027 arm — **17.9% and 11.3% from
their high edges before a single commit of this window landed**. ADR 1044 says so in its own words,
in the units it used: those two rows sat "1.6% and 1.8% above their floors" when 1027 measured them
and it left both ends alone. Their bands were derived at session 938; the 0.64% and 0.78% that put
them off-centre is 89 sessions of growth that no round attributed and this one cannot.

So ADR 1081's alarm had two causes wearing one shape. One was a regression and is gone. The other
is a standing asymmetry, and **a band re-centres with the change that needs it, carrying that
change's reason** — which is ADR 1044's rule read forwards: a round that adds to an open has to
move a band and say what it added. Re-centring in advance would hand the next such round a band
already widened for work nobody had to argue for, which is the thing ADR 1044 exists to prevent.

## Consequences

- Every `open_kinstructions` figure is back where the round that last measured it left it. The
  signed row returns from 14.6% of its high edge to 48.6% — dead centre, which is where 1027 put
  it. The other four read 38.2%, 17.8%, 11.1% and 48.9%.
- Three consecutive runs of the fixed arm spread at most 0.2 thousand instructions on any row
  (0.006% to 0.01%), which is the spread `doc/checks/launch-path.toml` states and is why a
  12-thousand step is legible at all.
- **`position_of` is gone**, having had these two callers and no others.
- The two off-centre rows keep their bands and their headroom — 94.2 and 409.1 thousand
  instructions. A round that spends it re-centres, with its reason above the number, exactly as
  session 925 and session 1027 lowered floors they had measured.
- What this does not answer: what the 89 sessions between 938 and 1027 added to those two opens.
  It is not a defect — both figures were inside their bands throughout — and it is not recoverable
  from a document, only from the same two-arm measurement run over that span.
