# 997 — The kind of an alpha, carried where it is decided

2026-09-12. ADR 1017. Transparency, ISO 32000-2 §11.3.7.2, §11.4.6, §11.6.4.2, §11.6.4.3:
the two hand-offs session 988 left in the colour round's files — the *kind* of an image's and a
shading's alpha carried beside its value — which close the last two reports of §11.3.7.2's and
§11.4.6's rows. Five sibling rounds were live in the same tree (992, 993, 994, 995, 996).

Files: `crates/pdf-model/src/image.rs`, `crates/pdf-model/src/content/image.rs`,
`crates/pdf-model/src/content/transparency.rs`, `crates/pdf-render/src/shading.rs`,
`crates/pdf-model/tests/transparency_groups.rs` (one assertion, outside the round's list: it
pinned the very report this round closes, and now pins the drawing),
`doc/conformance/ledger.toml` (§11.3.7.2, §11.4.6, §11.6.4.3),
`doc/adr/1017-the-kind-of-an-alpha-carried-where-it-is-decided.md`, this file.

## What the round did

An image's alpha channel is one of §11.3.7.2's two quantities and the raster cannot say which:
§11.6.4.2 makes a stencil, an explicit mask and a colour-key mask *shape* and §11.6.4.3 makes
an `/SMask` or a non-zero `/SMaskInData` *opacity* under Table 57's default. `image::SampleAlpha`
is decided in `decode_parts` from the two facts that fix it — is this a stencil, was a soft mask
multiplied in — and `Parts` carries it. It does not go onto `pdf_render::Image`, where it
belongs: that struct is built by literal in six crates outside this round's files and serialised
by `viewer-confined`'s codec, so the interpreter records the kind where it draws the image
(`image::DrawnAlphas`, inside `MaskCache`, the one field of the interpreter's both files could
reach) and finds it again by the raster's identity when a knockout group closes. The soft mask
ADR 0151 builds out of a stencil painted through a pattern is recorded the same way, as shape.

A shading needs no bit: every colour it carries is opaque before `Shading::with_alpha` folds
§11.6.4.4's constant in (§8.6.6.4's `/None` aside, which this tree reads as opacity 0), and
where it paints is not in its colours. `Shading::opaque` is its shape — §11.6.4.2's "1.0 inside
and 0.0 outside the bounds of the shading's painting geometry" — and `with_alpha` and `opaque`
share one traversal now.

`transparency::stated_shape` states each: an image whose alpha is shape is itself at alpha 1.0;
one whose alpha is opacity has the clause's image rectangle, the unit square under its transform
filled, which is the very path the CPU oracle draws the image through; a fill or stroke through a
shading has the opaque shading; a fill through a stencil's mask keeps that mask. The two reports
are gone, and what is reported now is named: a stencil under an `/SMask` of its own
(`SampleAlpha::Both`), an image drawn outside the run, a paint of unknown kind. The implicit
callers (`path.rs`, `text.rs`) are not handed the record and draw as before.

Four fixtures in `transparency.rs`'s tests — a stencil, an `/SMask` image, a non-extending
`sh` at `ca ½`, and the stencil through a pattern, each over an opaque blue square inside an
isolated knockout group on a yellow page — hold §11.4.6's two stages to the pixel and were
calibrated three ways: the record answering nothing fails the first two, the shading's opaque
form withheld fails the last two, the stencil mask's record withheld fails the last alone. The
same four documents rendered through `open_one` with all three plants in and with none show
`(128, 0, 128)` become `(255, 128, 0)` under the stencil, the shading and the pattern, and blue
become the page under the `/SMask`'s zero.

Found on the way, and taken back to the clause: mupdf and ghostscript draw the `/SMask` fixture
with the blue surviving under the mask's zero, which is a knockout weighted by the image's alpha;
poppler draws the square blue throughout. §11.6.4.2 names the masks that modify an image's
shape and the soft mask is not one, §11.6.4.3 lists an image's `/SMask` among the ways the soft
mask is specified, and §11.4.6's NOTE 5 says what shape 1.0 at opacity 0 yields. This tree does
not follow the two references; ADR 1017 §5 has the reading and §11.3.7.2's row records it. mupdf
also knocks out under a `sh`'s whole clip where the clause bounds the shape by the painting
geometry; ghostscript and poppler draw that one as this tree does.

