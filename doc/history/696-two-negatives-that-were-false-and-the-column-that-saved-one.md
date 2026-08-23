# 696 — Two negatives that were false, and the column that saved one of them

Four more of `doc/todo/01`'s owed negatives re-derived over both populations: **two false, two hold**.
One of the false ones took a ledger row's *status* down with it; the other was false in the half its
row states and true in a half nobody had written, which a third column in the same instrument
settled. A fifth erratum in five rounds arrived on one of the rows.

Date: 2026-08-23.
ADR: [0548](../adr/0548-four-negatives-two-false-and-the-consequence-that-was-not.md).

Touched: `crates/pdf-model/examples/operator_shape_census.rs` (new),
`crates/pdf-model/examples/variable_text_census.rs`, `crates/pdf-model/tests/text_state.rs` (doc
comments), `doc/conformance/ledger.toml` (§8.5.2.1, §8.5.3.1, §9.4.2, §9.7.5.4, §12.7.5.3,
§12.7.5.4), `doc/errata-read.md`, `doc/todo/01-ledger-partial-rows.md`, the ADR and this file.

## What the queue said, run rather than quoted

`doc/todo/01`'s own script in this worktree before the edit printed **30 done and 16 owed** over a
population of 46 — which is what the briefing said. After the edit it prints **34 and 12** over the
same 46; the population was checked as well as the counts, because a correction in house style takes
a row out of the sweep instead of across it.

**Of the twelve left, ten are not measurement**: four are not a claim about a corpus (§7.5.6, §8.9.3,
§10.7.4, §11.6.2) and six are the sweep's own noise. **Two are real** — §9.7.6.2's codespace range and
§11.6.7's tiling pattern paint — and neither is an operator shape, so neither belongs to the
instrument this round built.

## The instrument

`examples/operator_shape_census` answers a claim about an *order* of operators, which neither
`witness_census` (a name, or a token) nor `absence_audit` (a structure) can reach. It lexes a page's
`/Contents` and every form `XObject` its resources reach, takes ADR 0523's three scopes plus a
`--pages=` bound, skips inline-image data through `pdf_model::inline_image::scan`, and prints how many
content streams it does **not** walk.

Five planted fixtures were run before any population: one with every witness shape, one clean, one
whose inline-image data spells `l` and `q … Tm … Q` (scores zero), one with `0 0 l S` *after* the
`EI` (scores one, so the skip resumes where it should), and one whose restore reaches a mark. All
five behave, and all five were re-run unchanged after the file was refactored to satisfy clippy.

## The four

| clause | the claim | curated p1 | curated ≤p100 | crawl p1 |
|---|---|---|---|---|
| §8.5.2.1 | a segment operator with no current point | **1 doc** | **12 docs, 5010** | **5 docs, 133** |
| §8.5.2.1 | the same for `h` | 0 | 1 doc, 149 | 16 docs, 890 |
| §8.5.3.1 (control) | a painting operator on an undefined path | 8848 / 13 | 15 609 / 26 | 38 047 / 459 |
| §9.4.2 | a `q` or `Q` inside a text object | 9 / 2 docs | 784 / 5 docs | 52 / 8 docs |
| §9.4.2 | … with `Tm` moved between (sharp) | **4 / 1 doc** | 4 / 1 doc | **1 / 1 doc** |
| §9.4.2 | … or text shown between (broad) | 4 | 5 / 2 docs | 1 |
| §9.4.2 | … whose restore reaches a mark | **0** | 1 (damaged) | 1 (damaged) |

`witness_census --crawl` for §9.7.5.4: **0** `beginrearrangedfont` and **0** `beginusematrix` over the
65 703 crawled documents that open, with `endcmap` in **46 028** of them as the control. 23 minutes.

`variable_text_census --crawl` for §12.7.5.4: **2** list-box widgets over 2 documents, both with an
`/AP` `/N`, neither in a `/NeedAppearances` document — the negative holds. Its `--pdfjs` control run
reproduces the row's figures to the digit first. 273 combo boxes of which 14 state no appearance.

**§8.5.2.1's row went `implemented` → `partial`.** The clause's *an error shall be generated* is
reachable, and what it costs is a mark: `tiny_skia::PathBuilder` injects a move to the origin, so the
page gets a line from the corner of user space. The `h` half costs nothing, because
`content::path::close_subpath` pushes nothing onto an empty path — one sentence in the standard, two
costs here.

**§9.4.2's row keeps its conclusion on a sentence it never wrote.** Four `q … Tm … Tj … Q` pairs sit
inside text objects on `NegativeFontSize.pdf`'s first page, so *not one moves Tm between the two* is
false. But Table 106 makes the next `Tm` replace what the `Q` restored, so the restore reaches a mark
only in a damaged stream — both such pages were opened and read by hand, and both lex garbage into a
`q`. The synthetic pair in `text_state.rs` is still the only discriminator.

## Issue #373

