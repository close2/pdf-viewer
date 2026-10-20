# 2080 — The transparency interpreter is written in one module per concern

Session 1622, slot 8 of batch ninety-seven, a docs refactoring round (Sonnet 5.5); `doc/todo/74`
entry 7, first half. Status: **accepted** and **built** in `crates/pdf-model/src/content/transparency.rs`
and `transparency/`. Context: the owner's words of 2026-10-19, "Refactor god classes,..." and "Do not
fix any bugs."; ADRs 2004, 2018, 2032, 2046 and 2064 (the same split of five other types).
Clauses: none; no behaviour moves. ADR 2081 is the pattern interpreter's half of the entry.

## 1. What the census found

`transparency.rs` was 6 900 lines: 2 840 lines of free types and functions for clause 11, one `impl
Interpreter` of 42 methods over 2 080 lines, and about 1 950 lines of tests. The question the entry
asked was whether the file is one clause or several. It is several, and the clauses are the module
boundaries: the free functions and the methods that call them already clustered by clause.

## 2. The decision

`transparency.rs` stays the root: the three types the group entry points share (`TransparencyGroup`,
`GroupBody`, `GroupRun`), the entry points (`run_transparency_group`, `run_transparency_group_body`,
`transparency_group`, `appearance_group`, `imported_page_group`), and the tests. Twelve modules each
hold the free items and the `impl Interpreter` methods of one concern:

| module | concern |
|---|---|
| `transfer` | §11.7.5.2: which transfer function a painted object takes (`Painted`) |
| `commands` | what a display-list command does to the page: its bounds, whether it blends or composites |
| `blending_space` | §11.4.7 and §11.6.6: the blending colour space a page or group departs to (`Departure`) |
| `into_parent` | §11.6.6: the conversion into a parent's components, sampled as a cube or curve |
| `press` | §11.4.7 and §11.7.2: the press a page or group of four components composites in |
| `group_runs` | a group's content run once per space, the press pair's second run and the readback rewind |
| `soft_mask` | §11.5 and §11.6.5: a mask group run and registered |
| `readings` | §11.6.4.3: the `/AIS` readings, their scope, and the seals |
| `shape` | §11.6.4.2: an element's shape stated as a command, and whether a raster carries it |
| `knockout` | §11.4.6: the three constructions for a form's knockout group |
| `implicit_knockout` | §11.7.4.4 and §9.3.8: the knockout the standard makes of ungrouped parts |
| `departures` | what a group asked for and did not get, reported |

**Visibility is what a sibling reads.** Every item that was `pub(super)` (visible to `content`) is
`pub(in crate::content)` where `content`'s other files name it, `pub(super)` where only a sibling
module of `transparency/` reads it, and private where nothing but its own module does; the
compiler decided each by error, starting from the narrowest. The names `content` reads
(`Painted`, `Readings`, `Departure`, `OneColour`, `implicit_knockout_group`, `any_command` and
thirteen more) are re-exported from the root, so `use super::transparency::Painted` in the other
files did not change.

## 3. The proof that nothing moved

The multiset of tokens of the old file against the root and the twelve modules, with visibility
words removed: **removed-only is empty**. Added-only are the module docs, the `use` blocks, the
`mod` lines, the `impl Interpreter` wrappers and the paths (`super::` written `crate::content::`
in moved bodies). The line multiset (habit 93) differs by signatures that `rustfmt` wrapped because
`pub(in crate::content)` is longer than `pub(super)`, and by the test imports, which now name
each item's module. `cargo test -p pdf-model` passes unchanged and the six corpus arms agree with
HEAD's page by page.

## 4. What this costs

A reader looking for `blend_at_the_do` opens `implicit_knockout.rs`; one looking for a group's entry
opens the root. `transparency.rs` is still 2 450 lines, 1 960 of them tests. Nineteen ledger
rows now name the module that cites their clause (habit 96).
