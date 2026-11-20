# 2140 — The launch bands, and an ancestor read once

Session 2140, a measure round in the `lane3` worktree, on round 2135's failing `launch_path`. ADRs 3261 and
3262; no ledger row moves (14.9.2.3 and 14.9.4 keep `implemented`; their code changes, not their reading); no
question.

**The premise, corrected.** The brief named `373e7eb6` as the last commit inside the bands. It is not: round
2068 took them on `bf59984d` plus its own work, and main took seven audit-fix commits before `373e7eb6` landed.
Counted with the gate's own instrument (a script running the gate's `count-open` and `count-first-frame`
children under callgrind, summing this program's object), release builds read: `bf59984d` (under 1.100.0)
every figure inside, matching the band comments to 0.1 thousand; `373e7eb6` seven outside; HEAD the same
nine as 2135. No per-commit bisection was run: each fat-LTO build took 16 to 36 minutes at load 100 to 165,
so profiles of `bf59984d`, `373e7eb6`, HEAD and the fix were diffed by function instead, and each function that
grew was traced to its commit by `git log -S`.

**The regression (ADR 3261).** `2ef6887a`'s `structure::enclosing` walks `/P` to the root on every marked
sequence, copying each ancestor twice (`Document::get` returns a copy; `.as_dict().cloned()` another). Now an
`EnclosingMemo` on the interpreter keeps each element's answer, the walk stops at a remembered ancestor, the
parent is moved out rather than copied, and the bound stays exact by remembering each answer's chain length.
An imported page sets the memo aside. AN001 first frame 12 346.8 → 11 085.5 thousand, WTPDF 42 438.0 →
41 321.3 (back inside), EC3 169 902.8 → 169 113.2. Test: `a_remembered_ancestor_answers_what_the_walk_reads`.

**The bands (ADR 3262).** What remains is what the audit fixes read: §14.9.4's walk parsing EC3's top-level
elements, ToUnicode's code lengths, the whole AGL algorithm, a borrowed face in the outline key, the
object-stream map, and 8 to 10 thousand of open-time checks per document. Seven bands move by this file's rule
around this tree's figures, each comment citing ADR 3262. A one-pass `ToUnicode::own` was tried and measured
at +0.6 thousand on AN001 (no gain: an empty `collect` does not allocate), so it was reverted.

**Gates.** fmt 0; clippy `-D warnings --all-targets` over `-p pdf-font -p pdf-model` 0 (after two
`saturating_add`s); `pdf-sandbox --bins` 0; nextest `--lib --bins --tests` of the same set under `xvfb-run -a`
0, 3333 run, 3333 passed, against an empty manifest; rustdoc `-D warnings --document-private-items` 0;
conformance after `git add -N` 477 passed, 1 failed (this record's missing gates paragraph, then added);
`launch_path` 101: 36 banded, 1 outside, EC3's `peak_anon_mib` 21.457 against 20.5.

**Left.** EC3's `peak_anon_mib` (21.07, 21.92 in 2135, 21.457 here), likely the parsed ancestors held in the
object cache, is unmeasured and its band unmoved. `xfa_filled_imm1344e`'s 3.039 was one child of one 2135 run,
inside here: noise.
