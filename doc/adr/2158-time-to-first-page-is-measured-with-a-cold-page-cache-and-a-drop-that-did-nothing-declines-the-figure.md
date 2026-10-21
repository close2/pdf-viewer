# 2158 — Time to first page is measured with a cold page cache, and a drop that did nothing declines the figure

Session 1661, slot 7 of batch one hundred and two. Status: **accepted**. Context: `CLAUDE.md`
principle 2 ("Cold-start and time-to-first-page are CI gates with numbers attached, measured with a
cold page cache"); `doc/todo/42` section 6; ADR 1969, whose rule the bands are read against; ADRs
1872 and 1891, the carve-out guard; ADR 2125, the toolkit harness. It cites no clause and moves no
ledger row.

## 1. What was there, and what was not

`cold_open_ms` already dropped the document's pages before every sample (`dd oflag=nocache`, which
is `posix_fadvise(POSIX_FADV_DONTNEED)`, on a copy beside the build directory). Time to first page
did not: its child read the same copy straight after the warm open, so `first_page_ms` was a warm
figure. And neither arm asked whether the drop had taken effect. On `tmpfs` it never does, and
`dd` still exits 0.

## 2. What is built

- **`cold_first_page_ms`**, the first-page child unchanged, with each of its samples prepared the
  way a cold open's is: drop, residency read back, the disk's two probes. It is banded on every row,
  sits under the carve-out guard as `first_page_ms` does, and prints the document thread's instant
  beside the device's.
- **The residency read-back**: `fincore` (util-linux, `mincore(2)` on a mapping, faults nothing in)
  after each drop, carried as `cache_left_kib`. Any kibibyte left declines the figure as
  `TheCacheStayed`, named before any machine guard, for both cold figures. A machine without
  `fincore` is told so once at the top of the run.
- **A cold frame must hash as the warm frame does**, or the run fails on any machine.

## 3. What one hold measured (2026-10-21, one `--clock` hold, HEAD's export and this one interleaved)

The cold first page next to the warm one, in ms, this round's three runs. Every run had the adapter's
carve-out 464 MiB held by others, so both figures were declined there (the last merge's run had 7
declined for the carve-out too).

| row | warm first page | cold first page | cold open ÷ warm open |
|---|---|---|---|
| `PDF20_AN001-BPC.pdf` | 29.6, 29.3, 28.5 | 29.1, 28.7, 28.6 | 1.5 |
| `Well-Tagged-PDF-WTPDF-1.0.pdf` | 31.0, 30.3, 31.7 | 31.0, 31.0, 31.5 | 1.3 |
| `ISO_32000-2_sponsored_EC3.pdf` | 30.6, 29.8, 30.0 | 30.4, 30.5, 32.8 | 1.2 |
| `bug1815476.pdf` | 31.3, 30.3, 35.8 | 30.9, 30.7, 40.1 | 1.3 |
| `xfa_filled_imm1344e.pdf` | 27.5, 29.0, 36.3 | 28.7, 28.8, 34.1 | 5.3 |
| `opt_demo.pdf` | 30.6, 30.4, 34.5 | 31.1, 30.7, 31.9 | 1.6 |

The document thread finished 7 to 19 ms before the device on five rows. On the font-substitution
row it finished level with the device, warm or cold. A cold page cache costs the launch nothing past
the warm figure's own spread, because the disk's time is hidden behind bring-up. The brief guessed
the cold open would be two to five times the warm one. It is 1.2 to 1.6 times on five rows and 5.3 on
the signed one.

## 4. The decision on the bands

**Each row's `cold_first_page_ms` is that row's `first_page_ms` band.** The file's rule over the
three runs (smallest × 0.85 .. largest × 1.20) gives 24.3 .. 35.0, 26.3 .. 37.8, 25.8 .. 39.4,
26.0 .. 48.2, 24.3 .. 41.0 and 26.0 .. 38.3. Those come from figures taken while the carve-out had
no room, which is a state the gate declines, so the bands would be judged only on runs unlike the
ones they came from — and a judged run with room could fall out of their lower edges: the four rows
that record their quiet-machine warm figures give 23.8 to 31.5 ms, the signed row's 23.8 is under
the lower edge it would get, and the lowest of the other three are within 0.7 ms of theirs. The
warm band was taken by the same rule with room, and the table shows the cold figure inside the warm
one's spread. So a cold first page outside the warm band means the disk has reached the critical
path, and that is the claim being gated. When three runs with room exist, re-taking the cold band
by the rule is an edit to the check file.

## 5. The calibration (trap 13)

- **A slower open is named.** A 40 ms sleep on the document thread, in the same hold, put every row's
  `cold_first_page_ms` outside its band, 45.6 to 61.5 ms, and named it in the summary. It did not
  fail: the carve-out declined it, exactly as it declined `first_page_ms` beside it. A run with room
  is what makes it fail, and no run this round had room.
- **A drop that did nothing is declined.** The same gate binary, run with its copies on `tmpfs`,
  declined every cold figure it was not already declining for load as "the document was still in
  the page cache after the drop". The test `a_drop_that_left_the_file_resident_is_read_as_resident`
  keeps that reading honest.

## 6. Not built: the toolkit windows' cold stage

`crates/viewer-host/tests/toolkit_launch/mod.rs` opens the repository's own document, which a drop
must not open for writing. It would need a copy, a third interleaved set of launches across both
windows under `Xvfb`, and bands in `doc/checks/toolkit-launch.toml`. All of that would buy a term
this gate already measures, since the document and `Command::Open` are the same in all three windows.
What a cold *toolkit* launch would add is the toolkit's shared objects coming from the disk, and those
pages are shared with every process on the machine. An unprivileged gate does not drop them, and
`/proc/sys/vm/drop_caches` is root's.
