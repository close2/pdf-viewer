# 1560 — A form's font is loaded once, a tick waits for its frame, and a refused method is a function

Slot 3 of batch eighty-nine, 2026-10-15, a script round. `doc/todo/70` items 2–5; ADRs 1956, 1957; no
question; no ledger row moves (§12.7.3's row gains the new tests and `script_sites.rs`).

**Premise.** Held for item 2 (603 `LoadedFont::load`s, 44.4 % of 2 330 M instructions on a framebuffer
witness rewritten from scratch by a script the round left in its scratch) and item 3 (`quorra` at 0 ms:
524 ticks, 0 of the changed page's frames in 20 s). Not for item 5's moving throws: no corpus script
reads a refused method without calling it. **Hypothesis.** Held, except "every window": `quorra-gtk`
and `quorra-qt` do not starve (697/698, 426/427 ticks/renders at 0 ms) — fields are controls and the
field-less page rasterises in about 2 ms on their own thread (ADR 1957 §3).

**Built.** (2) `variable_text::LayoutFonts`, the interpreter's, passed down `decide` → `construct` →
the widget, free-text and regeneration paths, keyed by the resolved `/DA` dictionary and label, bound
to its document, failures kept, at most 16 (ADR 1956): 603 → 6 loads. (3) `quorra` sends no tick while
an on-screen page's render is unacknowledged (ADR 1957 §1). (4) `report_unreachable` at a page's open
with a runner: Table 31's sentence for an annotation written into `/Annots` as a dictionary, §12.7.3's
or Table 224's for widgets with no form or no `/Fields` — `field_table`'s two early returns. Page one
of 944 corpus files (pikepdf, calibrated on doom.pdf's 234 and 234): 0, 1 (`issue12963.pdf`), 1
(`issue15597.pdf`), and our code says both. (5) `surface::Member` on every row (8 property rows); a
method answers a function that throws when called. Both new tests fail with the change undone (trap 13).

**Measured.** One `--clock` hold, load 14.1 → 4.6, its output whole in the round's scratch: the page interpreted
after a tick, HEAD's export 43.5/43.5/43.9 ms, this round's 15.6/15.7/15.5 ms (means of 20 ticks, fast
cores). quorra under Xvfb, 20 s: 0 ms, 53 ticks and 54 frames presented; 250 ms, 80 and 82.

**Files.** `crates/pdf-model/src/{variable_text,appearance,annotation,content,view}.rs`,
`content/annotations.rs`, `view/script_sites.rs`, `tests/script_unreachable.rs`; `crates/pdf-script/
src/{surface.rs,engine/bridge.rs}`, `tests/fixtures.rs`; `quorra/window.rs` (one hunk); ADRs 1956,
1957; `doc/todo/{70,README}`, `doc/state-of-play.md`, `doc/conformance/ledger.toml`; this record.

**Gates.** In an export of HEAD plus this round's hunks: rustfmt clean; clippy `-D warnings` exit 0 on
pdf-model, viewer-ui, pdf-script and the worker (engine), `--all-targets`; nextest exit 0 — pdf-model
2 067 passed/21 skipped, pdf-script + worker 202/4, viewer-ui 192/2. Six arms against
HEAD's six arms: 0 of 5 799 page digests moved. t2 script_corpus exit 0 (356 held, 0 moved);
t2 script_corpus_engine exit 0, every column at its ceiling. turn_path exit 0. launch_path exit 101:
bug1815476.pdf's cold open 0.577 ms against 0.320–0.500; in three A/B holds HEAD's export failed it too
(0.668 ms), and the open's instructions, bytes and calls agree to 0.01 %. conformance exit 0;
`tools/state.sh docs` exit 0. Drive: quorra's two timer steps work; doom.pdf at `--scripts on` says
both new sentences once, at `off` only Tier 0's.
