# 960 — A bound below the standard's own data

Branch `round-945/the-fifth-round`, beside two rounds in `pdf-archive` and
`pdf-transform/src/archive/`.

## What this round did

ADR 0961's follow-up, taken as the instruction stated it: for every bound on the render and
interpret path, ask whether reaching it raises `Unsupported::LimitReached` or hands back a
silently short answer. Most of them already report — `content.rs`'s four, the four apiece in
`pattern.rs` and `run.rs`, `pdf_render::MAX_GROUP_DEPTH` and `MAX_EXTENT` as `BackendError`s,
`function.rs`'s refusals, `optional_content.rs`'s `Visibility::TooDeep`, `image.rs`'s
`ImageError`s. `pdf-font`'s four `CMap` bounds did not, and one of them was **below the floor
§9.7.5.2's own data sets**: `MAX_RANGES` was 16 384 and `UniCNS-UCS2-H` states 16 418
`cidrange` entries, so the last 34 — the fullwidth Latin block of Adobe-CNS1 — were dropped at
parse time and every one of those codes drew CID 0.

ADR 0963 has the finding, the clauses (§9.7.5.2, §9.7.6.2's `shall`, §9.7.6.3's substitution),
the census over `data/cmaps/`, the price of raising the bound and the two calibrations.

## Files

- `crates/pdf-font/src/cmap.rs` — `MAX_RANGES` to `1 << 15` with the census and the cost in its
  comment; `CMap::truncated`, `cut_by`, and the four bound sites that now set it.
- `crates/pdf-font/src/loading.rs` — `LoadedFont::cmap_truncated`, and
  `no_registered_cmap_is_cut_by_these_bounds`.
- `crates/pdf-model/src/content/font.rs` — `note_cmap_truncation`, at the load and at the
  cross-page cache's hit.
- `crates/pdf-model/tests/hostile_budgets.rs` — the fixture either side of the bound.
- `crates/viewer-core/src/report.rs` — the `LimitReached` sentence, which claimed every bound
  stops the page.
- `doc/conformance/ledger.toml` — §9.7.5.2 and §9.7.6.2, both still `implemented`, both with
  what was found written into the note and the new tests named.

## The accessibility census failed and it is not this round's

`what_a_screen_reader_is_told_about_every_document` failed on the ceiling *pages with no
`/StructParents` whose fallback answered nothing*: **60 of them, and there were 56**. The four
are `icc_1_2001-12.pdf` pages 4 to 7 — one file, dropped into `doc/` on 2026-09-10, gitignored
and therefore invisible to `git status`. `population()` is a `read_dir` of `doc/`, so the file
joined the census the moment it landed.

Attributed by removing the suspect rather than by argument: with that one file moved aside the
census prints `56` and exits 0, and with it back it prints `60` and exits 101. Nothing else was
changed between the two runs.

This is ADR 0962's subject on a second instrument. That ADR found the oracle's ratchet moved by
six specification PDFs arriving on 2026-09-09 and -10; this is the same cause reaching a
*ceiling* rather than a floor, where the message reads as a defect class growing. The ceiling is
a literal in `crates/viewer-core/tests/accessibility_census.rs`, a file this round did not
otherwise touch, and raising it is a decision with a shape worth arguing — a count over a
population two things can change is not a ratchet — so it is left for a round that will make
that argument rather than moved quietly here.

One side effect worth recording: moving the file out and back changed its owner from `cl` to
`AI`. Its bytes, mode and mtime are unchanged (`722868fae4f67d6c4b189749734525df`, 644,
10 Sep 20:08) and the agent cannot chown it back.

## Gates

Everything in `doc/todo/02` §2, on a tree two neighbours were writing into.

- `cargo fmt --all --check`, `cargo fmt --manifest-path fuzz/Cargo.toml --check` — clean.
- `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` and the `fuzz/` line — exit 0.
  An earlier run of the workspace line failed with two `E0061`s in
  `pdf-transform/src/archive/prepare.rs`, a neighbour's file mid-edit; it was green an hour
  later without anything of this round's changing.
- `cargo nextest run --workspace` — **4279 run, 4279 passed, 35 skipped**.
  `cargo test --workspace --doc` — exit 0.
- `pdf-model --test corpus` — ok, **63 documents incomplete**, none of them reporting any
  `max_cmap_` bound, which is the population's answer to whether any file on this disk reaches
  one.
- `pdf-model --test oracle` — ok, **62 contradicted pages, every one held by a group by name**.
- `text_extraction` — ok, **11095/11132 matched words in bounds (99.67%), 494 of 504 documents
  fully in bounds**.
- `selection_census` — ok. `accessibility_census` — **failed, and the section above is why**.
- `viewer-ui --test launch_path` (`--release`) — ok, **21 figures banded, 0 outside**, cold
  graphics bring-up 33.5 ms.
- `dates`, `xmp`, `jpeg2000`, `render-raster --test corpus`, `fixed_documents`
  (**77 checked, 0 absent**) — all ok.
- `pdf-transform`: `gate`, `writer_corpus`, `split_corpus`, `merge_corpus`, `pages_corpus`,
  `optimize_corpus` — all ok. `foreign_corpus` **failed on its first run and passed on a
  re-run**, on the `bookmarks` lane's §14.7 count going 1 → 0 with nothing else changed —
  which is §2's own paragraph about a contended foreign reader resolving less structure, and
  the same lane it names.
- `pdf-vfs`: `write_corpus`, `read_corpus` — both ok.
- `cargo test -p conformance` — exit 0.
