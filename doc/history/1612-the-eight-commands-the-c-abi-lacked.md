# 1612 — The eight commands the C ABI lacked

Session 1612, slot 6 of batch ninety-six, a host round; 2026-10-20. ADRs 2060 (the eight in C's
shape) and 2061 (a C caller's documents are handed a script runner). No Q file: no owner's word
was needed.

**Built.** RFC 0010 section 3.1's eight `Command`s reach `crates/viewer-ffi`: `quorra_commit_field`,
`quorra_clock`, `quorra_respond`, `quorra_oversize`, `quorra_keys`, `quorra_scripts`,
`quorra_answer_scripts`, `quorra_answer_script`. Beside them, the thirteen that make each usable from
C: `quorra_data_format_of_media_type` and `_of_name`; `quorra_event_submit`, `_submit_url`,
`_submit_media_type` (and `quorra_event_bytes` now gives a submission's body, as the header already
said it did); `quorra_event_script_question` and `_script_text`; `quorra_render_request_oversized`,
`_window`, `_rasterise_window`, `quorra_render_ready_listed`, `quorra_raster_info` and `_copy`. 240
to 261 entry points, 34 constants, no event kind (the count stays 30), `QUORRA_ABI_VERSION` 2.

**The premise and the hypothesis.** The premise held: 37 of 45. The hypothesis did not, in two
places: no event kind was needed (kinds 22–27 already existed, documented as "sent to no caller"),
and eight entry points were not enough — a `SUBMIT` crossed without its URL or body, a script's
question only as a sentence, and a window's raster could not be read. `Oversize` was priced as
waiting for section 4's device; the processor serves it now, since the request already is the list.
`Scripts` was priced as needing a runner made beside the library; `viewer_host::policy::scripting`
is that runner and every window's.

**Unfinished.** RFC 0010 N1's key table and its count; a submission's `owed` sentences do not
cross. The ABI table the brief names in `doc/ui-boundary.md` is prose at the `viewer-ffi` entry,
edited there.

**Gates.** `rustfmt --check` on every touched file and `cargo fmt --check -p viewer-ffi`: 0.
`RUSTFLAGS=-D warnings cargo clippy -p viewer-ffi --all-targets`: 0 (an earlier run stopped at slot
3's `header_and_interop_agree.rs`, since fixed there). `cargo nextest run -p viewer-ffi`: 0, 46 of
46, the C program among them (`-Werror`; the `/M` it saved, the server's value, the window
1201 × 1501 of a 23812 × 33676 page). `header_and_interop_agree`: 4 of 4 against the new header.
`tools/state.sh hosts`: Command 45 of 45, Query 36 of 36, 261 entry points. `cargo test -p
conformance --no-fail-fast`: 101, four failures all in siblings' files (`viewer-ui`'s
`software.rs` and `renderer.rs`, `viewer-device`, `render-raster`, record 1614). The four windows:
`cargo tree -i viewer-ffi --workspace` names no dependant, so no window's graph moved (habit 119).
Rustdoc `-D warnings` on `viewer-ffi`: 43 errors, none in a line this round wrote; the crate is not
held at zero. Duration: about 2 100 s.
