# 1654 — `Encoder`'s list-to-scene walk split by concern

Session 1654, slot 8 of batch one hundred and one, a docs refactoring round (Sonnet 5.5); 2026-10-20.
ADR 2144 (2145 unused). The owner's words: "Refactor god classes,..." and "Do not fix any bugs." None
was fixed. `doc/todo/74` entry 10 (new): `impl Encoder` in `crates/render-raster/src/scene.rs`.

**Built.** `scene.rs` (2 071 to 391 lines) keeps `Encoder`, the shared types (`ClipOutline`,
`ResolvedClip`, `Admitted`, `ShadedPaint`, `GroupParts`, `FunctionPaints`), the constructors and
`commands`; six modules under `scene/` hold `impl Encoder` by concern: `clips` (347 lines), `groups`
(316), `fills` (180), `shadings` (530), `images` (146), `outlines` (103); `vocabulary` (169) holds the
free conversions and is re-exported under the paths `present.rs` and `stroke.rs` already used.

**The premise that did not hold.** `doc/todo/74` had no unbuilt numbered entry (1 to 9 built, 10 and 11
are files and functions); the next cut was the census's own ranking, and the largest block outside
`pdf-transform`, `pdf-sandbox` and `pdf-archive` (slots 1, 5, 6 own them) was `Encoder`, which the
file had never named. Entries 10 and 11 became 11 and 12.

**The proof that nothing moved.** Line multiset of the old file (indent removed) against the root and
the seven new modules: removed-only 15 lines (eleven signatures, the old import list, `self::compose`),
added-only 102 non-blank (module docs, `use`/`mod`, six `impl` wrappers, the widened signatures,
`super::compose`). `GroupParts`'s doc comment sat above `refuse_untranslatable_group`; it now sits on
the type (a move, not an edit). Eleven items are `pub(super)`, each narrowed back one at a time
(habit 137). `render-raster` is in the arms' graph: the six arms on an export of this tree, compared by
page against `/home/AI/arms-1647/`: 968, 968, 968, 965, 965, 965 pages, 0 digest differences in each.

**Ledger and docs.** Fifteen rows re-pointed to the module holding the row's item; the teaching walk,
its diagram and `doc/crate-map.md` name the new files; three doc links re-qualified (habit 126);
`image_residual.rs` now names `scene::images`. Unfinished: entries 11 and 12; `shadings.rs` is 530 lines.

**Found, not fixed.** No bug in the code moved. `cargo test -p conformance`:
`the_shipped_worker_inherits_no_feature_another_member_turns_on` fails (bytemuck features turned on
by the whole-workspace build; `crates/pdf-sandbox/Cargo.toml` is a sibling's edit), not mine. In
`cargo doc --document-private-items -p render-raster`, `present.rs` has five unresolved private links
and two `[t]he` quotations read as links (`clips.rs:172`, `own_space.rs:9`), all older than this split.

**Gates.** `cargo fmt --check -p render-raster`: 0. `RUSTFLAGS=-D warnings cargo clippy -p
render-raster --all-targets` (in the export, a sibling's `raster-gpu` edit breaking the shared build):
0. `cargo test -p render-raster`: all binaries ok. Six arms: 6 x exit 0, 0 digest differences,
2 034 s under one hold. `cargo test -p conformance --no-fail-fast`: 101, one failure (above), the
`names` sweep green after the example's doc was fixed. Duration: about 5 400 s.
