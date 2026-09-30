# 1423 — The parsers without a fuzz target get one, and a campaign runs under the bound

Session 1293. Status: accepted and **built**.
Context: CLAUDE.md principle 3 ("Fuzzing from the first parser commit. Every crasher found becomes
a permanent regression test"); ISO 32000-2 §7.4.7, §7.4.9, §12.7.4.3, §12.7.6.4, Annex F. Code:
`fuzz/Cargo.toml`, `fuzz/fuzz_targets/{shaping,jbig2,jpx,xfdf,linearize,sfnt}.rs`, `fuzz/seeds.sh`,
`fuzz/seed_shaping.py`, `fuzz/seed_forms_data.py`, `crates/pdf-model/examples/image_codec_seeds.rs`, `tools/state.sh`
(`section_fuzz`), `doc/verify.md`'s fuzz block. Builds on ADRs 0742 (a fuzz run that fuzzed
nothing), 0747 (`INITED` beside `DONE`), 1024 (`--list` fails on an undocumented target).

## 1. What had no target

Five readers of untrusted bytes had been added since the last campaign with nothing fuzzing them,
and a sixth had lost its door:

| target | reads | property beyond "no panic" |
|---|---|---|
| `shaping` | UAX #9 and cursive joining over a field value (ADRs 1413, 1414, 1417) | joining shapes every character once; rule L2 is a permutation; a label's glyphs cover its text on character boundaries |
| `jbig2` | §7.4.7 as the worker runs it, in-process | rows packed to the grid; `delivered` ≤ height |
| `jpx` | §7.4.9: `pdf_model::jpeg2000::Headers::parse` and the worker's decode | samples fill the stated grid; a whole decode is at the codestream's grid and inside its allowance |
| `xfdf` | ISO 19444-1 and the import it feeds (ADR 1297) | the reader is a function of the bytes; the import terminates |
| `linearize` | Annex F both ways (ADRs 1293, 1309) | a file `serialize_linearized` wrote opens, and reads as linearised with `/L` its length and `/N` its pages |
| `sfnt` (extended) | ADR 1411's `composite_cycle` | a glyph named as closing a cycle is on one |

**And one target had never fuzzed what it is named for.** `forms_data` reads its input as a §7.9.4
date and then as an FDF file; its corpus on this disk held 1344 seeds and not one FDF file, so a
ten-minute run reached 275 edges and never entered `FormsData::read`. `fuzz/seed_forms_data.py`
writes one FDF file per Table 243 and Table 246 entry the reader handles; its 21 seeds alone reach
1241 edges. ADR 0742's check asks whether a corpus is *empty*; this one was full of the wrong thing.

**And one corpus had gone stale under it.** `display_list`'s 2019 seeds on this disk are lists in
an encoding ADR 0607's wire format has since moved past: the corpus alone reaches 105 edges, and
29 seeds written today by `list_over_the_wire --seeds` from the first thirty pdf.js documents reach
576. A seed count cannot see that; the `INITED` figure `tools/fuzz.sh` prints can, and it is the
number to read before a campaign — a corpus far below what a handful of fresh seeds reach is a
corpus to regenerate.

`ccitt` existed but had no `doc/verify.md` line, so `tools/fuzz.sh --list` exited 1 and
`tools/fuzz.sh ccitt` refused it — ADR 1024's check doing its job with nobody reading it.

The JBIG2 and JPEG 2000 codecs are `hayro-jbig2` and `hayro-jpeg2000`, confined in the worker;
`pdf_sandbox::Isolation::InProcess` calls the worker's own functions, so a target reaches the
framing, the prefix retry and the budgets this tree writes around them without a process per
input. A panic inside a codec would be the dependency's, stated as a regression test at the
filter's boundary.

## 2. How a campaign runs

- **Without the sanitiser.** `tools/bounded.sh`'s `RLIMIT_DATA` refuses AddressSanitizer's shadow
  reservation (`doc/verify.md`, trap 24), and every crate a target reaches forbids `unsafe`, so what
  ASan adds is a dependency's unsafe code. `cargo fuzz build -O -s none` keeps coverage and overflow
  checks, fits the bound, and lets the lock and the tree ceiling apply as they do to every walk. The
  sanitised build remains the one to reach for when a dependency's crash needs its stack.
- **A scratch corpus first.** A worktree's `fuzz/corpus` and `fuzz/artifacts` are links into the
  main checkout; libFuzzer writes new units to its *first* directory, so the seeded corpus is given
  second and read, and `-artifact_prefix` points at scratch.
- **Seeds by recipe.** `fuzz/seeds.sh` seeds the five new targets from the tree: the UCD's
  bidirectional cases and generated Arabic words, the XFDF test files, whole documents for
  `linearize`, and every JBIG2 and JPX codestream of the corpus framed the way each target reads it.
- **What the disk holds is printed.** `tools/state.sh fuzz` lists every target with its seeds, its
  `doc/verify.md` line, and the crashes, timeouts, memory refusals and slow units in
  `fuzz/artifacts/`, with the newest one's date — the artefact ADR 1424 fixed had sat there six weeks.

`fuzz/Cargo.lock` is gitignored, so the fuzz workspace resolves on its own; a copy older than the
workspace's lock failed to build `pdf-signature`'s `brainpool_p512` (`hybrid-array` 0.4.13 against
the 0.4.15 the workspace pins). `cargo update -p hybrid-array --precise 0.4.15` and `-p wnaf --precise 0.14.1` in `fuzz/`
aligned it; nothing in the tree checks the two locks agree.
