# 988 — A priced follow-up whose witness was another shape

2026-09-12. ADR 1009. Transparency, ISO 32000-2 clause 11 continued from ADR 1000: the form
`XObject`'s knockout group, §11.3.7.2's one alpha per pixel, and *when* §11.7.2's conversions
happen. Five sibling rounds were live in the same tree (984, 986, 987, 989, 990).

Files: `crates/pdf-model/src/content/transparency.rs`, `doc/conformance/ledger.toml` (§11.3.7.2,
§11.4.4, §11.4.6, §11.7.2, §11.7.4.4, §11.7.5.3), `doc/todo/23-transparency-departures.md`,
`doc/adr/1009-a-priced-follow-up-whose-witness-was-another-shape.md`, this file.

## What the round did

ADR 1000 §7 priced a form's non-isolated knockout group "whose elements share an affine mode
(`issue18032.pdf`)" as able to take route 2 — the elements' shared blend mode moved to the group's
`Do`, the elements drawn Normal on transparency, every backend drawing the result. Opening the
witness: its two elements are a nested group holding a shading under `/BM /Color` and a nested
group at `ca 0`. No affine mode, no elementary element. The derivation holds anyway, for a reason
the pricing did not state: an element of zero opacity has weight 0 everywhere and contributes no
colour (under the opacity reading it still knocks out to the backdrop, §11.4.6's NOTE 5), and with
one coloured element left `K ⁄ Σwᵢ` is that colour at every pixel, so *any* mode moves. A nested
isolated group is admitted as an element, its own `Do`'s mode being the one that moves.
`transparency::knockout_construction` is the three constructions for the form caller in the order
the implicit callers try them, with the mode-at-the-`Do` one before the own-backdrop one because
two backends refuse the latter. `issue18032.pdf` at scale 2: **0 pixels changed** against the
own-backdrop construction, on three backends rather than one.

On the way: `command_blends` fell to `_ => true` for a `Command::Shaped`, so the switched form
reported "non-isolated, and an element blends with the backdrop it excludes" about a mode nothing
carried — and so would any `/I false` knockout group drawn on transparency with a stated element,
since ADR 0234. A stated element blends as its object does now. No corpus page carried it.

§11.3.7.2: the row asked where one alpha per pixel is the wrong answer. Every formula of §11.3.3,
§11.3.6 and §11.5.3 reads `α_s = f_s × q_s` alone; the one reader of a shape apart from an opacity
is §11.4.6, where this tree states the shape (`Command::Shaped`) or reports the element by name.
The fixture from the clause has been in the tree since ADR 0234
(`a_stated_shape_knocks_the_element_under_it_out_entirely`, `(127, 127, 255)` against the product's
`(127, 0, 127)`). The row stays `partial` for the two reported elements — an image whose samples
may be shape or opacity, a translucent shading — and its note now says the "shape channel every
command carries" is not owed by this clause.

§11.7.2 / §11.7.5.3: the conversion in happens at the painting operation (`Interpreter::colour`,
under that operation's state) and the conversion out at the `Do`, after every composite inside
(`GroupBlending` resolved over the finished raster before the paint) — both the clause's moments,
`a_group_that_introduces_a_press_composites_in_it` the test of the second. What is not read at the
`Do` is the parameter, `/RI` and `/UseBlackPtComp` (the press is sampled once, `A2B1`-else-`A2B0`,
compensation on), which is `crate::colour`'s — the colour round's file — and is recorded in both
rows.

## Owed to other rounds' files

- `crates/render-raster/tests/corpus.rs::REFUSED_BEFORE_THE_SCENE` (987's): `issue18032.pdf`
  comes off, `[&str; 4]`, with the doc comment's sentence naming it for ADR 0327's construction.
  The gate's line below is the red one this leaves.
- `crate::image` / `crate::shading` (987's): the *kind* of an alpha beside its value, one bit,
  closes §11.3.7.2's and §11.4.6's last two reports.

## Gates

The whole §2 sequence, this being a `pdf-model` change, run alone after the neighbouring
round's walk finished (load read before starting). Exit statuses read off the run.

| line | exit | what it printed |
|---|---|---|
| `cargo fmt --all --check` | 0 | |
| `clippy --workspace --all-targets`, `-D warnings` | 101 | one lint, `tools/conformance/tests/state_sections.rs:31` (an unfulfilled `expect`) — an untracked file of the tools round, mid-edit; `pdf-model` lints clean alone |
| `nextest run --workspace --no-fail-fast` | 100 | `4390 tests run: 4387 passed, 3 failed, 35 skipped` — `conformance::state_sections` (`tools/state.sh` runs `save_round_trip`, which `doc/todo/02` §2 does not list; the tools round's file), `conformance::bounded` (`--self-test`: "one sample cost 1273 ms", a timing floor read under the whole workspace's tests at once), `pdf-vfs-ffi::the_kio_worker` ("delete refused: … pages/0005.pdf does not exist"); the seven new and changed tests in `transparency.rs` pass |
| `test --workspace --doc` | 0 | |
| `fuzz/` fmt and clippy | 0, 0 | |
| corpus | 0 | `974 documents in 11.0s: 0 unopenable, 9 locked, 1 encrypted beyond us, 5 pageless, 61 incomplete, 0 slow` — unchanged from the merge's; no `TransparencyGroup` report anywhere in the run |
| oracle | 0 | `1956 pages in 61.1s (1857 we call complete, 99 incomplete)`; `agrees 990, contradicted 62, ambiguous 835, not comparable 47` — unchanged; `issue18032.pdf` listed nowhere, which is what an agreeing page looks like |
| text_extraction, selection_census, accessibility_census | 0, 0, 0 | `99.3% (24609/24788 words)`; PDFBox `99.8%` |
| launch_path (`--release`) | 101 | "the launch path moved": `open_kinstructions` 26773.808 against `26230 .. 26760` (WTPDF) and 185492.208 against `181751 .. 185424` (ISO 32000-2), both by under 0.1 % and both on `Document::open`, which nothing in this change reaches — `pdf-syntax`'s lexer was being rewritten beside it |
| dates, xmp, jpeg2000 | 0, 0, 0 | `1545 date strings … 1514 conform` |
| render-raster corpus | **101** | `958 pages compared in 33.5s: 931 agree, 21 differ, 6 refused, 16 not comparable`; `REFUSED_BEFORE_THE_SCENE` held to equality and `issue18032.pdf` is no longer refused — it **agrees with the CPU oracle** through raster. The one red line this change owns, and the edit is the colour round's file (above) |
| fixed_documents | 0 | |
| transform gate, writer/split/merge/pages/optimize/foreign corpus | 0 ×7 | |
| pdf-vfs write_corpus, read_corpus | 0, 0 | |
| conformance | 101 | the same `state_sections` test; every ledger and quotation check passes, the six edited rows included |
