# 1273 — A doc comment that names a function names one that exists

Status: accepted and **built**, with the population it found named rather than zero. Session 1218.
Context: `tools/conformance/src/names.rs`, `tools/conformance/src/bin/names.rs`,
`tools/conformance/tests/names.rs`, `tools/state.sh`, `doc/todo/01`.
Builds on: trap 13 (a sweep is calibrated against the defect before it is believed), trap 25 (a
hand-written population decays in both directions), `doc/adr/1240` (the defect), `doc/adr/0372`
and `tools/conformance/src/pointers.rs` (the file half of the same question).

## 1. The defect, and why nothing in this project could see it

`Edit::ChooseFile`'s doc comment said the host decides the request in
`viewer_host::policy::may_choose_file`. No such function existed, and none had. It was written
when the variant was, it was read by every round that opened the file, and the habit placed after
ADR 1240 is the only thing that caught it.

Three instruments look at what this tree writes about itself and not one of them reaches a Rust
path in prose:

- the conformance gate checks a `§` against ISO 32000-2 and a rustdoc blockquote against the
  clause it is attributed to;
- `--bin pointers` (ADR 0372) checks a **file path** a note names, and the symbol half of a
  `file.rs::item` pointer — a different spelling with a different population;
- the ledger's own check resolves the `code` and `test` arrays' sites.

A path written as `` `crate::edit::apply` `` or `` `Interpretation::text` `` is the commonest way
this tree names its own code to a reader, and it was read by nothing at all.

**Nor does `rustdoc` cover it, which is the part worth stating because it looks as though it
should.** Two thirds of these paths are intra-doc links, `` [`crate::edit::apply`] ``, which
`rustdoc` does resolve — under `cargo doc`, and `doc/todo/02` §2 runs `cargo doc` in none of its
three tiers, so nothing runs it. The other third is written in plain backticks, which `rustdoc`
does not resolve and never will: a backticked span is code formatting, not a link. The sweep reads
both populations the same way, because a reader does.

## 2. What resolution is, and the two directions it is deliberately loose in

**The last two segments are the whole of the question.** For `` `a::b::c` `` the sweep asks whether
some item named `c` is declared in a context named `b` — a module, the type an `impl` block is for,
a trait, an enum, a struct, a variant with named fields, or a crate. It does not check that `a`
contains `b`; it does not check visibility, generics or re-exports; a one-segment path is not read
at all, because there is no second half to check it against.

That is a choice and not a shortfall. A stricter reader needs Rust's own name resolution, and a
sweep that half-implements name resolution reports its own gaps as this tree's defects — which is
what the first three drafts of this one did.

The index is built by a brace-tracking scanner over every Rust source the workspace manifest names,
recording each declaration under the block it sits in, the file's own module name and its crate's
name. Two kinds of declaration are the compiler's rather than the tree's and are recorded anyway,
because a reader names them: a `#[derive]`d method (`Asked::default`), and a `pub use` re-export
(`content::Placed`).

## 3. Where a prefix is looked for is the crate the comment is written in

This is the rule that made the instrument readable, and it was measured rather than assumed. Three
scopings were run over the same tree:

| the prefix is looked for | findings, over one tree at one state of the index |
|---|---|
| anywhere in the workspace | 92 |
| in the comment's own crate alone | 328 |
| in the comment's crate and the workspace crates it depends on | 179 |

The middle row is the one that says why: a comment in `pdf-model` naming `colour::MAX_PRESSES`
means `pdf-colour`'s constant, because `pdf-model` depends on `pdf-colour`; scoping to one crate
calls every such sentence a defect. The first row is loose in the other direction: `viewer-ui`
declares a `Surface` and `render-raster` does not depend on `viewer-ui`, so a
`` `Surface::configure` `` written in `render-raster` is `wgpu`'s and no claim about this tree at
all. Reachability is derived from each member's own manifest, so a dependency added changes the
answer without this file being edited (trap 25).

Once the prefix is something the comment's crate can reach, the **last** segment is looked for
under it anywhere in the workspace. That asymmetry is deliberate: `pdf-model` re-exports
`pdf-colour` as `colour`, and following a re-exported module to the crate that fills it is name
resolution again.

## 4. Three rungs are counted rather than reported, and each is a reason the sweep is not being asked

- **A generic prefix** — `Self::`, `self::`, `super::`, `crate::` as the *immediate* prefix, or a
  bare type parameter. `Self::text` names whatever type the enclosing `impl` is for.
- **Another crate's** — the path opens with, or its prefix is, a name the workspace manifest
  declares as a dependency rather than a member, `std`, `core`, `alloc`, or a primitive type; or
  the file's own `use` statements bring that name in from one. A crate this tree re-exports is
  still that crate's: `raster_gpu::wgpu::SurfaceTarget` is `wgpu`'s.
- **A prefix this tree declares nowhere the comment can reach** — section 3's rule, and a path of
  three segments or more whose *head* is likewise unreachable, which is how `peniko::Compose::Copy`
  is read in a file that reaches `peniko` only through `vello`.

## 5. Why it is a gate, and why the gate is a named population rather than a zero

`--bin pointers` is not a gate because a dead **file** pointer is sometimes the right thing to
write: a round may cite the file it is about to add, and a correction has to quote the pointer it
retired. A Rust path is not like that. `CLAUDE.md`'s comment rule says a comment carries the
current reason and nothing about how it got there, so a comment has no business quoting a name that
was retired, and a name a round is about to add is a name that round is adding. There is no version
of this tree in which a doc comment naming an item nobody declares is acceptable.

The population is not zero yet, and a bare count that may only fall says nothing about *which* path
is new. So `tests/names.rs` holds `STANDING`, one line per path the tree carries today, and fails
both ways: a path outside the list is a comment written now about code that is not there, and a
path on the list the sweep no longer finds is a list outliving its facts. The day it empties, the
gate becomes the zero the population deserves.

**A line keyed the list at first and that was wrong**, which a sibling round's unrelated edit
showed within the hour: a doc comment moves whenever anything above it moves, so a line-keyed list
fails on somebody else's change and teaches the next round to regenerate it rather than read it.
The key is the file and the path; the run prints the line.

**The round's own prose fixes were the other half.** Two further mechanical rules took the 179
down to 77 — a prefix naming a dependency, and an unreachable head three segments up — and then
thirty one-line edits qualified a library's type with the crate that declares it:
`wgpu::Surface::configure`, `peniko::Compose::DestIn`, `tiny_skia::Transform::from_row`,
`std::process::Command::output`. That is not a concession to the instrument. A reader of
`render-gpu/src/scene.rs` could not tell `peniko`'s `Compose` from a type of this tree either, and
the sweep asking the question is what made the sentence say which. What `STANDING` holds after
those is the residue, and every line of it is a sentence to correct rather than a rule to widen.

## 6. Calibration

Trap 13, planted in the function and never in the tree, because a plant written into a file is a
plant somebody has to remember to remove. `names::calibrate` runs a doc comment naming a module of
`conformance` and a function nothing declares through the same two functions the sweep uses, and
`tests/names.rs` fails unless the sweep puts it on the finding rung. The unit tests beside it fix
each resolution rule against a fixture: a function under its module and its crate, a method under
the type its `impl` is for, a variant and a variant's field under their holders, a struct field
under its struct, a nested module closing with its brace, and the three counted rungs told apart.
