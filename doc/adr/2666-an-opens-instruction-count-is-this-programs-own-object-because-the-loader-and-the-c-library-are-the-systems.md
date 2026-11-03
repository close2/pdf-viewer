# 2666 — An open's instruction count is this program's own object, because the loader and the C library are the system's

Session 1918, a single round (instruments). Status: **accepted**. Context: ADR 0917 (the counted
open), ADR 2660 §2.6 and §3 (the server VM's counted findings, and this question), trap 98 (a test
binary's start moving a ceiling). It cites no clause and moves no ledger row.

## 1. What the count contained

`open_kinstructions` was callgrind's total for a whole process that opens a document and stops. On
the server VM it read 315 to 571 thousand past five rows' notebook bands. Callgrind by object and by
function, on HEAD, `xfa_filled_imm1344e.pdf` (1484.9 thousand in all):

| part | thousand | whose |
|---|---|---|
| the dynamic loader (`ld-linux-x86-64.so.2`), 567 of it `_dl_relocate_object` | 821.7 | the link editor's and the system's |
| the C library (`malloc`, `free`, `memset`, `memcpy`, `vfscanf` at start) | 389.9 | the system's |
| the test binary's own object: the harness's start and the open | 272.3 | this program's |

The open itself, `open_document` inclusive, is 404.0 of it, 177.6 of that in the C library.

## 2. What differs from the notebook, measured

- **The linker, by 320 thousand on every row.** The notebook's `cargo` used the default `cc`
  (the agent user's cargo configuration, kept in the notebook's handover, names none); the server
  VM's names
  `clang` with `-fuse-ld=mold`. The same test binary relinked here with the default linker carries
  `DT_RELACOUNT` = 15183, which `mold` 2.37.1 does not write, so the loader takes its quick loop for
  the relative relocations: `_dl_relocate_object` 248.7 against 567.3 thousand. Every row's total
  fell by 320 thousand, to the tenth: three rows then sit inside the notebook's bands
  (`xfa_filled_imm1344e` 1164.8 in 1148 .. 1172, `Well-Tagged-PDF-WTPDF-1.0` and ISO 32000-2).
- **The C library, on the rest.** With the notebook's linker `PDF20_AN001-BPC.pdf` reads 113,
  `bug1815476.pdf` 66 and `opt_demo.pdf` 27 thousand over: 11 %, 9 % and 13 % of the C library's
  instructions inside each open (985, 771, 210 thousand). Debian's glibc 2.41 is not the notebook's.
  This is consistent, not proven: no notebook profile by object exists to compare with, and three
  weeks of the tree lie between the notebook's last reading of the five-page row (3976.1, round
  1549) and this one.
- **Not the processor.** Under valgrind every program sees valgrind's own CPUID, an Intel model
  chosen from the host's AVX2 class: glibc's `--list-diagnostics` under valgrind reports
  `basic.kind=0x1` on this AMD host. Zen 1 and Zen 5 both present that model; neither presents
  AVX-512, which valgrind does not implement. `zlib-rs` took its AVX2 path here as it would there.
- **Not the toolchain.** `rust-toolchain.toml` pins 1.97.1 on both.

## 3. The decision

1. **The figure is the self cost of the test binary's own object, summed over the whole profile**
   — every thread, from the process's start to its exit. The loader's relocation and the C library
   are counted, printed beside it as the whole-process total, and banded nowhere.
2. **Not a window opened at the open's first call** (callgrind's `--toggle-collect`), which the
   brief proposed: work moved to another thread would leave the window and read as a win (trap 16),
   and what this object executes outside the open — the harness's start, the viewer's drop — is
   46 thousand on the smallest row and identical from run to run.
3. **The bands are this file's rule over the new figure** — four children of HEAD, the observed
   range widened by one per cent, rounded outwards. They are a new quantity's, derived where it is
   counted; the notebook's bands were the old quantity's and are not moved to admit anything.
   The count of the binary's own object read the same linked by `mold` and by the default linker.

## 4. What it costs

- **A regression that lives in the C library is seen only through this program's calls into it.**
  An open that allocated ten times as often still executes its allocation calls' set-up here, and
  `open_anon_mib` and `read_kib` still see the memory and the bytes; an open that copied ten times as
  many bytes through one `memcpy` would move the printed total and not the banded figure.
- A change of valgrind that changed its CPUID model could move the figure; `valgrind --version` is
  3.24.0 here.
