# 1379 — A pointer to an owner's answer is the questions gate's, a template is a form, and the trap index is checked both ways

Session 1271. Status: accepted and **built**.
Context: `tools/conformance/src/pointers.rs` (`Reach::AnswerNotHere`, `question_of_answer`,
`template_end`, `NOT_CARRIED`), `tools/conformance/src/bin/pointers.rs`,
`tools/conformance/tests/traps.rs`, `tools/state.sh` (`section_instruments`), `doc/verify.md`.
Builds on: ADR 0372 (`--bin pointers`), ADR 1036 (the trap index), ADR 1355 (a sweep that cannot
reach zero names what stays).

## 1. What the pointer sweep said that was not true

`--bin pointers` printed 349 absent pointers, and four of them sat in the four navigational
documents on every run of `tools/state.sh navigation`: `scratchpad/r` twice in `doc/HANDOVER.md`,
`doc/questions/A130` twice in `doc/PLAN.md`, `doc/questions/A100` once in `doc/state-of-play.md`.
None was a dead pointer.

- `scratchpad/r<round>/` is a **template**: the tokeniser stopped at `<` and resolved the stem.
- The `A` files are the owner's. They land through the owner's own commit (`doc/questions/README.md`,
  the batch brief), so a worktree routinely holds a `Q`, and prose citing its answer, before the
  `A` is in the tree. 74 pointers across the ledger, the sources and the documents were that shape.
- `scratchpad/`, `doc/pdfa`, `doc/veraPDF-library`, `doc/veraPDF-corpus`,
  `doc/specifications.password`, `doc/errata.md` and `doc/adr_revisit` are each ignored, a
  submodule, generated, or the owner's own notes — what `NOT_CARRIED` exists to name — and
  `doc/.gitignore` was absent only because the walk skipped hidden *files* as well as directories.

## 2. The decision

- A `<…>` segment of lower-case words straight after a path character belongs to the token, and a
  token holding `<` is a form (`Reach::Placeholder`), as a glob is. The limit to lower-case words
  and hyphens is what keeps `Vec<String>` and `a<b` out.
- `doc/questions/A<n>…`, absent, **where `Q<n>` is in the tree**, is `Reach::AnswerNotHere`, its own
  rung, printed as the distinct answers named. An `A` with no `Q` stays absent — nothing asked it —
  and the unit test plants `A999` to show it does. Whether a question is answered is
  `tests/questions.rs`'s to say over the directory, not this sweep's over prose.
- The ignored and generated paths join `NOT_CARRIED`, each with its reason in the constant's
  comment; a hidden file is walked, a hidden directory is not.

Before → after, same tree: 349 absent (318 standing, 31 corrections) → 247; the four in the
navigational documents → 0. What remains is a reading list of real shapes (documents addressed to
raster's old tree, corrections), unchanged by this.

## 3. The trap index resolves in one hop, and a gate now says so

The index's last column names the group file holding a trap's incident; the merge writes the row and
the entry in two edits. `tests/traps.rs` fails on a row whose group file has no `###`/`####` heading
of its number, and on a numbered heading no row names with that group; a row demoted to a habit is
resolved in `doc/habits/measuring.md`. Planted: trap 59's row moved to `pixels` fails naming both
directions. Today every row resolves, 53–59 included.

## 4. What is counted rather than written

`doc/traps/README.md` states no share of citations; `tools/state.sh traps` prints it. `doc/verify.md`
no longer claims to hold every instrument: `tools/state.sh instruments` lists the examples it does
not name with each one's own first header line (140 of 198 before this round added its lines, 135 after), a reading list rather than a
failure, because an example may be a one-off probe a record already describes.

## 5. Cost

A pointer to an `A` file that is mistyped but whose `Q` exists is hidden on the new rung; the
rung prints each distinct answer so a person sees the name. `NOT_CARRIED` is a hand list beside
`.gitignore` (trap 25's shape); deriving it would need a gitignore matcher, which this crate, with
one dependency, does not carry.
