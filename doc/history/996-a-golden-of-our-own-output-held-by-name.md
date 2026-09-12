# 996 — A golden of our own output, held by name: the review's second next step became a gate

Date: 2026-09-12. ADR: 1016. Worktree `/home/AI/pdf-viewer-rounds`, branch `batch-992-997`, on
`1ea10797` (`main`); a first attempt at this round was cut off by a session limit before it did
anything.

Files: `crates/pdf-model/tests/raster_golden.rs` (new), `crates/pdf-model/tests/raster_golden.tsv`
(new, 974 lines), `crates/pdf-model/examples/raster_digest.rs` and `display_list_digest.rs` (doc
comments pointing at the gate and saying why their hash could not be it), `doc/adr/1016`, this file.
Not touched, by the batch's fences: `doc/todo/02` §2 and `tools/state.sh` (session 995 places the
line and the section; the text is below), `crates/pdf-model/src/` (997), the rasterisers.

## What was built

`doc/reviews/984` Finding 4, verbatim: "There is no golden of our own output, so the oracle sees
disagreement and not change." The gate walks the 974 tracked first pages, rasterises each through
`render-cpu` at 72 dpi, and holds three SHA-256 digests — raster, display list, reports — and an
outcome word by name in `tests/raster_golden.tsv`. A page whose line differs fails naming the page
and the layer that moved; a page absent from the file is reported and passes; a line whose page is
not on disk is reported and passes. `PDFVIEWER_RASTER_GOLDEN=update` rewrites the file and prints the
same classification, so the diff is the review. ADR 1016 has the argument; the test's doc comment has
the construction.

**`display_list_digest` joined**, as two columns rather than a second golden: the cost is a `Debug`
string per page inside a 29-second walk, and what it buys is the first sentence a moved page needs —
whether the interpreter or the rasteriser moved it, and whether only a report did (trap 37).

## Determinism

Two consecutive `update` runs: byte-identical files (`cmp` silent), 974 entries, 966 drawn, and the
check between them `held 974, moved 0, unheld 0, left 0`. Outcomes held besides `drawn`: five
`no-page` (`Brotli-Prototype-FileA.pdf`, `REDHAT-1531897-0.pdf`, `bug1020226.pdf`,
`poppler-85140-0.pdf`, `poppler-937-0-fuzzed.pdf`), one `unopened` (`PDFBOX-4352-0.pdf`), one
`no-target` (`issue19517.pdf`, whose list and reports are still digested), one `locked`
(`issue21579.pdf`, no password on record).

## Calibration (trap 13), both ways

- One name deleted (`tracemonkey.pdf p1`) and one that never existed added: exit 0,
  `held 973, moved 0, unheld 1, left 1`, both named.
- Scratch worktree, own build directory, `render-cpu` inverting the low bit of every raster's first
  byte: exit 101, `held 8, moved 966`, all 966 `raster only`.
- Same scratch tree, `Medium::PAGE_ONLY`'s surround at `r: 0.99`: exit 101, `held 781, moved 193`,
  all `raster only` — the 193 fractional-extent pages `examples/raster_digest` recorded, and no
  other.

The scratch worktree and its build directory were removed afterwards (`git worktree list` shows the
two that were there before).

## Cost

Alone under `tools/bounded.sh`: 28.4 to 38.0 s over 974 documents across eight walks (the later,
slower ones beside neighbours' builds); peak 7.10 to 7.40 GiB resident over the process tree; 24
rayon threads at nice 19. Six walks were run in this worktree and two in the scratch one, never two
at once.

## For session 995 to place

The `doc/todo/02` §2 line, beside `pdf-model`'s `corpus` gate:

```sh
cargo test  --profile gates -p pdf-model      --test raster_golden   -- --ignored --nocapture   # ADR 1016: our own output held by name — a change detector; PDFVIEWER_RASTER_GOLDEN=update regenerates
```

The `tools/state.sh` section (dispatch `golden) section_golden ;;`, and `golden` in `$all` after
`corpus`):

```sh
# ADR 1016's golden of our own output: three digests per tracked first page — raster, display
# list, reports — held by name in `crates/pdf-model/tests/raster_golden.tsv`. A change detector
# and not a verdict: a moved page names the layer that moved, and the round that meant the change
# regenerates with `PDFVIEWER_RASTER_GOLDEN=update` and commits the file. The filter keeps the
# denominator, the four counts and every named page, because the names are the whole signal.
section_golden() {
    run "our own output, held by name (ADR 1016: a change detector, not a verdict)" \
        '^[0-9]+ tracked documents on disk|^held [0-9]+|^  (moved|unheld|left):' \
        cargo test --profile gates -p pdf-model --test raster_golden -- --ignored --nocapture
}
```

`tools/conformance/tests/sandbox_gates.rs` will find `require_the_sandbox` in the test once the line
is placed; `state_sections.rs` will want the section and the line together.

## Gates

Core four, both `fuzz/` lines and `cargo test -p conformance`, run in this shared worktree with
five neighbours mid-edit:

- `cargo fmt --all --check`: 0.
- `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets`: **101**, in `pdf-model (lib)` —
  `drawn`, `alpha_of`, `mask_is_shape`, `same_source` never used and one field-order lint, all in
  session 997's files (`src/image.rs`, `src/content/{image,transparency}.rs`). This test target
  lints clean on its own (`cargo clippy -p pdf-model --test raster_golden`, run before those edits
  landed).
- `cargo nextest run --workspace`: **100**, 4393 of 4394 passed; the one failure is
  `pdf-vfs-ffi::the_kio_worker`, whose cmake cache in the shared build directory was generated from
  `/home/cl/projects/pdf-viewer/kio` and refuses this worktree's path — the shared `target-dir`
  meeting a worktree, not code. The gate's two un-ignored tests pass.
- `cargo test --workspace --doc`: 0.
- `cargo fmt --manifest-path fuzz/Cargo.toml --check`: 0; the fuzz clippy line: 0.
- `cargo test -p conformance`: **101**, `every_quotation_is_the_standards_own_words` on
  `crates/pdf-render/src/shading.rs:200` — a quotation §11.6.4.4 does not contain, in a file with 128
  uncommitted lines that are not this round's.
- This gate, twice alone under `tools/bounded.sh` against the committed file: `held 974, moved 0,
  unheld 0, left 0`, exit 0 both times; 33.6 s and 36.4 s; peak 7.40 and 7.18 GiB.

## Where the digests were taken

The committed `raster_golden.tsv` was regenerated after session 997's rendering sources had been
unchanged for five minutes (watched every 30 s; the diff over `crates/{pdf-model,pdf-render,
render-cpu,pdf-font,pdf-syntax}/src` hashed `3c61def6b905`: `content/image.rs`,
`content/transparency.rs`, `image.rs`, `pdf-render/src/shading.rs`, 789 insertions and 98
deletions over `1ea10797`). At that state the gate holds 974 of 974 — and the file is **byte for
byte the one written at `1ea10797` clean**, so 997's uncommitted work moves no first page of the
tracked corpus in raster, list or report. If a later change of theirs does, the merge regenerates
and the printed list is that change's review.
