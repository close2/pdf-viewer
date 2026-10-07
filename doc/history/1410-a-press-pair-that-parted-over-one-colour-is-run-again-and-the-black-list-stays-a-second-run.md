# 1410: a press pair that parted over one colour is run again, and the black list stays a second run

Pixels slot, batch sixty-four. ADRs 1656 and 1657; no ledger row moved, no question.

**Premises.** ADR 1645 section 3's site list (8 `colour` calls, two conversions, the image plan, the
overprint arms, 33 reads of `self.compositing`): held as a count, but incomplete — `blend_at_the_do`
decides a knockout construction on resolved colours, compared per plane. The 0.1% line was measured
first, as the brief asked.

**Measured, not built** (ADR 1656): a probe of the carrying's shape in an export of HEAD, callgrind by
function, one thread, md5-distinct exports. One binary measured twice moves the totals by up to
0.08% (allocator, `HashMap` seeds), `pdf_model`'s functions by 0, so the tax is read off those: ISO
32000-2 p101's turn +0.015%; the launch rows' page one −0.010 to +0.010% on four, +0.118% and +0.122%
on `opt_demo.pdf` and `xfa_filled_imm1344e.pdf` (about ten instructions an emitted command). And a
derived list would inherit each knockout construction from the chromatic plane silently.

**Built** (ADR 1657): `0 0 0 0.3 k` / `0 0 0 0.7 K` under `/BM /Darken` in a `/DeviceCMYK` group is one
colour to the chromatic run and two to the black one, so the pair was given up with a report on
valid input — a `B`, a form's knockout group, a soft mask's group (four fragments; one-colour and
`/Multiply` twins keep their pairs). `OneColour` counts the admission; a pair or mask pair that parts
after one is run again with it refused. Looked at (trap 1): the `B` over cyan draws backdrop plus 30%
and 70% black, as `Darken` on colorant amounts does; the one-colour twins are byte-identical to HEAD.
Cost: `pdf_model` +148 instructions on p101's turn, −11 to +169 on the launch rows, +10.5 k of
1 260.1 M on `bug1721218_reduced.pdf`'s turn.

**Unfinished.** The exact form, by ADR 1656. The page's three rows were not re-taken: nothing built
reaches its turn (callgrind above; `turn_path` read 146.93 and 147.50 ms inside its band).

**Proposed trap.** An export under a worktree inherits its `.cargo/config.toml` and builds into the
batch's shared target, fresh to a sibling's next build: pass `CARGO_TARGET_DIR` (this round's first
probe build did not; its `pdf-model` unit was deleted from the shared directory at once).

**Gates.** `rustfmt --check` on the six files: 0. On HEAD's export with the change applied (the
worktree holds siblings' mid-edits): clippy `-D warnings -p pdf-model --all-targets`: 0; `cargo
nextest run -p pdf-model`: 0, 1957 passed; both new tests watched failing with the retry planted
out. `cargo test -p conformance` in the worktree: 382 passed, 1 failed — `records.rs` on records 1409
and 1412 being written by their rounds. Behind the lock (queued 1 034 s, held 1 655 s): six arms
from one change binary, all exit 0, digests by page against `/home/AI/arms-1408/` 0 moved of 968 /
968 / 968 / 964 / 963 / 964; `raster_golden`: 0, held 974, moved 0, in 16 s; `pdf-model --test
corpus`: 0, in 7 s; `turn_path` twice: 0 and 0, 33 of 33 judged, 0 outside, in 94 s and 95 s.