## Owed to other rounds' files

- `path.rs` (one site) and `text.rs` (two): `implicit_knockout_group` takes the record as a
  fourth argument, `self.image_masks.drawn()`, so a Type 3 stencil glyph among a text object's
  parts states its shape.
- `pdf_render::Image` owes `SampleAlpha` beside `interpolate`, and the record comes out when it
  lands; the constructors are the ones `grep -rn 'interpolate[:,]' --include=*.rs crates` names,
  and `viewer-confined`'s codec carries the bit.
- `doc/todo/23`'s two paragraphs ending "the *kind* of an image's or a shading's alpha carried
  beside its value" are answered by ADR 1017.

## Gates

The whole §2 sequence, this being a `pdf-model` and `pdf-render` change, run alone from
`tools/bounded.sh` for every walk, after reading the load (five siblings; the first launch was
stopped at its third line to fold in a fix and restarted from the top). Exit statuses read off
the run.

| line | exit | what it printed |
|---|---|---|
| `cargo fmt --all --check` | 0 | (a first launch found `pdf-transform` unformatted — a sibling mid-edit, formatted by the time of the run recorded here) |
| `clippy --workspace --all-targets`, `-D warnings` | 0 | (the first launch found one `too_many_lines` in `pdf-transform/src/archive/prepare.rs`, the sibling's, fixed by them before this run) |
| `nextest run --workspace --no-fail-fast` | 100, then 100 | first run `4417 tests run: 4415 passed, 2 failed, 36 skipped` — `transparency_groups::a_knockout_group_reports_only_where_the_two_models_differ`, whose last assertion required the translucent-shading report this round closes (assertion turned round, above), and `pdf-vfs-ffi::the_kio_worker`, the same failure 988 recorded; re-run alone after the sequence: `4424 tests run: 4423 passed (1 slow), 1 failed, 36 skipped`, the kio worker alone |
| `test --workspace --doc` | 0 | |
| `fuzz/` fmt and clippy | 0, 0 | |
| corpus | 0 | `974 documents in 10.7s: 0 unopenable, 9 locked, 1 encrypted beyond us, 5 pageless, 61 incomplete, 0 slow` — the same line as 988's; no `TransparencyGroup` report in the run |
| oracle | 0 | `1957 pages in 33.4s (1858 we call complete, 99 incomplete)`; `agrees 991, contradicted 62, ambiguous 835, not comparable 47`; no page moved into the contradicted set (the gate holds its lists by name and is green); 988's line had 1956 pages and 990 agreeing, the one page's difference being the merge's, not this change's |
| text_extraction, selection_census, accessibility_census | 0, 0, 0 | `99.3% (24609/24788 words)`; PDFBox `99.8% (14257/14281)`; drag `1000/1011 words selected (98.91%)`, find `1002/1002` |
| launch_path (`--release`) | 0 | |
| dates, xmp, jpeg2000 | 0, 0, 0 | `1545 date strings in 974 documents: 1514 conform` |
| render-raster corpus | 0 | `958 pages compared in 33.0s: 931 agree, 21 differ, 6 refused, 16 not comparable` — 988's counts, and green now that its name is off the list |
| fixed_documents | 0 | |
| transform gate, writer/split/merge/pages/optimize/foreign corpus | 0 ×7 | |
| pdf-vfs write_corpus, read_corpus | 0, 0 | |
| conformance | 0 | every ledger and quotation check passes, the three edited rows and the new blockquotes included — after one correction: the checker attributes a blockquote to the nearest clause cited before it, and `Shading::opaque`'s first draft named §11.6.4.4 last |

The four fixtures' pictures are the whole population of the change; the corpus states neither
element inside a knockout group (ADR 1009 §6), which is what the unchanged lines say.
