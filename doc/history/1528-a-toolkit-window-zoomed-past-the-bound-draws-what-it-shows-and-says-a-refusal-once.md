# 1528 — A toolkit window zoomed past the bound draws what it shows, and says a refusal once

Slot 1 of batch eighty-four, 2026-10-14, a host round on the owner's "endless rejection loop". ADR
1892; no ADR 1893, no question. No ledger row moved: the target rules, and no clause is cited.

**Premise.** Half held: the whole-page request and `MAX_PIXELS = 1 << 28` did; the refusal named did
not — `Event::Refused` is a restriction, and the sentence was `schedule`'s `Event::Reported`, pushed
at every settle. **Hypothesis.** The crossing held, "six times its fit" depends on the window (the
twelfth notch at 1594×1718 on the owner's display, the fifteenth in Xvfb's). The loop was the
pointer: HEAD said the refusal 51 times in ten idle seconds on the owner's display with the pointer
over the window, once without it; under Xvfb 30 pointer moves added ten refusals (Qt), three (GTK).

**Built** (ADR 1892). `Command::Oversize(Listed)`: a page past the bound is drawn as the window shows
it — the viewport and a quarter round it (`Drawing::ask_window`, `viewer_host::oversize`), answered
`Rendered::Listed`, asked again when a scroll leaves it; under the bound nothing moves. The scheduler
says a refusal once per magnification and interpretation, for every host. The events trace formatted
a 718 MB raster whole to keep 120 characters (8–11 s of Qt's thread a frame); it stops at 120.

**Measured** (one `--clock` hold, `scratchpad/r1528/hold.sh`; the witness is the original
github.com/cyfung1031/BergPDF `sample/demo_heavy_engineering_A1.pdf`, md5 `166bb8be…`). Xvfb, 18 `=`
notches 300 ms apart, two runs an arm: **HEAD never drew the last notch** (last frames 17594×12428 Qt,
17309×12227 GTK; 8 and 16 refusals with 30 pointer moves); **this round's window of it landed
100–103 ms (Qt) and 108–112 ms (GTK) after it was asked, 0 refusals**. Owner's display: only the
hold's first run was pressed; seven windows were not activated (before: `tmp/r1528/repro*`).

**Unfinished.** `launch_path` (its build queued behind three holds; the launch gains one O(1)
command); the owner's-display after figure; pages under the bound are still drawn whole (ADR 1892 §4).

**Files.** `viewer-core` `src/{command,lib,open,viewer}.rs`, `tests/headless.rs`; `viewer-host`
`src/{drawing,lib,trace,oversize}.rs`, `tests/oversized_pages.rs`; `viewer-{gtk,qt}/src/host.rs`;
`viewer-confined/src/protocol.rs`; `quorra/trace.rs`; `tools/drive-windows.sh`; ADR 1892;
`doc/{ui-boundary,state-of-play}.md`; `doc/todo/30`; this record.

**Gates.** `rustfmt --check --edition 2024` exit 0 on every file; `cargo clippy` of the six crates
`--all-targets` exit 0, none in this round's files (`-D warnings` unusable: slots 2 and 4 warn
mid-edit); `cargo nextest run -p viewer-core -p viewer-host` 641 passed, exit 0; `-p viewer-gtk -p
viewer-qt -p viewer-confined` 155 passed, exit 0; `cargo test -p conformance` exit 101 on `records`
(slots 3–5's records), none in this round's. Drive, export, steps 82 and the two new in four windows:
every verdict works or not offered; the confined window's new step hit the 6 GiB tree ceiling and its
rerun timed out in the queue. Clock hold exit 0, 875 s. `launch_path` not run. Duration 8 700 s.
