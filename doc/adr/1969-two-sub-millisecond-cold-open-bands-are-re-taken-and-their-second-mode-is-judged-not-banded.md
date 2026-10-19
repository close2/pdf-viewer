# 1969 — Two sub-millisecond cold-open bands are re-taken, and their second mode is judged, not banded

Session 1566, slot 3 of batch ninety. Status: **accepted**. Context: `doc/checks/launch-path.toml`'s
own clock rule (three runs, clocks smallest × 0.85 to largest × 1.20), habit 101 (a sub-millisecond
figure outside its band is judged by its instruction count against HEAD before anything is
bisected), ADR 1863 (a round's clock figures are one hold), ADR 1044 and ADR 1620 (the two rows).
It cites no clause and moves no ledger row.

## 1. What the gate kept saying

For three merges running, `t2-launch_path` failed on the cold open of `bug1815476.pdf`
(0.32 .. 0.50 ms) or `opt_demo.pdf` (0.13 .. 0.27 ms). The batch commit's record read 0.511 and
0.589 ms for the first and 0.426 ms for the second, and round 1560's record read 0.577 ms for the
first, where HEAD's export failed it too. Each time, HEAD measured beside the batch agreed with it
in instructions, bytes and read calls.

## 2. What one hold measured

One `--clock` hold at a one-minute load of 1.4 to 2.6 ran the gate three times on an export of HEAD,
interleaved with three runs of slot 3's export, whose open is the same code (slot 3 changed only
`quorra`'s renderer).

| row | HEAD's export, ms | the other export, ms | the rule's band over HEAD's |
|---|---|---|---|
| `bug1815476.pdf` | 0.447, 0.413, 0.466 | 0.452, 0.413, 0.492 | 0.35 .. 0.56 |
| `opt_demo.pdf` | 0.176, 0.231, 0.183 | 0.407, 0.176, 0.205 | 0.14 .. 0.28 |

## 3. The decision

- **The two bands are the file's rule over HEAD's three runs**, rounded outward to the file's two
  decimals: 0.35 .. 0.56 and 0.14 .. 0.28. The date and the rule are written beside each row.
- **The second mode is not banded.** The readings past the new ceilings are 0.577 and 0.589 ms on
  `bug1815476.pdf`, and 0.426 ms and this hold's 0.407 ms on `opt_demo.pdf`. Each came with the
  row's counted figures in their bands, and the batch commit's record puts the difference in the
  disk's round trips on an open of under a millisecond. A band widened to hold that mode would no
  longer be the file's rule, and a band that is not moved by the rule is moved to make a gate pass.
  So a reading in that mode stays a failure on the gate's line. It is then judged as habit 101 says:
  the instruction count and the bytes against HEAD's, before anything is bisected.

## 4. Not done

Nothing in this tree says whether three quiet runs are too few for a row whose clock has two modes.
The rule is the file's, for every row. A rule that took more runs for the sub-millisecond rows would
be a change to the gate, argued on its own.
