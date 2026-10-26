# 1775 — Table 380's decoration and position reach the accessibility tree

Session 1775, a clause round in batch 117; 2026-10-26. ADR 2384; ADR 2385 unused; no Q.

**Contract.** Slot 5, `A634`: Table 380's decoration and baseline shift (by its sign, a documented
choice) and ruby where the role exists, in the accessibility tree; the launch path proved unmoved.

**Moved.** §14.8.5.4.4 `inapplicable` → `partial` — NOTE 1's "export it to some other document
format" is what the tree is; §14.8, §14.8.5, §14.8.5.4 derive `partial`; `doc/todo/65` places all.

**Premises.** *One function from the reader* — **held**: `Tree::inline_statements`, one read.
*`/TextPosition` has no counterpart* — **did not hold**: `accesskit` 0.24.1's `VerticalOffset` is
it, so it is built and outranks the sign; five stay unread — `/LineHeight`,
`/GlyphOrientationVertical`, `/TextDecorationThickness`, `/RubyAlign`, `/RubyPosition` (the ruby
roles already crossed). *AT sees all four* — **half**: `accesskit_atspi_common` 0.19.1 publishes
`underline` and `strikethrough` only. *A sign* — **held, with the clause's two rules**: summed along
the inline chain, reset at a block and at a `Figure`, `Form` or `Formula` (§14.8.5.4.6).

**Found.** A real AT-SPI bus reads the headless fixture's `E = mc` and `2` `underline: single`,
` gone` `strikethrough: true`, the block in the underlined `Div` nothing. Census: 1 879 underlined,
1 overlined, 56 raised, 1 392 lowered in 20 documents; ISO 32000-1's 1 350 lowered are
class-shifted `Span`s (Table 86's `x00 y00`), read in its bytes. The C ABI carries none of it yet.

**Files.** New: `crates/pdf-model/src/structure/inline.rs`, ADR 2384, this record. Edited (`-U0`):
`pdf-model`'s `structure.rs`, `structure/tests.rs`; `viewer-core`'s `accessibility.rs`,
`tests/accessibility_census.rs`, `tests/headless.rs`; `viewer-accessibility`'s `tree.rs`,
`tests/tree.rs`; `viewer-confined`'s `protocol/panels.rs`, `protocol/tests/panel_answers.rs`;
`doc/conformance/ledger.toml` (four rows), `doc/todo/65`, `doc/state-of-play.md`,
`doc/crate-map.md`. I worked in `scratchpad/r1775/`.

**Gates.** In my export, on the four crates: `cargo fmt --check` and `clippy -D warnings
--all-targets` exit 0; `nextest` exit 0, 2 791 passed; rustdoc `-D warnings` exit 0 (the
workspace's `tools/state.sh docs` walk not run). `accessibility_census` at HEAD and after: exit 101
both on HEAD's ratchet (894 untagged pages over a floor of 893), 93 count lines identical. The drive
(`accessibility`, `drive`, `confined`, four windows): exit 0, 121 works, 0 wrong. `launch_path` with
clocks, one `--clock --nice 0` hold, HEAD/mine/HEAD/mine: exit 101, 101, 101, 0, each arm outside a
different band, every first page `not judged` (carve-out), loads 3.34, 2.85, 2.79, 2.74, render
node unheld, opens allocating alike; the tree is asked after the first present and `launch_path`
never asks it; callgrind, ISO 32000-2 p. 100's `gather`: 165 241 221 Ir HEAD, 165 100 424 mine.
`cargo test -p conformance --no-fail-fast`: exit 0, 69 targets green.
