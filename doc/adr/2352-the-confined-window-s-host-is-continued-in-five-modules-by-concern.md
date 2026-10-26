# 2352 — The confined window's `Host` is continued in five modules, one per concern

Session 1759, the standing code-quality slot of batch 115. Status: implemented; no behaviour changed, no bug fixed.

`Host` in `crates/viewer-ui/src/bin/quorra-confined.rs` was the largest impl block `tools/state.sh gods` named after
`Interpreter`: one block of about 1 300 lines and 32 methods in a file of 2 700 lines, whose tests and command-line
reader sat in the same file. **The block is not mostly one `match`**: `Host::event` is one exhaustive match (about 300
lines) and stays whole for the reason its `#[expect]` gives, but the other 31 methods are five different kinds of
thing, so `Host` is what was split and `Vfs` (the census's other candidate) is left for a later slot.

**What it is now.** One struct and one `impl Host` per module, the way `asking.rs`, `sending.rs` and `typing.rs`
already continued it:

- `worker.rs` — the worker's life: `start_confined`, `anticipate`, `take_the_opening`, `mind_the_opening`, `stop`,
  `died`, `reopen`, `abort`, `end_a_busy_worker` and `view`;
- `events.rs` — what the worker says and what each answer does to the window: `dispatch`, `event`, `declined_in`,
  `password_required`;
- `frames.rs` — the frames: `pull_frame`, `route`, `frame_landed`, `overlays`, `ask_frame`, `present`,
  `present_software`;
- `window.rs` — the window's own state: `bring_up`, `software`, `shown`, `redraw`, `retitle`, `name`, `card_changed`;
- `input.rs` — the keyboard and the pointer: `password_key`, `password_answered`, `click`, `key`, `press`.

`arguments.rs` takes the command line (`Arguments`, `arguments`, `USAGE`) with the two helpers every module says a
message or a path with (`brief`, `file_name`), and `tests.rs` the file's tests. The root keeps what a reader opens
first: the module header, the type `Host` with `new`, `Presentation`, `Opened`, the `ApplicationHandler` that is the
window's dispatch, `next_look` beside it, and `main`. The file fell from 2 684 lines to 695; it did
not reach the 600 the slot hoped for, because the type's fields (about 140 lines), its constructor, the handler
(about 250 lines) and the module documentation are what the root is for.

**Proof.** The multiset of non-comment lines, stripped of indentation and of a leading `pub(crate) `, taken over the
root and its seven new children against HEAD's file (script `split.py` with its anchors asserted, `proof.py`): no
line of code is lost but the imports the root no longer needs, the module-header `mod tests {` and an empty impl
block; the additions are `use` lines, `mod` declarations, `impl Host {` heads and their closing braces. A method
called from a sibling module became `pub(crate)`, found by the compiler's `E0624` and not by a list; the 18 tests
moved with their imports (the ledger rows that named `quorra-confined.rs::` for three of them name `tests.rs::`).
The drive of the confined window (`tools/drive-windows.sh --window quorra-confined`, 51 steps) on an export of this
tree and on HEAD's binaries gives the same verdict on every step: 49 works, 1 wrong and 1 not offered in each, the
`wrong` being step 66 on both.

**Choices.** The methods keep their names, so no document that names `Host::…` needed editing; `Opened`,
`start_worker` and `Presentation` stay in the root because the constructor and the handler read them and a type
moved away from its only user is a longer hop for the reader. `next_look` stays beside the handler it times.

**Gates.** See `doc/history/1759`.
