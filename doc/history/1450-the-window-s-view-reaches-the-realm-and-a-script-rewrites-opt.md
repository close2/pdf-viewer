# 1450 — The window's view reaches the realm, and a script rewrites `/Opt`

Slot 1 of batch seventy-one, 2026-10-08, a script round. ADRs 1736, 1737; no row moved, no question.

**Premise.** Held for the four `Doc` members (ADR 1724 §3's reason, and `Request::Display` is the
action path's). Did not hold for the history pair: the viewer keeps no back/forward a person could
use, so `app.goBack`/`goForward` stay in `surface::REFUSED` with that reason (ADR 1736 §1).
`NOT_BRIDGED` held at 21 and is 21; `REFUSED` is two rows.

**Built (ADR 1736).** `ScriptEvent::view`/`Request::view` carry the window's `WindowView` with every
event; `viewer-core` tells it after each `settle`. `zoom`, `zoomType`, `layout` are read and written
and `scroll(nX, nY)` crosses in default user space; each write is `ScriptEdit::View`, held by the view
state (`take_view_requests`) and carried out after `carry_out_focus_requests`. Wire version 11.

**Built (ADR 1737).** `setItems`, `insertItemAt`, `deleteItemAt`, `clearItems` set
`Property::Options`, the whole list; the list box's construction, `form::fields`, `view::chosen`, the
realm's next telling and the save read it, a deleted selection cleared as an empty choice. Looked at
(trap 1): a list box rewritten to four items, saved and rendered, draws them with `Green` selected
at its new place.

**Found.** No `viewer-core` code calls `run_annotation_scripts` or `run_page_scripts`, and the action
path declines `JavaScript`: past the open, a button's mouse-up and a page's `/O`/`/C` scripts are
declined in every window (ADR 1736 §3). `doc/state-of-play.md` said they ran; corrected.

**For slot 6, and unfinished.** Wire version 11; `script_wire` re-seeded (`wire_seeds` now writes
a window view and the four view changes); new members for `seed_script.py`: the view four,
`zoomtype`, the four rewriters. `gotoNamedDest` still turns only the page.

**Gates.** rustfmt `--check` on my files and hunks: clean. `RUSTFLAGS="-D warnings" cargo clippy
--all-targets`: `pdf-model`, `pdf-script` (with and without `engine`), `pdf-script-worker` (`engine`)
exit 0; `viewer-core` fails only in slot 2's `notes.rs`. `cargo nextest run`: `pdf-script` 136,
`pdf-model` 2 004, the worker 26 (a deadline failure at load 20 passes at load 5), `viewer-core` and
the worker 364 passed, 0 failed. `cargo check --manifest-path fuzz/Cargo.toml` and `--workspace
--all-targets`: exit 0. `cargo test -p conformance`: 421 passed, 0 failed. Behind the lock,
`--tree 6`: the Tier 1 column exit 0 (21 101 runs, 11 778 finished, 9 315 threw, 8 unparsed, 0 over,
as HEAD; `HELD_THREW` lowered from 9 317 to 9 315 as its ratchet asked; 160 s, 1.40 GiB); the worker
column exit 0 (149 workers, 0 lost, 0 `SIGSYS`; 157 s); `pdf-model --test script_corpus` exit 0 (356
held, 0 moved); `save_round_trip` exit 0 (every floor held; 41 s); `pdf-model --test corpus` exit 0
(every ratchet at its ceiling); the `script_wire` re-seed exit 0 (32 seeds). Duration 7 000 s.
