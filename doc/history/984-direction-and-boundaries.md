# 984 — Direction and boundaries: the architectural review

Date: 2026-09-12. Branch `round-945/the-fifth-round`, at `0dde6224`. Topic: an honest
architectural review of the whole codebase — the viewer, the validator and the converter —
commissioned by the project owner with the words *"is the code going into the right direction, does
the structure look good? Are the boundaries at the right positions … I want to avoid that we are
fixing problems with 'hacks' and overlook a big picture problem."* Read-only over `crates/` and
`tools/`; five sibling rounds (986–990) were live in the same tree, and nothing under `crates/` or
`tools/` was written. A first attempt at this round was ended by a session limit before it wrote
anything; its four early notes were re-verified rather than inherited, and three of the four held
(no producer-name branching; `salvage_number` justified by Acrobat and pdf.js; the save round-trip
in no gate line) while the fourth moved by a point (the oracle's `ambiguous` share is 42.7% —
835 of 1,956 — not 44%).

Argued in **ADR 1005**. The review itself is **`doc/reviews/984-direction-and-boundaries.md`**, a
new directory.

## What was done

- **The crate graph, read from `cargo metadata`** rather than from the crate map: 32 workspace
  packages, acyclic, the three rasterisers on `pdf-render` alone, `viewer-core` with no external
  package. Every crate's `lib.rs` header read against its dependency set.
- **The hunt for hacks, with the commands.** Producer names in conditions: none. `for now` /
  `workaround` / `hack` / `TODO` / `todo!` outside tests: 8 lines, all about somebody else's. 103
  float constants in non-test source read; the two that looked tuned (`90.51`, `0.525`) checked with
  `git log -S` and found derived. 651 `#[expect]`s binned by lint. Every `salvage` / `recover` /
  `repair` function found and its clause or its report named.
- **Complexity ranked** by a scanner over all 13,512 functions: 227 over 100 lines, 22 over 200, 6
  over 400; the three worst read and classed essential or accidental.
- **The gate sequence matched against every `#[ignore]`d file** by package and test name: seven of
  twenty-seven are in no line of `doc/todo/02` §2 or `tools/state.sh`, among them the converter's
  corpus walk that found ADR 1006's signature and the validator's veraPDF comparison, and
  `tools/state.sh --list` has no validator section at all.
- **The chronology measured**: 1,353 session ordinals and 3,744 ADR references in Rust comments,
  261 comment blocks of fifty lines or more, 855 ADRs of which 132 amend another, 241,995 lines of
  Markdown under `doc/`, 26% of the last 150 commits' lines into `doc/`.
- **The spine read**: `pdf_archive::check` → `Examination` → `Report` → `decision.rs`'s table →
  `rewrite.rs`'s whole-file walk → re-validation; `survey.rs` found to be a second content-stream
  state machine over the interpreter's own token reader.

## What it found

Five shapes, ranked, none a hack: `pdf-model` as five crates in one (191,001 lines, 37% of the
tree, twelve cryptographic packages); knowledge indexed to a chronology that a student would have
to read; the gravest instruments outside the sequence and no golden of our own output; a validator
contract that is per-object and a converter whose unit of change is the file; and RFC 0007 parked,
correctly, on eight owner questions. And what is right, to the same standard: the layering, the
absence of hacks, the typed refusals, a ledger with no `unreviewed` and no `silent` row, the
converter's net, and tests that are behaviour over real documents.

## Gates

`cargo test -p conformance` over the three files written — see the round's report; a sibling
mid-edit (`crates/pdf-model/tests/zz_salvage_census_990.rs` was untracked in the tree during this
round) can make the workspace red elsewhere and did not touch these files. No other gate was owed:
the round changed no line under `crates/` or `tools/`.
