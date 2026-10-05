# 1512 — The oracle's held pages are printed from its constants, without the walk

Session 1338. Status: accepted. Context: ADR 1483 (whose departure a contradicted group holds, and
the next page to take). Code: `tools/conformance/src/held.rs`, `tools/conformance/src/bin/held.rs`,
`tools/state.sh` (`oracle-held`, in `all` and `quick`). Prose: `doc/HANDOVER.md` (the robustness
row).

## 1. The question

The oracle holds every page of every verdict but `agrees` by name, in about a hundred
`const <VERDICT>_<CAUSE>: [&str; N]` groups whose notes say why, spread over 16 600 lines.
`tools/state.sh oracle` runs the walk, which is a tier-3 gate behind the heavy-walk lock; nothing
printed the groups themselves, so a robustness round choosing a page read the test file.

## 2. The decision

`cargo run -q -p conformance --bin held`, run by `tools/state.sh oracle-held`, reads the file and
prints per verdict the held count, the groups by size (with their pages where a group holds three
or fewer), the contradicted pool's split by `WHOSE_DEPARTURE`, and the candidates for ADR 1483's
next page — every page of a group held as a departure of ours. It reads `N`, which the compiler
checks against each list, and never runs the walk.

**The section is `oracle-held`, not `oracle`**: `oracle` was already the walk, and renaming it
would take the gate out of `all`.

## 3. What it can and cannot say

On 2026-10-02 it reads 47 contradicted pages in 15 groups, 810 ambiguous in 68, 46 not comparable
in 9, 9 no render in 4, 2 reference geometry in 1 — and the last run counted 836 ambiguous. The 26
are not a parse error: the walk's ambiguous and contradicted ratchets hold only pages it rendered
completely, so a page drawn with a refusal is counted by the walk and held by no group. The order of
the two departure-of-ours candidates (`issue4436r.pdf`, `issue7891_bc1.pdf`) is a run's ranking and
stays the walk's to print.