`spec-errata emit` on every clause touched found **Issue #373, `Review/Completed`, on §9.4.2, recorded
nowhere in this tree**: Table 106's `T*` row gives the operator as `0 –Tl TD` and the erratum strikes
`TD` for `Td`. Its `/QuadPoints` and `pdftotext -bbox`'s word box agree to three decimals. It costs no
arithmetic and the code had the amended form all along; §9.4.2's ledger row and `text_state.rs` both
quoted the retired one. **The four rounds before this found a `Caret` with no `StrikeOut`; this one
strikes and replaces and `check` is still blind, because a one-word strike is below its four-word
floor.** Two more recorded so nobody looks again: #372 is typography, and #191 is filed under
§12.7.5.4 by `emit` and belongs to §12.7.5.3's Table 232 `/MaxLen`, whose floor `appearance.rs`
already enforces.

## Gates

`doc/todo/02` §2 whole, `PDFREF_CACHE` pointing at the shared warm cache
(`/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`) as the briefing asks. Every line exit 0 **except
the oracle**, and that one fails identically at `HEAD`.

- `cargo fmt --all --check`, `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets`,
  `cargo nextest run --workspace` (**2491 passed, 18 skipped**, 71 s), `cargo test --workspace --doc`,
  `cargo check --manifest-path fuzz/Cargo.toml --bins`.
- Both trap-10 builds.
- **corpus** — 974 documents in 3.3 s, 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless,
  67 incomplete, 0 slow.
- **oracle** — **exit 101**: `1 page(s) newly contradicted … ["function_based_shading_cmyk.pdf page
  2"]`, 1945 pages in 57 s. **This round did not cause it and that was established rather than
  argued**: the working tree changes no library source at all — two `examples/` binaries, one test
  file's doc comments, and documents — and the same gate run on a tree reverted to `HEAD`, in the same
  sitting, fails on the same page with the same message. It arrived on `HEAD` from a neighbour's
  merge; page 1 of that document is already on the contradicted list and page 2 was not.
- **text extraction** — 98.26% (10969/11163 words) in bounds over 508 of 974 documents judged,
  486 fully in bounds; all four tests pass.
- **selection census** (98.91% over 453 documents), **accessibility census** (104 with structure),
  **dates** (97.99%), **xmp** (319 carry the stream), **jpeg2000**, **fixed documents** (40 checked,
  0 absent), **quorra corpus** (957 pages, 932 agree, 23 differ) — all exit 0.
- `cargo test -p conformance` — **875 rows**: 443 implemented, 224 partial, 18 reported, 69
  inapplicable, 8 writer-side, 113 out-of-scope, **0 unreviewed**, 10 690 citations. One status moved
  and it is §8.5.2.1's.

§5's binaries were deliberately not installed: this is a parallel round told not to merge, no launch
or frame figure was taken, and `target/` is the main tree's.

## Sweeps

Fifteen sweeps plus `spec-errata check` and `applied`, before and after, run with `cargo run` from
this worktree (trap 15). Nine moved and every delta is this round's own prose.

- `counts` 7145 → 7190 sentences, attributed 390 → 391, the one new attribution being under a clause
  with no rows below it — that sweep's own documented noise shape.
- `owed` 223 → 224 `partial` rows and 3647 → 3666 terms, with **175 unnamed over 111 rows unchanged**:
  §8.5.2.1 joined the population and every term it states is already named by a source.
- `pointers` 7566 → 7591 with **absent unmoved at 130**. Two categories moved and both are exact:
  "a form" 171 → 170, because `variable_text_census`'s module comment stopped naming
  `doc/pdf.js/test/pdfs/*.pdf` and now says `--pdfjs`; "not carried" 416 → 418, because two censuses
  now name `corpus-cache/safedocs/cc-main-2021-31`, which is machine-local by design.
- `quotations` 5665 → 5670 over 854 → 856 documents with verbatim 2440 → 2442 and **diverging unmoved
  at 34**; in the ledger 1864 → 1867 with verbatim 1426 → 1427 and **diverging unmoved at 2**. The
  second document and the last quotation are this file, which is why the figure is the one printed
  after it was written rather than before.
- `tables` 6116 → 6133 sentences and 2286 → 2289 attributed key citations, with **absent unmoved at
  101** and the denial count unmoved at 6 — so no wrong table number in the prose this round wrote,
  which is what that sweep is for.
- `ledger` implemented 444 → 443, partial 223 → 224.
- `overtaken` 514 → 515 decision records with **39 overtaken unchanged**.
- `spec-errata applied` 51 713 → 51 835 places, 555 → 569 naming an erratum, 1535 → 1580 comparisons,
  with the **90 / 10 / 171 split unchanged** — no new place quotes struck text.
- `spec-errata check` did not move at all.

`blockers` moved to 32 sentences with 9 naming no clause on the first run after the edit, and it was
this round's: a doc comment saying an `ET` ends "anything waiting on its matrix" reads as a blocker to
that grep. Reworded, and the sweep is back at its own level of 31 / 12 / 11 / 8 — which is the
argument for running the sweeps after the edit rather than only before.

`capabilities`, `callers`, `entries`, `inapplicable`, `overstated`, `retired` and `unread` did not move.
