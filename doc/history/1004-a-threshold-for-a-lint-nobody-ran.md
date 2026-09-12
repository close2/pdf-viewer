# 1004 — A threshold for a lint nobody ran, and the directory the checker never read

Session 1004, 2026-09-12, branch `batch-999-1004` in the worktree `/home/AI/pdf-viewer-rounds`,
with five sibling rounds live in the same tree (999–1003). Argued by
`doc/adr/1024-a-threshold-for-a-lint-nobody-ran-and-the-directory-the-checker-never-read.md`.

**The finding**: `tools/conformance`'s scan reads a hand-written list of three directories while
the workspace's members are a glob, so 1,884 clause citations in 243 of `raster/`'s Rust files have
been outside the citation and quotation gate since that sub-project was folded in — and nothing
looked different, because a directory nobody reads produces no findings.

Files touched:

- `clippy.toml` — `cognitive-complexity-threshold` deleted, after the lint was enabled over the
  whole workspace and calibrated against planted functions.
- `crates/viewer-ui/src/bin/quorra.rs`, `crates/viewer-ui/src/bin/quorra/arguments.rs` —
  `#[allow(clippy::too_many_lines)]` → `#[expect(…, reason = "…")]`.
- `raster/CLAUDE.md` and 114 Rust files under `raster/crates/` — 209 pointers at raster's own
  documents now name them from the repository root, so that `doc/adr/0053`, `doc/PLAN.md` and
  `doc/HANDOVER.md` stop meaning two different files each.
- `doc/verify.md` — the `serialize` fuzz target's invocation, the command that runs
  `clippy::cognitive_complexity` over the workspace with what its metric actually counts, and why a
  fuzz target cannot go under `tools/bounded.sh`.
- `tools/fuzz.sh` — `--list` exits non-zero on a target `doc/verify.md` does not name.
- `tools/round.sh` — a seventh check: every workspace member is under a directory the conformance
  checker scans, both sides derived.
- `fuzz/seed_page.py` — names `serialize` in the recipe it is.
- `doc/adr/1024-…md`, this file.

## What the round was, and what it was not

It was the direction review's small list (`doc/reviews/984-direction-and-boundaries.md` Finding 7)
plus one question of its own. Four items of the list were taken, one judged and left alone, and
three handed to the rounds that own the crates: `Secret` to `pdf-syntax` and the Annex C.4 wording
are 999's, the `CUTOFF` constant's only possible home is 1002's `pdf-render`, and the crate map is
1000's. ADR 1024 §6 carries each as a line the next round can take.

The judged one is worth repeating here because "judged" reads like "skipped". `pdf-font` depends on
`pdf-render` for `Path`, `PathCommand`, `Point` and `Transform`, and the review reads that as a
font crate reaching up into the rasteriser. It is not: a loaded font's cached glyph outline *is* an
`Arc<pdf_render::Path>`, handed to a display list unchanged, and `pdf-render` depends on nothing
internal. What is fair is the name — the crate holding the tree's geometry also holds the
`Rasterizer` trait — and the move that would answer it is `geom.rs` into a `pdf-geom` crate, which
is a rule-2 change and costs a whole gate sequence. It is worth doing beside another reason to open
`pdf-render` and not on its own.

## The two things that were nothing until they were calibrated

Enabling `clippy::cognitive_complexity` over every workspace member found nothing at all, which
is a sentence about a grep until the instrument is asked whether it can see anything (trap 13).
Asked, it turns out to count `if` expressions: 60 sequential `if … else` score 61, a `match` of 75
arms scores nothing, and four levels of nesting score nothing. The interpreter's `run_reader` — 908
lines, 75 arms — is invisible to it by construction. So the threshold went, and `doc/verify.md`
took the command instead of this file taking the count.

`tools/fuzz.sh --list` was the other. It has printed `serialize … NO INVOCATION` in a row above an
exit status of 0 for as long as that target has existed, and `tools/fuzz.sh serialize` has refused
to start on the same ground, so the fuzz target for the writer this project became answerable for
has never run. The row is now an exit status; the invocation is now written; the corpus on this
disk is seeded with 1,133 documents.

**And the first run of it aborted in the seed pass, before a mutation**: a 184-byte recovered file
the serializer writes back with a `/Root` that no longer reaches a dictionary — the property RFC
0002 §11.3 named, failing on the target nobody could start. The bytes and the reasoning are in
ADR 1024 §5; the site is `pdf-syntax`'s, which is a rule-2 crate and not this round's to open.

## The one the question was really after

Of the four candidates the round was given — `tools/state.sh`'s sections, the fuzz targets,
`doc/verify.md`'s instruments, `raster/`'s gates — three came back with a number that ended them:
31 sections against 31 functions against 31 dispatch arms and no drift in any of the 37 commits
that have touched the file; 16 fuzz `[[bin]]` entries against 16 files, derived rather than listed;
109 undocumented runnable targets out of 197, which is a predicate that fires on half a population
and finds nothing.

`raster/` was the one, and not for the reason it was offered. It is not a second workspace — it was
folded into this one, and `--workspace` reaches it. Its examples are gated better than the main
tree's, by a test that reads the CI workflow and fails naming any example no line runs. What is
outside everything is its citations, because `SOURCE_ROOTS` is a list of three directory names in
the checker's own source and has not grown since. The check now lives in `tools/round.sh`, where
every round already looks, with both sides derived and the cost printed beside the name.
