# 2244 — What an open allocates is banded, and not what the page cache left resident

Session 1705, the measure round of batch one hundred and nine. Status: **accepted** and **built**.
Follows ADRs 0910 and 1902, which did the same to `peak_anon_mib`. Context: ADRs 2209, 2221 and
2233, which merged the launch gate red; review 1704's finding 1; traps 13, 34; habit 113. Code:
`crates/viewer-ui/tests/launch_path.rs` (`Row::open_anon_mib`, the row loop's `open_allocation`
child). Check file: `doc/checks/launch-path.toml`, every row's `open_anon_mib`. Cites no clause.

## 1. What failed, and what it was

The launch gate's "what an open costs in memory", `open_peak_mib`, was the whole `VmHWM` of the
child that opens the document and stops. Its doc comment held that it "has not moved since,
because the only large file this process maps is the test binary itself". Batches 106 to 108's
controls (`/home/AI/launch-ab-1698/location-analysis.md`) ran one binary at one path four times in
one hold and read 8.508 and then 9.891 MiB on `PDF20_AN001-BPC.pdf`, failing a band of 5 .. 9.
Copying the binary moved the figure as well.

Calibrated in this round, with no clock involved: the open child's `VmHWM` is seven parts in
eight file-backed. That share (`Rss − Anonymous` of `smaps_rollup`) is the test binary's text and
the C library's, and it read 6.9 to 7.5 MiB from run to run. The anonymous share read the same to
the kibibyte: 0.805 MiB in every child on that row, 0.582 on `opt_demo.pdf`, 1.273 on the
57-page row. Twelve children each of `5ded4916` (the base of batches 104 to 108) and of
`ee365e4f` (HEAD) give the same anonymous figure on all six rows, so **no commit of batches 104
to 108 allocates more at open**. What moved was the instrument. Which pages a fault maps from a
cached file is the page cache's choice, and ADR 0910 found that already for the device's
libraries.

## 2. The decision

- The row's figure is now `open_anon_mib`: `VmHWM` less the file-backed resident pages, taken in
  one extra child per row whose transparent huge pages are off. That is ADR 1902's child, made
  for the same reason: with `enabled = always`, one huge page would be a 2 MiB step on a figure
  under a mebibyte. The whole `VmHWM` is still printed on the open's line, and nothing bands it.
- Each band follows the file's allocation rule over twelve children of HEAD: the observed range
  widened by a tenth on each side, rounded outwards to half a mebibyte. That gives 0.5 .. 1.0 on
  four rows, 1.0 .. 1.5 on the 57-page row and 3.0 .. 4.5 on ISO 32000-2.
- **Calibrated by a plant (trap 13).** A megabyte held live across the open in a scratch build
  read 1.812 MiB where the tree reads 0.805, and 1.590 where it reads 0.582. That is past both
  new ceilings. The old figure read the same plant at 8.75 to 8.81 MiB, inside its 5 .. 9: the
  band this replaces could not see a megabyte, and the new one can.

- **The owner's `A32` still holds, on its own premise.** Q32 recommended the floor on this row
  because the figure "has never moved" and has no device in it, so a two-sided band would "catch
  a harness measuring nothing". The whole `VmHWM` no longer meets that premise and the anonymous
  share does. So the two-sided band stays on the open's memory, and only what is measured
  changes.

## 3. What it costs

One more child per row, about a tenth of a second each. The gate no longer claims how much
resident memory the open process occupies. ADR 0910 already gave up that claim for the
first-page process, and `read_kib` still bands the read side.
