# 2660 — Every clock band is the server VM's, re-taken by ADR 1969's rule, and the counted figures it fails are findings, not bands

Session 1913, a single round (measure). Status: **accepted**. Context: CLAUDE.md "The machine: the
server VM" (development moved there on 2026-11-03), ADR 1969 (the band rule), ADR 2242 (how a red
gate is judged), ADR 2158 (a cold first page takes its row's warm band). It cites no clause and
moves no ledger row.

## 1. The question

Every clock band in `doc/checks/` was taken on the owner's notebook — a Ryzen AI 9 HX 370 with a
Radeon 890M. On the server VM (an EPYC 7301, Zen 1, one class of core; a Radeon RX 580 shared with a
second account) the launch gate's calibration probe read 2.0 ms against a band of 0.62 .. 0.74, so the
launch gate judged no clock at all, and every clock figure sat three to eight times past its band.
A gate that cannot judge here detects nothing here.

## 2. Decisions

1. **The bands in `launch-path.toml`, `turn-path.toml` and `toolkit-launch.toml` are this machine's**,
   each ADR 1969's rule — the smallest of three runs × 0.85 to the largest × 1.20, rounded outward —
   over three runs of HEAD in quiet `--clock` holds on 2026-11-03. Each hardware run took its own
   `with-gpu-lock` with `VK_ICD_FILENAMES` naming the radeon driver alone, and the device's busy share
   read 0 % before and after every run. Quiet is the calibration's own evidence: the launch gate's
   probe read 1.996, 2.000 and 2.001 ms over the three runs at a one-minute load of 22 to 35, of which
   the other account's work on CPUs 32-47 is a share no child of ours runs beside.
2. **The calibration bands are the same rule over the probe's own three readings**: 1.69 .. 2.41 in
   both files (the launch gate's run figure; the turn gate's quickest child of each run). That is
   wider in proportion than the notebook's band, which sat 7 % under and 4 % over its good children,
   because the notebook's edges came from figures already judged in band and nothing was judged here
   until the bands existed. The cost is that a machine up to a fifth slower passes the probe, which
   is the width every clock band already allows.
3. **`io_latency_ms` and `io_ms` are re-taken by their own shape** — the observed minimum less a
   sixth to the observed median × 2.5 — over the hold's 33 cold samples: the build directory, where
   the gate keeps its copies, is a rotating disk here.
4. **`cold_first_page_ms` takes the row's new warm band**, as ADR 2158 rules; its own runs are written
   beside it, and every one sits inside.
5. **`device_busy_percent` stays 20**: `a_planted_busy_device_is_seen_as_busy` read 0.0 % idle and
   95.8 % planted on the RX 580.
6. **The counted figures are judged, not re-taken**, and three do not hold on this machine. They are
   findings, recorded in the round's record; no band of theirs moves here:
   - `open_kinstructions` reads past every row's band by a near-constant 320 to 530 thousand
     instructions (4497.5 against 3933 .. 4070 on the five-page row; 1480.3 against 1148 .. 1172 on
     `xfa_filled_imm1344e.pdf`; 70256.1 inside on ISO 32000-2), identical over three runs. A constant
     added to every open is the process's start rather than the open — this system's dynamic loader
     and C library against the notebook's — and was not separated further.
   - `peak_anon_mib` reads 18 to 21 MiB over every row's band (39.6 against 15.5 .. 20.0), and
     `bring_up_anon_mib` 15.2 against 4.8 .. 7.5: the driver's own allocation is about 9 MiB of the
     difference, and the rest was not separated.
   - `read_kib`, `read_calls` and `open_anon_mib` hold on every row.

## 3. What follows

- The three counted figures are red on this machine at every run, so they are ADR 2242's
  "persistent" case from the next merge on, and need their own round: a counted figure is a property
  of the reader and a band for it may not be moved to admit a machine. Whether the process's start
  should be subtracted from `open_kinstructions`, and the driver's share from `peak_anon_mib`, is that
  round's question.
- The `step_lowest_ms` bands of two turn rows are the notebook's: no run here read at the RX 580's
  lowest level, 300 MHz, and their `step_raised_mhz` are this machine's 1366.
- Round 1914 asks whether bands expressed as factors of the probes survive a change of machine; the
  runs each band rests on are written above it, and the notebook's are in git.
