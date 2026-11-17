# 2915 — What Rust 1.100 adds to an open is LLVM 23's inflate loop

Session 2049. Status: **accepted; no band moves here.** It cites no clause and moves no ledger row.
Context: ADR 2910 section 4 (the 4.4 to 5.8% more instructions an open executes under 1.100), ADR 2666
(the counted figure is this program's own object).

## 1. The reading

`launch_path`'s counted child (`PDFVIEWER_LAUNCH_PHASE=count-open`, the environment cleared as the gate
clears it) was run under `valgrind --tool=callgrind` from two release builds of the same source, this
round's tree: 1.97.1 (LLVM 22.1.6) and 1.100.0 (LLVM 23.1.5). The functions of the program's own object,
self cost, in thousands of instructions:

| function | WTPDF 1.97.1 | WTPDF 1.100 | EC3 1.97.1 | EC3 1.100 |
|---|---|---|---|---|
| the program's object, total | 9547.5 | 10096.6 | 53271.6 | 55617.9 |
| `zlib_rs::inflate::inflate_fast_help_avx2` | 1608.8 | 2098.0 (+30%) | 5755.2 | 7422.7 (+29%) |
| `pdf_syntax::xref::read_section::{closure#0}` | 484.5 | 513.5 | 16506.5 | 17515.1 (+6%) |
| `pdf_syntax::filter::decode_with_parms_reported` | 276.1 | 266.4 | 10317.3 | 9980.5 (−3%) |
| `pdf_syntax::parser::Parser::parse_object_from` | 3598.6 | 3649.1 | 1985.3 | 2004.4 |

**One function of a dependency carries most of it**: zlib-rs 0.6.6's AVX2 fast inflate loop executes
about 30% more instructions for the same streams (489 of the 549 thousand added on WTPDF, 1667 of 2346 on
EC3), and its machine code grew from 3862 to 4101 bytes. The source is the same crate at the same
version, so the change is LLVM 23's code for that loop. The second is the closure that reads each
cross-reference stream entry, 6% more on EC3, whose 1023 pages make its xref stream the largest of
the four documents; `decode_with_parms_reported` lost a third of that, which reads as work moving
across an inlining boundary rather than appearing. Everything else moves by under a thousand
instructions, and the standard library's I/O paths only change their names (`std::io::default_read_to_end`
becomes `alloc::io::read::default_read_to_end`).

This round cannot tell which LLVM pass made the inflate loop longer; that needs the loop's assembly side
by side, and it is LLVM's and zlib-rs's question rather than this tree's.

## 2. What the bump's round does about the bands

A count is not a time: the loop may execute more instructions in fewer cycles. So the bump's round takes
the counted figures **and** the clocks of the same two documents under the new compiler in one clock
hold, and then:

- if the cold and warm opens of WTPDF and EC3 sit inside their bands, it restates the two
  `open_kinstructions` bands (and every other row's, which move by under 1%) with this ADR named as the
  cause — the compiler's code for a dependency's loop, not a change of this tree;
- if they do not, the instruction growth is a time cost too, and it is reported upstream to zlib-rs
  with the two builds' figures before any band moves (principle 2 counts a cost wherever it comes from).

Either way the band file's comment names LLVM 23 and zlib-rs's `inflate_fast_help_avx2`, so a later
round that sees the figure move back knows what moved it.
