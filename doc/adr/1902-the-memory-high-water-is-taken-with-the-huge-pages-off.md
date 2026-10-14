# 1902 — The memory high-water is taken in a child with its huge pages off

Session 1533, an instruments round of batch eighty-four. Status: **accepted** and **built**.
Follows ADRs 0910 and 0911, which made the figure the anonymous high-water. Context: ADRs 1891 (the
carve-out guard), 0884; traps 13, 34, 35. Code: `crates/viewer-ui/tests/launch_path.rs`
(`SMALL_PAGES`, `small_pages`, `small_pages_are_on`, the `pages` phase, and the row loop's
`allocation` child). Check file: `doc/checks/launch-path.toml`, the `*_anon_mib` paragraph and
every row's `peak_anon_mib`. Cites no clause.

## 1. The question

`peak_anon_mib` is one of the gate's figures with no clock in it, judged on any machine. It is the
first-page child's `VmHWM` less its file-backed resident pages. On HEAD it fell under two floors
(PDF20 18.70 against 19.5, WTPDF 22.43 against 23.0), and interleaved children of one binary read
PDF20 at 17.76 to 20.79 MiB. Nothing in the tree had moved. The brief's hypothesis named the page
cache or transparent huge pages, and asked for a guard of the `io_ms` and carve-out shape: a
reading that declines the figure.

## 2. What was measured

This machine has `transparent_hugepage/enabled` set to `always` and `defrag` set to `madvise`. A
fault in an aligned 2 MiB range of a large anonymous mapping takes a whole huge page, however
little of the range is touched, but only where the buddy allocator has a free 2 MiB block right
then. If it has none, the fault falls back to a small page. The test used one release binary and
interleaved first-page children on PDF20, with huge pages as the machine has them against
`prctl(PR_SET_THP_DISABLE)` set before exec. With the machine's pages the children read 19.38 to
20.92 MiB. With the pages off they read 17.47 to 18.25. Polling each child's `smaps_rollup` showed
4 or 6 MiB of `AnonHugePages` at its peak and none at its end. The huge pages are transient
buffers, freed before the child reads its own figure. On the other five rows the step was 1.3 to
2.4 MiB. Bring-up showed no huge page at all.

**What a decline could not see.** With every block free and `thp_fault_fallback` not moving, a
child reached two huge pages or three, depending on where its mapping landed. The two-page children
read 19.45 to 19.63, under the floor. A guard of the carve-out's shape, declining where a fault fell
back, would have judged those children and failed them. Round 1527's 17.76 to 18.59 sit at the
level with no huge pages, and that run was beside neighbours' builds. That cause is inferred from
the level: no counter was read then.

**A plant that did not land.** A hog that took the free 2 MiB blocks (1 596 down to 86, and later
3 382 down to 568) could not drain them. The kernel compacted more back, and the children kept
their huge pages. So fragmentation was not planted. The pages-off state is set by construction,
not by the machine.

## 3. Decision

1. **Every row's `peak_anon_mib` is taken from one more child, a first-page child with its huge
   pages off.** The child's phase name ends `+small-pages`, and the child calls
   `rustix::thread::disable_transparent_huge_pages` at its start. That is a safe function, with
   `rustix` as a Linux-only dev-dependency of `viewer-ui`. The kernel keeps the flag on the address
   space. The figure is then what the program touched, at small-page granularity, in every machine
   state. This is trap 34's "the same work, in the same state": the state is pinned, not sensed.
2. **The first-page children keep the machine's pages.** Every clock figure is what a launch meets.
   They print their own allocation beside the banded one, unbanded.
3. **The gate refuses a figure whose child does not say `small_pages=1`**, read back from
   `PR_GET_THP_DISABLE`, so a child that ran with huge pages cannot pass for one that did not.
4. **Every row's band was re-derived under the file's allocation rule.** Twelve small-page
   children per row in one clock hold on 2026-10-14, the range widened by a tenth on each side and
   rounded outwards to half a mebibyte. PDF20 is 15.5 .. 20.0, WTPDF 18.5 .. 24.0, ISO 32000-2
   23.0 .. 29.5, bug1815476 38.5 .. 58.5, xfa 9.0 .. 12.5, and opt_demo 12.5 .. 16.5. Every ceiling
   came down, because the huge pages had been part of each. `bring_up_anon_mib` keeps its band,
   since its child faults no huge page.

## 4. Calibration and cost

`a_small_pages_child_has_its_huge_pages_off_and_no_other_child_does` reads the state back both
ways. With `small_pages` planted as a no-op it fails, `left: "0"` against `"1"`. The counted gate
on the export reads 31 figures banded, 0 outside. The cost is one child per row, about 0.1 s each.
The small-page figure is not tighter on every row: xfa's spreads 7.7 %, against 4.9 % with huge
pages. What the change removes is the step of 1.3 to 2.5 MiB that the machine
chose. The bug1815476 row keeps ADR 1647's two modes in both states.
