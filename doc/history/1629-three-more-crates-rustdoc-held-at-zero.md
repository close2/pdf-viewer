# 1629 — Three more crates' rustdoc held at zero

Slot 7 of batch ninety-eight, 2026-10-20, a docs round under the owner's switch to code improvement.
No ADR (2094 and 2095 not used): every answer is one ADR 1938 already gives, so its status line names
the three crates. No Q524. No ledger row moves; no clause is read anew.

**Premise.** Held: all three crates failed `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
--document-private-items` at HEAD. **The hypothesis did not hold in its counts or its kinds**: 151
errors, not about 54 — `pdf-font` 101 (45 bracketed letters, 40 public pages linking private items,
15 paths to items another module's privacy makes unreachable, one name that is a function and a
module), `viewer-device` 39 private links (round 1608 counted 43), `viewer-confined` 7 private links
and 4 unreachable or vanished names.

**Done.** The 136 of ADR 1938's two mechanical classes by a script reading the error log, each
edit asserted at rustdoc's own column and on a comment line: a letter escaped (`"\[t]he`), a private
or unreachable link a code span. The other 15 by anchored replacements: a reference definition where
the item is reachable by a full path (`crate::FontError::Type3`, `crate::cff::FontDictRepair`,
`crate::composite::collection_gap`, `skrifa::outline::{OutlineGlyphCollection::require_interpreter,
Target::Mono}`, `confined_transport::frame::MAX_MESSAGE`, `crate::Route::Window`, `fn@fold`), and
three definitions in `stale/plan.rs` naming private items removed with their links. `Marks`'s doc
named `Self::retain`, which `Marks` never had (`HeldLists` does); it now names `Self::place`, the
eviction it describes. `writing.rs`'s `/I` comment states the current reason and cites ADR 0253 and
`doc/errata-read.md`'s Table 234 row instead of what the line used to say. `tools/state.sh docs`
holds six crates.

**Files.** 30 in `crates/pdf-font/src/`, 7 in `crates/viewer-device/src/`, 3 in `crates/viewer-confined/
src/`, `crates/pdf-model/src/view/writing.rs`, `tools/state.sh`, ADR 1938's status line, this record.

**Gates.** The multiset of non-comment lines (habit 93) over the 41 Rust files against HEAD: 0 files
moved, the instrument shown a planted `u64`→`u32` first. In the export `r1629/docs`: `tools/state.sh
--round 1629 docs` exit 0 over the six; planted back — a `[t]he`, a `[`Job`]`, a
`[`message_budget`]` — it fails on each crate in turn (exit 101 twice); the public-only build of the
three exit 0. `cargo fmt --check -p pdf-font -p viewer-device -p viewer-confined -p pdf-model` 0; `-D
warnings` clippy of the four `--all-targets` 0; doctests of the three 0 failed. `cargo test -p
conformance --no-fail-fast` in the export: 55 binaries green, the 4 that read the git index fail
there for having none and pass in the worktree (32 passed); `--bin quotations`, `--bin pointers` exit
0. About 1 650 s.

**Unfinished.** None of the slot. `cargo test -p conformance --test names` fails in the worktree on
three `colour::` paths of the `pdf-colour` split under way beside this round, not here.
