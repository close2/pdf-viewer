# 1583 — A page's walk from bytes to pixel is written down, and its names are held

Slot 7 of batch ninety-two, 2026-10-19, a docs round (Sonnet 5.5). ADRs 2002, 2003; no question.
No code changed and no bug was fixed (the owner's rule); each one met is below.

**Premise.** It held: `grep -rl mermaid doc` was empty, and no navigational document answers *in what
order*. 
**Built.** `doc/teaching/how-a-page-lands-on-the-screen.md`: one running example (`q`, `rg`, `re`, `f`,
`Q`), seven stages (`pdf-syntax`; the page tree; the content interpreter with `pdf-colour` and
`pdf-font`; `pdf-render`'s list and trait; `render-cpu`, `render-raster` over `raster/`, `render-gpu`;
compositing; the window), eight Mermaid diagrams (one for the whole), a Maintenance note after each,
and a closing section for the round that changes the code. Four sentences of ISO 32000-2 are quoted
(§7.8.2 twice, §7.5.4) and the sweep finds them verbatim. `tools/conformance/tests/teaching.rs` holds
every path, every `path::name` and every `ADR NNNN` the document writes to something that exists, every
diagram to its note, and `doc/HANDOVER.md` and `doc/crate-map.md` to a pointer (one line each added).
Calibrated: a text with four planted faults (a missing file, a missing name, a missing ADR, a diagram
with no note) is reported four times; the mended text, none. The first run found two errors in the document; both were fixed.

**Proof that nothing moved.** No `.rs` under `crates/` or `raster/` was touched; the files are below.

**Found, not fixed.** (1) `crates/render-raster/src/lib.rs` line 2 says the library is "built at
`../render-lib`"; it is `raster/` in this workspace. (2) `raster/crates/raster/src/lib.rs` and
`raster/crates/raster-gpu/src/lib.rs` open with a "# State" paragraph naming milestone M1 and
"analytically-covered axis-aligned rectangles" for a crate that has glyph, path, line, compute and
retained lanes: a status sentence that is no longer what is. (3) `crates/render-cpu/src/lib.rs`'s first
job, "It renders when no usable GPU is present", is not how `CLAUDE.md` principle 2 words the backend's
jobs (a refused frame, and the faster lane for a page past the bound). (4) At this round's end the
worktree holds siblings' work in progress that fails two conformance tests: `batch.rs`
(`tools/state.sh:259` runs `python3 -I -B tools/god-census.py` without `PYTHONDONTWRITEBYTECODE=1`) and
`records.rs` (`doc/history/1581-…` is 44 lines against 40). Neither is this round's.
(5) Slot 8 is splitting `crates/viewer-core/src/viewer.rs` into `viewer/`; the document names
`viewer.rs::handle`, and `teaching.rs` will fail if `handle` moves out, which is the test working.

**Files.** `doc/teaching/how-a-page-lands-on-the-screen.md`, `tools/conformance/tests/teaching.rs`,
`doc/HANDOVER.md`, `doc/crate-map.md`, ADRs 2002 and 2003, this record.

**Gates.** `cargo test -p conformance --test teaching`: 3 passed. `--no-fail-fast`: every binary ok
but the two siblings' above (exit 101). `--bin quotations` and
`--bin pointers`: exit 0, no line naming this round's files. `tools/state.sh docs`: exit 0. About
2 500 s.
