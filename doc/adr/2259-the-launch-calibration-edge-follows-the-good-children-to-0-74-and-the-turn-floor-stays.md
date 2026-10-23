# 2259 — The launch gate's calibration edge follows its good children to 0.74, and the turn floor stays

Session 1712, the measure round of batch one hundred and ten. Status: **accepted**. Context: ADR
2245 section 4 (the question left), ADR 1969 (when a band is re-taken), habit 101, trap 13. Check
files: `doc/checks/launch-path.toml`, `doc/checks/turn-path.toml`. Cites no clause.

## 1. What was asked

ADR 2245 left two readings. Should the calibration band, 0.62 .. 0.78 ms, decline above 0.72,
because the merges' failing children read 0.711–0.758 and quiet holds 0.669–0.704? And
`turn_ms` on `PDF20_AN001-BPC.pdf` once read 1.868 ms against a 2.13 floor: is that floor wrong?

## 2. The instrument first

The gate printed a child's calibration only beside a figure it declined or failed, so "would a
tighter edge decline a figure that passed" had no data. A clock figure judged in band now prints
the same readings on a `judged in band:` line (`crates/viewer-ui/tests/launch_path.rs`); it changes
no verdict.

## 3. One clock hold, HEAD and ADR 2258's change interleaved (`--clock --nice 0`)

Ten launch runs (four of each arm under `nice -n 19`, the merge's niceness, and one each at 0) and
four turn runs, load 2.2–4.9 while siblings built; a first hold gave four more HEAD launch runs and two turn runs.

- **The 57 clock figures judged in band in the change's five runs had children reading
  0.667–0.715 ms** (HEAD's binary predates the line). The 15 judged failures of both holds read 0.675–0.769: seven above 0.72, two above 0.74, eight at 0.675–0.715.
- Both arms failed the same warm-open mode (HEAD 4 of 9 runs, the change 2 of 5). Their open
  instruction counts agree within 0.7 thousand on every row, and the turn gate passed all four runs.
- **Every `turn_ms` under the 2.13 floor came from a child the gate declined** for waiting more
  than a tenth of its time for a processor: 2.063 (HEAD, nice 0) and 2.062 (the change). Every
  judged reading was 2.169 or more. The turn gate's own children read 0.672–1.571, median 0.735.

## 4. The decision

- **The edge is 0.74**: the file's own rule, a little over 4% above the slowest good reading,
  applied to the good readings this code produces. The probe is an interpretation, so it reads the
  code as well as the machine. The old edge rested on good readings up to 0.750, and those moved.
- **Not 0.72.** Eight failures sit inside the passes' own calibrations, so the probe does not
  see the mode that fails (ADR 2245). An edge 0.7% over the slowest good child is inside the
  probe's quiet spread. It would decline seven failures, and that would be the probe hiding them,
  not explaining them.
- **The turn floor stays at 2.13.** HEAD's own quiet runs never left it (ADR 1969). The readings
  under it are a waiting child's, and the guard already declines those.
- `turn-path.toml` keeps 0.78. Its children run the probe after their device rounds, and 111 of
  272 read above 0.74.
