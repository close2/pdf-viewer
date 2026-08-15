# 532 — The release that answered an ask, and the counter a retained frame was missing

2026-08-15. One round, taking a quorra release. ADR 0367 is the decision; this is the record.

## What was taken

`580fa4ac` → `a64a9084`, twenty-eight commits. Upstream's four ADRs 0049–0052 plus ADR 0053,
which is the answer to `doc/QUORRA_FUNCTION_PAINT.md`. The range is tabled in
`doc/QUORRA_UPGRADE.md`'s `a64a9084` section, one line per commit.

Two documents arrived with it, written from the renderer side and now in this tree:
`doc/QUORRA_API_2026_08_15.md` (the migration, the first bump to arrive with one) and
`doc/QUORRA_FUNCTION_PAINT_ANSWER.md` (the reply to the ask of the same date).

## What it cost to compile: nothing, for the third release running

`cargo update -p quorra-gpu -p quorra-scene`, two hashes, then `build --workspace --all-targets`
clean and `clippy --workspace --all-targets` silent with no source touched — including across
upstream's ADR 0051, which split `scene.rs` into seven files, `compose.rs` into six and
`winding.rs` into four.

## What was adopted, and what was declined

`FrameCost` gains `atlas_repacked` and `atlas_working_set_bytes`; the frame line gains a
`repacked` word appended only when true, the legend gains its row, and `Timing::summary` gains a
line beside the replay count. `Timing::retention` is that paragraph extracted, because the new
line took `summary` past `clippy::pedantic`'s hundred lines.

`clip_residue_regions` and `clip_residue_tiles` were declined with the cost written down: they
answer a census question and the window's per-frame line is the wrong instrument for one.

## The four lanes, A/B in one working copy, `Cargo.lock` the only variable

| | `580fa4ac` | `a64a9084` |
|---|---|---|
| scale 1, `cpu` | 930 / 24 / 2 / 18 | 931 / 23 / 2 / 18 |
| scale 1, `gpu` | 928 / 26 / 2 / 18 | 929 / 25 / 2 / 18 |
| scale 4, `cpu` | 936 / 10 / 5 / 23 | 936 / 10 / 5 / 23 |
| scale 4, `gpu` | 937 / 9 / 5 / 23 | 937 / 9 / 5 / 23 |

Page by page: `issue2177.pdf` leaves the differing list on both scale-1 lanes (1.1168 / 7.14 on
`cpu`, 1.0992 / 7.14 on `gpu`, then absent); `issue11473.pdf` moves 0.1003 → 0.1004 mean and
10.04 → 10.07 worst tile on both and stays listed; `issue6081.pdf` at 4× moves 9.17 → 8.86 on the
worst tile on both lanes and stays listed. Every other differing page is identical to the
character, and no refusal moved at any scale on either lane. `DIFFERS_AT_THE_EDGES` is 6 names
from 7.

The 1× `gpu` base had to be measured rather than read: ADR 0351 records 933 agreeing there and the
honest base is 928, because ADR 0355 and 24a moved pages in the two sessions between.

## The retained frame still replays

`crates/render-quorra/tests/retained_frame.rs`, eight tests, green in the workspace run.

ADR 0351's frame-structure check re-run at this pin: `Xvfb :132` at 900×1100, llvmpipe, the
release binary of this tree, `tmp/Entwurf.pdf` (58 009 commands) opened with `--trace`, 24 presses
of `Up` with the page already at the top, one run. **Structure only; the machine was shared and no
wall clock from it is a claim.**

- `24 of 25 frame(s) replayed a retained encode` — ADR 0351's count exactly.
- `58029` resource uploads, all in the first frame — ADR 0351's number exactly.
- `the handle held at most 3830032 byte(s)` — ADR 0351's number exactly, to the byte.
- `the atlas was repacked after 0 of them; the busiest frame's glyph tiles wanted 526098 byte(s)`
  — the new line, and the two numbers say why nothing was repacked: half a megabyte of working
  set against the default atlas budget.
- Medians: `scene` 0.0, `encode` 0.0, `settle` 0.0, `fallback` 0.0 — the shape ADR 0351 measured.

## Gates

`cargo fmt --all --check` silent. `cargo clippy --workspace --all-targets` silent of lints (the
`proc-macro-error2` future-incompatibility note is cargo's and pre-existing; the `viewer-qt@`
`-Wmaybe-uninitialized` lines are gcc's on a cold build, as `doc/todo/02-every-round.md` §2
records). `cargo nextest run --workspace`: 1946 passed, 15 skipped. `cargo test --workspace
--doc`: green. `conformance`: green. Corpus: 974 documents, 0 unopenable, 8 locked, 2 encrypted
beyond us, 6 pageless, 64 incomplete, 0 slow. Oracle: green. `text_extraction`: 4 passed. Dates,
XMP, JPEG 2000: green. Four quorra lanes as tabled above, plus their base arms.

Release binaries rebuilt and installed into `target/` (§5), the FFI library included.

## What this round did not do, deliberately

- **The function paint is not built.** The answer is yes and its price is in
  `QUORRA_FUNCTION_PAINT.md` §8; what it converts is the meaning of the corpus gate, which is a
  round with its own ADR.
- **The two `pdf-model::function` defects upstream found are not fixed** — `Operator::Round`'s
  rounding direction and `Operator::Eq`'s epsilon, both against PLRM3 through §7.10.5.2. Each
  changes what a function evaluates to, and this round's whole evidence is that one page moved
  and that upstream's border cut is why.
- **Upstream's two asks stay open**, both now one piece of work: the rectangular-fill census and
  the residue-clip distribution, over the corpus rather than over a frame.
