# 2048 — The build and the agents, measured

Session 2048, an evaluation round in the audit worktree, asked by the owner on 2026-11-16 to find what
the build and the agents could do better. No ledger row; ADRs 2912 and 2913; Q876 and Q877; review
`doc/reviews/2048-the-build-and-the-agents.md`.

**The slice.** The brief's four questions — compiler and linker settings, gate time, token load,
anything else — each answered with a measurement or marked a hypothesis.

**The premise.** Two of the brief's premises did not hold. The required documents are not what a round
reads: over 21 round transcripts `state-of-play.md` returned 13 kB in all, and the reading list is about
8k tokens a round. And a build directory's size is not its libraries: the 522 `.rlib`s are 1.9 GiB of
75.5, the 825 test executables and 240 examples the rest, three quarters of their bytes debug sections.

**Built.** `[profile.dev] split-debuginfo = "unpacked"` (ADR 2912): the workspace's cold test build
from 75.5 to 45.1 GiB and from 16 043 to 12 974 processor-seconds; backtraces keep file and line.
`tools/round-tokens.py` (ADR 2913): the token census as a command.

**Found and proposed.** The post-merge train's first worktree had no submodules, so its first run
failed every gate in under 90 s; the orchestrator relinked them for the 20:30 run. Link-time zlib compression of debug sections, a further −58 % on `pdf-model`,
belongs in the machine's Cargo configuration, beside `mold`. `sccache` with a warm cache cut a cold workspace
build from 539 to 351 s for a 641 MiB cache. Rustdoc held the heavy-walk lock 47 % of all its hours
since 2026-11-05. A round's nextest builds examples it never runs (−28 % disk, −20 % processor time
on `pdf-model` without them, Q876); 584 integration-test binaries are the largest saving left
(Q876's pilot). Results over 4 000 characters are 54 % of a round's command output (Q877).

**Gates.** fmt; clippy `-D warnings --all-targets` and nextest over the workspace (9 970 passed, 73
skipped); conformance; no fuzz check (no public type changed); no launch gates (no release setting
changed). No `state.sh docs` and no tier-2 walk, by the owner's train rule.

**Trap or habit.** The harness moves a foreground command to the background after 180 s, so a
`tools/wait.sh` wall must be under it; the default 240 is not.
