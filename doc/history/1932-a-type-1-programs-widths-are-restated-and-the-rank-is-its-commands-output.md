# 1932 — The rank is its command's output; a Type 1 program's widths are restated

Session 1932, a single archive round. ADR 2685 (2686 unused); no question.

**The rank.** The shipped census was re-run with the CJK pack. `rank.py --table` now prints section
2 of `doc/pdfa-blocking-issues.md`, which is that output, replaced whole. It gives the order, the
documents freed at 2b and 4, the `as-shown` stops, and `STATUS`, which marks each row built, builds
only or owner-blocked. Since session 1828: R02 is built and no document reaches it. R06b is built
(ADR 2498). R03, R08, R11, R03a, R04, R01, R14 and R15 are built, with builds-only residue. An
unmapped TrueType reason is now filed under R11. The top builds-only row was R10, 16 at each part.

**R10's largest part (ADR 2685).** Seven documents were refused because a Type 1 program's widths
sit inside its encrypted charstrings. Premise: veraPDF fails `bug1157493` and `issue2833` at
6.2.11.5, and both embed `/FontFile`. The build restates the `hsbw` or `sbw` width operand in the
decrypted charstring, re-encrypts it, states the lengths again, and copies every other entry byte
for byte. The writer's scan and `read-fonts` prove it. veraPDF failed the first build's outputs
for `bug1539074` (`[0.0005 0 0 0.001]`) and `bug1539074.1` (skewed): their width unit is not their
height unit. A width is now the advance's horizontal component through the `/FontMatrix`, and
`LoadedFont::program_advance`, the checker's reading, carries it. veraPDF also failed
`issue11150_reduced`, rightly: its `Symbol` states a descriptor and no widths, which ISO 32000-1
Table 111 allows a standard 14 font only with all four absent. It is now refused by that name.

**Tests.** Five in `type1::glyphs_given::widths::tests`, an extended matrix among them, and two in
`tests/archive/type1_widths.rs`. With the `/FontFile` arm or the Table 111 check planted back,
its test failed.

**Census.** Shipped, against this round's run before the change: 22 new outputs, none gone, none
changed. `bug1157493`, `bug1539074`, `bug1539074.1`, `issue1905` and `issue2833` convert under both
profiles at 2b and 4, and `ESUjulyaugust2002` under `as-shown` (`halftone-name`, R17, still stops it
under `as-if-printed`). R10 goes from 16 to 11. Stating (ADR 2669's command), against round 1931's
run: 24 new outputs (the same six documents), none gone. 139 differ, and all match once A779's
`close2/quorra` namespace is masked (71 of them after decompression). veraPDF 1.30.3 passes all 46
new outputs. Page 1 of `bug1157493` and `issue2833` renders to the same PNG before and after.

**Gates.** fmt and `clippy -D warnings --all-targets` (`pdf-font`, `pdf-transform`): exit 0.
nextest of `pdf-font`, `pdf-archive` and `pdf-transform`: 1734 passed, exit 0. Archive tier
(`corpus`, `archive_corpus`, `witnesses`): exit 0 each. `cargo test -p conformance` after
`git add -N`: exit 0. `state.sh docs`: exit 0.
