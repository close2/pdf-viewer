# 1042 — The lint that holds trap 7, and the 49 silencings it found under `raster/`

Session 1025. Status: **accepted**. Enables `clippy::allow_attributes` in
`[workspace.lints.clippy]` and in `fuzz/Cargo.toml`'s copy of that table; converts 190 `#[allow]`
attributes to `#[expect]`, deletes 31 that silenced nothing, and moves the three
`unsafe_position.rs` tests that pin the attribute's spelling. Enacts the one RETIRE recommendation
of `doc/reviews/1018-what-retiring-a-trap-would-cost.md`, and **does not retire trap 7** — see §4.

`§N` is ISO 32000-2 and nothing else. Every figure below was produced in this session by a command
over this tree.

## 1. The decision

**A rule a round must remember is worth less than a rule a program enforces.** Trap 7 —
`#[expect(..., reason = "...")]`, never `#[allow]` — was prose for 555 sessions. Clippy has a
restriction lint whose own description *is* that rule:

> `#[allow]` will not trigger if a warning isn't found. `#[expect]` triggers if there are no
> warnings.

It is `allow` by default and present in this toolchain (clippy 0.1.97). Set to `warn` in the
workspace lint table it becomes a **build failure** under `doc/todo/02-every-round.md` §2's tier-1
`RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets`, which every round runs.

`fuzz/Cargo.toml` restates the table rather than inheriting it — one workspace cannot take
another's (trap 23, ADR 0742) — so the line is added there too, which is what
`tools/conformance/tests/workspaces.rs` checks.

## 2. What it cost, and the population the review's grep could not see

Review 1018 named **three** violations, all under `crates/pdf-transform/src/archive/`, from a grep
for `#[allow(clippy::` over `crates/` and `tools/`. The lint, run once, named **221**:

| where | sites |
|---|---:|
| `raster/crates/raster-gpu` | 200 |
| `raster/crates/raster-scene` | 9 |
| `raster/crates/raster-function-conformance` | 5 |
| `raster/crates/raster-pages` | 1 |
| `crates/pdf-transform` | 3 |
| `crates/viewer-qt`, `crates/viewer-ffi`, `crates/pdf-vfs-ffi` | 1 each |

**215 of the 221 are under `raster/`** — the renderer commissioned in its own repository and folded
into this workspace as members, which is exactly the shape trap 25 describes: a hand-written
population names what its author was thinking about. The review's count was not wrong about what it
looked at; it was wrong about what the tree is. The instrument that answered correctly was the
compiler, and that is the general lesson rather than a remark about one grep.

## 3. What it found: 49 silencings that silenced nothing

Converting an `allow` to an `expect` is not a rename — an `expect` whose lint never fires is
`unfulfilled_lint_expectations`, which is an error under the same `-D warnings`. Over four passes,
**49 lint names in 221 attributes turned out to be dead**, and **31 whole attributes came out**:

- **`clippy::expect_used` and `clippy::panic` inside tests** — this tree's `clippy.toml` sets
  `allow-expect-in-tests`, `allow-panic-in-tests` and `allow-unwrap-in-tests`, so a test module's
  `#[allow(clippy::expect_used)]` has never silenced anything. Fourteen of them, several carrying a
  comment ("test-file policy, as in `resources.rs`") that documented a policy the configuration had
  already granted.
- **`clippy::arithmetic_side_effects` on floating-point code** — the lint does not fire on `f32`
  arithmetic, so four allows on `raster-gpu`'s stroking geometry and several elsewhere were
  decorative.
- **`clippy::float_cmp` against a zero literal** — the lint exempts it, which makes
  `Affine::preserves_axes` and `transform_preserves_axes` two functions whose exactness needed no
  permission at all. Their comments are kept and now say so.
- **`clippy::too_many_lines` under the threshold** — four, each carrying a real design sentence
  about ISO 32000-2 Table 42 being one table. **The attributes are deleted and the sentences are
  kept**, moved into the functions' doc comments: the lint was not what made them true.

Nothing was deleted that carried a reason. Where an attribute came out and its trailing comment
justified only the silencing, the comment came out with it; where the comment stated a fact about
the code, it was kept or merged into its neighbour.

**No `#[allow]` is kept as a deliberate exception.** The honest-exception case the enabling
anticipated — a lint that fires only on some targets or toolchains, where an `expect` would itself
warn — did not occur once in 221 sites on this toolchain. A future one is written as `#[allow(...,
reason = "...")]` with `#[expect(clippy::allow_attributes, reason = "...")]` above it, which makes
the exception itself something the compiler has to be told about.

## 4. What the lint does **not** hold, which is why trap 7 stays

`clippy::allow_attributes` fires on **outer** attributes only. Calibrated rather than assumed
(trap 13), in `raster-pages`:

| planted | result |
|---|---|
| `#[allow(clippy::missing_panics_doc)]` on a function | `error: #[allow] attribute found`, exit 101 |
| `#![allow(clippy::missing_panics_doc)]` inside a module | exit 0, silent |

**160 inner `#![allow(...)]` stand in the tree today**, most of them the crate-level lists at the
top of the census programs under `crates/*/examples/`. So review 1018's recommendation — "enable
the lint … and then delete trap 7's prose" — is taken on its first half and refused on its second.
The trap is rewritten to say that a program holds the outer half and that the inner half is still
the author's, and its incident is kept in full. **A rule that is enforced over four fifths of its
population is not a rule that can stop being written down**; the index line changes from *you
silence a lint* to *you write a crate- or module-level `#![allow]`*, which is the part still left.

## 5. The three `unsafe_code` lifts, and the tests that spell them

`viewer_qt::bridge`, `viewer_ffi::abi` and `pdf_vfs_ffi::abi` each lift `#![deny(unsafe_code)]`
once, and three `tests/unsafe_position.rs` assert on the attribute's **text** — `#[allow()]` after
their comment stripper, `#[allow(unsafe_code)]` raw. All three now read `expect`, and the
conversion is an improvement rather than a cost: an `#[expect(unsafe_code)]` on a module that stops
containing `unsafe` becomes an error, so the count those tests keep is now held twice.

`only_the_three_named_crates_in_the_tree_lift_the_denial` sweeps `crates/*/src/` for the attribute
and is the one place the spelling is a **population** rather than a fact. It now matches all four
forms — outer and inner, `expect` and `allow` — because the property is *who lifts the denial*, and
a sweep that reads only the spelling in use today would go quiet the day the other one came back
(trap 13).

## 6. What this does not decide

The 160 inner `#![allow]` are not converted here. Most are census programs whose crate-level lists
name lints that do not all fire in every one of them, so each conversion is an `expect` that has to
be trimmed to what actually fires — real work with a real yield, and a round of its own. It is
`doc/todo/43`'s kind of item rather than this ADR's.
