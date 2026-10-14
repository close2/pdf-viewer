# 1537 — The renderer and the stand-in are module trees, and a turn's sharp present is owed

Slot 4 of batch eighty-five, 2026-10-14, a loop round under the owner's switch to code improvement.
ADRs 1910 (the split) and 1911 (the turn); no question; no ledger row moves.

**Premise.** Held. `wc -l` printed 3 569 and 3 686. Round 1532's turn figures are in
`tmp/r1532/owner/`: the stand-in 1 ms after the key, the frame at 16 ms, a second present at 44 ms.
**Hypothesis.** Failed. The 44 ms present is ADR 0699's sharp pass, not the stand-in, and
`--supersample 1` breaks it: on Xvfb a turn then settles at its first change (AN001 29–35 ms against
63–84; WTPDF 24–34 against 54–102). The stand-in is the instrument's first change, okular's
figure. The frame missed a refresh (two at 120 Hz). No page has a `/Trans`, so no ADR 1853 face.
Nothing is taken out (ADR 1911). A landed frame waits up to a tick for want of a wake (§5).

**Built.** `renderer` is now a root and nine children, `stale` a root and five, by concern (ADR
1910's tables). Callers' paths are kept by re-exports, and `PROXY_EDGE` moved to `stale`'s root.
45 `pub(super)`, each needed: taken away one by one, every one fails to compile. The todo
pointers in `doc/todo/47` and `/50` name the children.

**Proof that nothing moved.** As multisets of lines, HEAD's two files and the trees differ only
in imports, visibility, re-wrapping, the canvas tests' qualifications and comments. The instrument
names a planted constant. 187 tests before and after, the same functions, all pass. The drive
(`quorra` and `quorra-confined` whole, HEAD's tree and this round's) gives the same verdict on all
112 steps, 106 works and 1 wrong. Observations differ only in paths, ports, and two sampled
progress shares and frame counts. rustdoc prints the same 24 warnings.

**Found, not fixed.** The drive's one `wrong` is HEAD's: `quorra-confined` says "cannot be drawn
at this size" at ten notches past the bound (step 82), once per magnification as ADR 1892 §2.4
says, where the drive allows one. **Unfinished.** The owner's display: every window presented, but
the session was locked (`LockedHint=yes`, no visible window), so the before and after is Xvfb's.

**Files.** `crates/viewer-ui/src/bin/quorra/{renderer,stale,surface}.rs`, `renderer/` (nine),
`stale/` (five); `doc/todo/47`, `doc/todo/50`; ADRs 1910, 1911; this record.

**Gates.** Tier 1: rustfmt clean on every touched file. `clippy -p viewer-ui --all-targets -D
warnings` exit 0; `nextest -p viewer-ui` exit 0, 187 passed (worktree and export). `cargo test -p
conformance` in the worktree: exit 101, one test, which reads a sibling's removed
`pdf-model/src/image/cut.rs`; the names, ledger, records and cited tests pass. One `--clock` hold of
1 094 s held `launch_path` head, mine, head, mine: exit 0, 0, 0, 101. The 101 is AN001's cold open
at 0.837 ms against 0.830, with the open's instructions identical (3 974.07–3 974.13 k). `turn_path`
exit 0, every row inside. Drive exit 0 twice. Xvfb turns exit 0. Duration 10 325 s.
