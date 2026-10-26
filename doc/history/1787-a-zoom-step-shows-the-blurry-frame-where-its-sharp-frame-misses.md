# 1787 — A zoom step shows the blurry frame where its sharp frame misses the refresh

Session 1787, slot 9 of batch 118, a "loop" round added at the owner's word; ADR 2408; ADR 2409 unused; Q653 unused.
No code changed; no ledger row moved; no pixel or figure a gate holds moved.

**Question.** The owner's of 2026-10-26: is the blurry frame a zoom-out always shows necessary, or a bug? Verdict
(ADR 2408): necessary, not a bug. No defect was found on the in/out paths: the same lane, the same prediction rule,
every sharp frame presented on landing, no notch dropped.

**Measured.** quorra on the Radeon 890M, natively on Wayland in a nested headless KWin of this round's own at the
owner's 2880 × 1800, scale 1.6, Control+wheel through KWin's EIS input, HEAD's build, one `--clock --nice 0` hold
(loads 0.95 to 2.98, the render node held by none of this account's processes between arms, `pp_dpm_sclk` at its
middle level before and after each arm). Notch to sharp frame against 8.33 ms: a simple page lands 2 to 6 ms out
(12 of 12 inside); ISO 32000-2 p. 101 alone 5 to 10 ms (14 of 24 inside); pp. 101–102 as the `OneColumn` it opens in
8 to 24 ms (2 of 24), every step rebuilding and walking its scene because page space is one page (ADR 0702), and its
stand-in the 512-pixel retained pages alone because the 8-logical-pixel gap puts the second page 1.5 to 4.0 px off
one placement; a heavy page 12 to 51 ms (0 of 24). `quorra-qt` stands in at every notch by design (ADR 1843), its
first picture of the new size 3 to 18 ms after the notch. The asymmetry is geometry: what a zoom-out reveals has no
picture sharper than the retained page.

**Hypothesis.** "A zoom-out takes a path or prediction that defers to the stand-in where the sharp frame is cheap" did
not hold: where the sharp frame is cheap (one page) neither direction stands in; where it stands in, the sharp frame
missed. The premise ADRs held as read, and ADR 0702's one-page limit is what a column's cost comes from.

**Levers, priced and not built** (ADR 2408 §4): a column in page space (ceiling: 14 of 24 steps inside, the one-page
figure); a column's stand-in carrying the last frame page by page (no frame faster, the blur gone in a column); the
toolkit windows waiting one refresh before standing in (ceiling: 9 of 12 notches unstood-in; ADR 1843's choice is the
owner's to re-open).

**Instruments.** The session worked in `scratchpad/r1787/`, with an Xvfb display and a nested KWin of its own; the
Xvfb was stopped with TERM and left no lock or socket. Under Xvfb quorra gets llvmpipe (no DRI3), so the GPU
figures are the nested KWin's only.

**Gates.** No crate touched, so tier 1, the turn gate and the drive of the windows owe nothing. The hold: exit 0
(351 s), every arm's drive exit 0. `cargo test -p conformance --no-fail-fast` exit 0 (69 targets, 466 passed, 113 s;
its first run was red on `evidence` for this record's own scratch sentence, which now says where the session worked).
