# 683 — The text that could not leave, and the four platforms it now reaches

Date: 2026-08-23. ADR [0519](../adr/0519-a-selection-that-can-leave-the-program.md).

Touched: `crates/viewer-host/src/copying.rs` (new) and `lib.rs`;
`crates/viewer-ui/src/clipboard.rs` (new), `lib.rs`, `Cargo.toml`, and
`src/bin/pdf-viewer{.rs,/app.rs,/typing.rs}`; `crates/viewer-gtk/src/host.rs`;
`crates/viewer-qt/src/{bridge.rs,host.rs,keys.rs}` and `cpp/window.cpp`;
`crates/viewer-ffi/src/{abi.rs,kinds.rs,lib.rs,session.rs}`, `include/pdf_viewer.h`,
`c/open_a_page.c` and both header tests; `Cargo.toml`, `Cargo.lock`, `deny.toml`;
`doc/conformance/ledger.toml` (§14.8.2.5), `doc/todo/30-a-native-host.md`, `doc/stack.md`,
`doc/state-of-play.md`, `doc/environment.md`.

The second round on the project owner's *"even though low priority, I think we should start
investing time into the UI (and its API for the native versions)"*, taking item 1 of the ordering
ADR 0509 wrote.

## The item

No host could copy a selection out of the program. All three windowed hosts drew one; `viewer-ui`
kept an in-process `String` whose own comment said a native host "owns that end by construction",
and neither native host had taken it.

**The claim it was ranked on holds and was checked rather than assumed: no new message.** Nothing
this round added a `Command`, an `Event`, a `Query` or an `Answer` — `Query::Selection` and
`Query::LogicalSelection` have both answered since the three-hundred-and-eighty-eighth session. That
is the seventh time since the six-hundred-and-seventh that a feature landing in every host needed no
channel, and the first where the capability reaches outside the program.

## What the briefing was wrong about, and it was worth checking

**"The C ABI has `pdfv_selection_text` already"** (ADR 0509 §3). It does, and it answers in **page
content order** — the order the quadrilaterals are in, which is right for drawing and wrong for
copying off a page whose producer wrote its columns out of order. `Query::LogicalSelection` reached
no entry point at all: it was one of the twelve `tools/state.sh hosts` names. So the fourth consumer
could copy and could not copy *right*, and a fifth of item 5 came with item 1 because item 1 is not
finished without it. `pdfv_selection_copy_text` is the new symbol and the count is 20 of 31 now.

**And `viewer-qt` could not call `QClipboard` from Rust** — this tree's shape rather than Qt's.
`crate::bridge`'s own documentation states that C++ owns the `Host` for the life of
`QApplication::exec` and Rust never calls a Qt object, which is what keeps the crate to one
hand-written `unsafe` token with a test on its position. The copy goes out as a `QtUpdate` flag with
`take_clipboard` beside it — the shape the `window` flag has had since ADR 0470 — rather than as a
second declaration in the `unsafe extern "C++"` block.

## The decision, and the four platforms

`viewer_host::copying` is §14.8.2.5's choice made once: the logical content order where the
structure tree reaches every byte, page content order otherwise, nothing at all where nothing is
selected, and the order named out loud either way. `viewer-ui` lost a private function to it, which
is what distinguishes this from a fourth copy — the same sentence `Clock` and `Presenting` carry.

The platform ends are `gdk::Clipboard`, `QClipboard`, `arboard`, and, for a C caller, its own.

## The one dependency, priced before it was taken

`arboard` 3.6.1, `MIT OR Apache-2.0`, and **one compiled package on this platform**: `cargo add`
locks four and three are `cfg`-gated to Windows and macOS, while its Linux dependencies were already
in the lockfile through `winit` and `softbuffer`. `image-data` is off (a page copied as a picture is
a feature nobody asked for) and `wayland-data-control` is off — six more packages including a MIME
sniffer and a graph library, for a wlroots protocol rather than a Wayland one; without it the X11
backend reaches a Wayland session through XWayland, and where there is none the copy is **refused by
name** rather than silently dropped.

`cargo deny check licenses` failed on it and the fix is narrow: `clipboard-win` and `error-code` are
`BSL-1.0` and are named as exceptions rather than added to `allow`, with the reason that neither is
ever compiled here and that a Windows build *would* link them.

Nothing is on the launch path: `Clipboard::new` is a `const fn` that connects to nothing, and
`connected()` makes that a property a test asserts.

## What was measured

**Byte-identical across all four consumers.** `PDF20_AN001-BPC.pdf`, whole page selected, copied,
and read back off the X11 `CLIPBOARD` selection with `xclip` under `Xvfb` — `2f5c6578…`, 173 bytes,
170 characters, `§14.8.2.5's logical content order` in each host's own note. The C caller is a
throwaway program compiled against `target/libviewer_ffi.so` and `include/pdf_viewer.h` with
`-Wall -Wextra -Werror`, and it reports `PDFV_ORDER_LOGICAL` for the same bytes.

**One instrument fact worth keeping**: `xdotool key --window <id>` sends a synthetic event that
**Qt ignores** and GTK accepts, so the first Qt run copied nothing and looked exactly like a binding
that does not work. Plain `xdotool key` goes through XTEST and reaches both. The tell was an empty
clipboard beside a log with no `copied` line in it.

`crates/viewer-ffi/c/open_a_page.c` also demonstrates the entry point and reports the *other* order
for the same document, because the program has turned to page 2 by then and that page's structure
tree does not reach every byte — the documented refusal working rather than a defect.

## What was run

`fmt`, `clippy --workspace --all-targets` under `-D warnings`, `nextest --workspace`, the workspace
doctest, the fuzz targets' `check`, `cargo test -p conformance`, and `cargo deny check licenses bans
advisories` because the round took a dependency. The change map puts all five host crates under the
core and the ledger under the conformance gate; `viewer-core` was not touched, so neither census is
owed. The `pointers` and `quotations` sweeps were run because the round moved documents; neither
names anything this round wrote. §5's binaries were rebuilt and installed before any of the window
work, which the round owed before measuring.

One gate caught a real mistake and is worth naming: `every_citation_names_a_clause_that_exists`
refused five `CLAUDE.md §2` citations across three files — a `§` is checked against ISO 32000-2's
clauses and would have passed by landing on one.

One thing this round got wrong and `doc/environment.md` now carries: **`git add -u` springs the
submodule trap exactly as `git add -A` does**, and that paragraph named only `-A`. The new files
were added by name, as the rule says, and the rest went in with `-u` — which is the same
consequence by a different route, because a gitlink whose working-tree entry is a symlink is a
modification to a tracked path. `git commit` printed `mode change 160000 => 120000` and
`every_declared_submodule_is_still_tracked_as_one` failed, which is what that gate is for. The
repair is that test's own loop **minus its `rm -f`**, which in a parallel worktree would delete the
symlinks the tree reaches `doc/pdf.js` through.

**Nothing is queued for the owner's measurement loop.** A clipboard under `Xvfb` is a real X11
selection with a real owner, and `xclip` reads it back from a second process; none of this needs the
real display or the real adapter.
