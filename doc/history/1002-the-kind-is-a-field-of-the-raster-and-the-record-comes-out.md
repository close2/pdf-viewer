# 1002 — The kind is a field of the raster, and the record comes out

2026-09-12. ADR 1022. The viewer: ISO 32000-2 §11.3.7.2, §11.4.6, §11.6.4.2, §11.6.4.3, §8.9.5
Table 87 — ADR 1017 §7's three owed items, taken together. Five sibling rounds were live in the
same tree (999, 1000, 1001, 1003, 1004).

Files: `crates/pdf-render/src/{lib.rs, paint.rs, shading.rs}`,
`crates/pdf-render/examples/area_bench.rs`, `crates/pdf-model/src/image.rs`,
`crates/pdf-model/src/content/{image.rs, path.rs, text.rs, transparency.rs}`,
`crates/pdf-model/tests/{transparency_groups.rs, raster_golden.tsv}`,
`crates/render-cpu/src/images.rs`, `crates/render-cpu/tests/image_placement.rs`,
`crates/render-gpu/tests/headless_gpu.rs`, `crates/render-raster/examples/filtered_edge_colour.rs`,
`crates/viewer-confined/src/protocol.rs`,
`crates/viewer-confined/src/protocol/{display_list.rs, panels.rs}`,
`crates/viewer-core/src/transition.rs`, `crates/viewer-host/src/clock.rs`,
`crates/viewer-ui/src/bin/quorra-confined/screen.rs`, `crates/viewer-ui/tests/panel.rs`,
`doc/conformance/ledger.toml` (§11.3.7.2, §11.4.6, §11.6.4.3), `doc/todo/23`,
`doc/adr/1022-the-kind-is-a-field-of-the-raster-and-the-record-comes-out.md`, this file.

## What the round did

**The field.** `pdf_render::SampleAlpha` sits beside `interpolate` on `pdf_render::Image`, with
`ImageAtDeviceScale::sample_alpha()` answering for a deferred source without producing a raster and
`ImageSource::sample_alpha()` the one call a reader makes. Every literal constructor the command
`grep -rn 'interpolate[:,]' --include=*.rs crates` names was updated in one pass, and each is a
decision rather than a default: `decode_parts` derives a document image's from the two facts that fix
it; `combine_on_the_finer_grid`, `MaskedAtDeviceScale` and `Image::area_averaged` carry the base's,
because averaging does not change what an alpha *is*; and every raster that is not a document's image
— a shading's mesh, radial and sampled rasters, §12.3.4's thumbnail, a viewport's own pixels, every
backend's fixture — is `Shape`, which is the rectangle it covers and which nothing in §11 reads.

**The record.** `image::DrawnAlphas` held two records and the field replaces only one. The image
half — the `ImageSource` cloned to pin its allocation, found again by `Arc::ptr_eq` — is gone with
`same_source` and `alpha_of`. The other half is the whole of the type now, renamed `image::ShapeMasks`
for what it is: ADR 0151's soft mask built out of a stencil painted through a pattern, which is
§11.6.4.2's shape wearing §11.6.4.3's vocabulary and which no field on a *raster* can ever say,
because the mask is on a fill.

**The wire.** `viewer-confined`'s display-list codec writes one byte after `/Interpolate`'s and reads
it back, `sample_alpha_tag` written out both ways so an addition to the enumeration is a build failure
in the codec. The bit is not recoverable from the samples — a stencil's `{0, 255}` and a one-bit soft
mask's are the same bytes — so it is carried or lost. §12.3.4's thumbnail is the one `Image` it is not
carried for, and the encoder's destructure names the field rather than eliding it with `..` so that
saying so stays a decision.

**The implicit callers.** `implicit_knockout_group` takes `&ShapeMasks` as a fourth argument, and
`path.rs`'s one site and `text.rs`'s two pass `self.image_masks.shape_masks()`. An image among
§11.7.4.4's or §9.3.8's implicit group's parts states its shape now; the sentence "an image drawn
outside this run, whose alpha's kind was not recorded" has no referent left and is gone.

**`SampleAlpha::Both` stays reported, and the price ADR 1017 quoted was wrong.** There is no second
decode anywhere: the deferred route already retains the unmasked stencil as `MaskedAtDeviceScale.base`,
and the eager route could keep it for an `Arc` clone. What is unaffordable is that one `Image` would
have to carry two rasters — a field every backend, both censuses and the codec carry, held for the
life of the display list, paid for every stencil under an `/SMask` whether or not a knockout group
asks. And the round owed a clause reading it had assumed the other way: **`Both` is conforming**, on three
sentences rather than one. Table 87's `/ImageMask` row enumerates what a stencil may not carry and
`/Mask`'s row says it twice — "If `ImageMask` is true, this entry shall not be present" — while
`/SMask`'s row says nothing of the kind; §8.9.6.2 lists the three ways "[a]n image mask differs from
an ordinary image" and they are those same three; and §8.9.6.1, the paragraph before it, adds the
soft mask to the masking effects an image dictionary may carry without excluding a stencil — "a
fourth type of masking effect, soft masking, is available through the SMask entry". Table 143
restricts the *soft-mask image's* dictionary rather than the parent's. The third of the three came
from `doc/habits/reading-the-specification.md`'s first line and would not have come from the row.

## The fixtures, and the calibration

`an_image_among_the_parts_states_the_shape_its_raster_names` (`content/transparency.rs`) is §11.4.6's
two stages over an element whose alpha is opacity: a 2×1 image, left sample transparent and right
green, over an opaque blue square it exactly covers. §11.6.4.2 gives it the whole rectangle for a
shape, so the blue is knocked out under all of it, and NOTE 5 leaves the initial backdrop where the
opacity is 0 — the page on the left, the image on the right. Planted two ways: `stated_shape`'s image
arm answering `None` (the old unrecorded raster) refuses the group; the `Opacity` arm stating the image
itself — mupdf's and ghostscript's reading, ADR 1017 §5 — draws blue where the clause gives the page.

The codec round trip was planted by making the decoder's `Opacity` arm answer `Shape`:
`what_decodes_re_encodes_to_the_same_bytes` and `a_whole_page_round_trips_to_an_equal_list` both fail,
and both pass with the arm right. The fixture `an_image` carries `Opacity` deliberately, so a codec
that dropped the field would disagree rather than agree with a default.

ADR 1017's four fixtures were not touched and still pass, which is what says the field carries the
same values through a real decode that the record carried: they drive `decode_parts` from written
documents and hold §11.4.6's two stages to the pixel.

## The golden

`PDFVIEWER_RASTER_GOLDEN=update` moved **231 of 974** entries and every one of them is `list only (an
interpreter change no pixel shows)`. Checked mechanically against the diff rather than taken from the
classification: on all 231 lines the outcome word, the extent, the raster digest and the reports digest
are identical and the display list's digest alone moved. That is what a new `Debug` field on `Image`
predicts, and it is also the evidence that nothing structural moved — a knockout group that had been
reporting and now draws changes the raster *and* the reports, and neither moved anywhere in the
corpus. The 231 are the pages whose list holds an `Image`, a `Command::Image` or a shading's own
raster; `image_region_census` counts 534 rasters over 4540 painting operations on 959 first pages.
Trap 1's own instrument is the one built to answer this (ADR 1016 §3), and the oracle and
`render-raster`'s corpus gate are the two independent confirmations below.

## The other half: the sweep's head, read to a clause

`doc/todo/02` §1's second track. The ink sweep was run over this round's own oracle log
(`cargo run --release -p pdfref --bin undrawn`), and its fourth row — `bug1050040.pdf` page 1,
**−11.272**, ours 0.000 against `hayro` 11.272 — was opened rather than passed over. It is ADR
0836's check-value refusal: a 200×50 page showing `"Bug 1050040"` in an embedded TrueType face
whose `/FontFile2` reaches RFC 1951's final block and then fails RFC 1950's Adler-32, so the font
is refused and the page is blank while all four references draw the line legibly and identically.

The refusal prints a claim — "these are not the bytes that were compressed" — and this round found
**two independent statements that test it, both of which agree**:

- **ISO 32000-2 §9.9 Table 125.** `/Length1` is "the entire TrueType font program, after it has
  been decoded using the filters specified by the stream's Filter entry, if any", and the file
  states 59212 where the filter delivers 59211.
- **The font's own per-table `checkSum` fields.** `cvt `, `fpgm`, `hhea`, `hmtx` and `prep` — every
  table ending before byte 12951 — check correctly *in place*; `loca`, `cmap`, `post`, `name`,
  `maxp`, `OS/2` and `head` check correctly *exactly one byte early*; `glyf`, which spans 12952 to
  52932, checks at no offset at all. One byte is missing from inside the outline table, and
  `loca`'s offsets point one byte past where the outlines now are.

§7.4.4.1 is why the Adler-32 counts at all: the Flate method "is fully defined in Internet RFC
1950 , and Internet RFC 1951", and RFC 1950's own compliance clause requires a decompressor to
check ADLER32 and give an error indication. So the refusal is right, and ADR 0459 decides the page.
What the reading leaves owed is one line: `pdf_font::program::whole_program` reads Table 125's
`/Length1` on `Damage::Truncated` and not on `Damage::CheckValue`, where on this witness it would
have corroborated the check value exactly. Written into `doc/todo/00` §7.

## Gates

The whole §2 sequence, this being a `pdf-render`, `pdf-model` and `render-cpu` change — rule 2 and
rule 3 both. Every walk under `tools/bounded.sh`. Five siblings were live in the same tree
throughout and four of the sequence's lines are red in **their** files; each is named with the crate
it is in, and every one of them was re-checked against this round's own nine crates separately.
The reference-spawning lines were run at a load the machine's own desktop accounts for, with the
oracle's reference cache at a **100.0% hit rate** — 6750 renders from the cache, 0 produced — so no
reference was measured under contention at all.

| line | exit | what it printed |
|---|---|---|
| `cargo fmt --all --check` | 1 | eleven files, every one of them `pdf-transform`'s or `pdf-archive`'s (siblings 999 and 1001, mid-edit). `cargo fmt -p <crate> --check` over this round's nine crates is silent |
| `clippy --workspace --all-targets`, `-D warnings` | 101 | two errors, both in `crates/pdf-archive/src/reach.rs` (sibling 1001). The same command over this round's crates with the same flag is silent |
| `nextest run --workspace --no-fail-fast` | 100 | `4447 tests run: 4434 passed (1 slow), 13 failed, 36 skipped` — twelve in `pdf-transform::archive` and one `conformance` quotation for `pdf-transform/src/tool.rs`, all sibling 999's; none in any crate this round touched |
| `test --workspace --doc` | 0 | |
| `fuzz/` fmt and clippy | 0, 0 | |
| corpus | 0 | `974 documents in 10.7s: 0 unopenable, 9 locked, 1 encrypted beyond us, 5 pageless, 61 incomplete, 0 slow` — 997's line to the tenth |
| raster_golden | 101, then 0 | `held 743, moved 231` before; `PDFVIEWER_RASTER_GOLDEN=update` regenerated it and the re-run says `974 tracked documents on disk, 966 first pages drawn, 974 entries …; held 974, moved 0, unheld 0, left 0`. Every one of the 231 was `list only`; the diff is above |
| oracle | 0 | `1957 pages in 53.9s (1858 we call complete, 99 incomplete)`; `agrees 991, contradicted 62, ambiguous 835`, 100.0% reference cache hit rate — **997's three counts unchanged**, which is the second instrument saying no pixel moved |
| text_extraction | 0 | `overall 99.3% (24609/24788 words)`; PDFBox `99.8% (14257/14281)` |
| selection_census, accessibility_census | 0, 0 | drag `1000/1011 words selected (98.91%)`, find `1002/1002` |
| launch_path (`--release`) | 0 | |
| dates, xmp, jpeg2000 | 0, 0, 0 | `1545 date strings in 974 documents: 1514 conform to §7.9.4 (97.99%)` |
| save_round_trip, actions, `pdf-syntax` on_disk | 0, 0, 0 | |
| render-raster corpus | 0 | `958 pages compared in 38.6s: 931 agree, 21 differ, 6 refused, 16 not comparable` — 997's line exactly, and the **third** instrument saying no pixel moved |
| fixed_documents | 0 | |
| transform gate, writer/split/merge/pages/optimize/foreign corpus | 0 ×7 | |
| `pdf-archive` corpus, `pdf-transform` archive_corpus | 0, 0 | |
| `pdf-vfs` write_corpus, read_corpus | 0, 0 | |
| `viewer-confined` awkward_classes | 0 | |
| conformance | 101 | `crates/pdf-transform/tests/archive.rs:972` and `:987`: "§8.8.2 is not a clause of ISO 32000-2" — sibling 999's, and the only failure. The quotation half is green: `1377 quotations, all verbatim within the clause they cite`, this round's new blockquote and inline quotations included. Run alone before the sibling's edit landed, the whole crate was `exit 0` |
| `--bin quotations`, `--bin pointers` | 0, 0 | documents changed, so both were run; neither names a line of this round's |

Trap 10's four `--bins` builds — `pdf-sandbox` under `gates` and again under `--release`, `pdf-vfs`,
`viewer-confined` — were run before the lines that need them, and `hayro-compare`'s `pdfref-hayro`
before the oracle.

## Owed to other rounds' files, and what was taken anyway

`crates/render-gpu/`, `crates/render-raster/`, `crates/viewer-core/`, `crates/viewer-host/` and
`crates/viewer-ui/` are not this round's, and one line was added in each: a struct literal of
`pdf_render::Image` cannot omit a field, so `cargo check --workspace` is red until every one of them
carries it. The alternative — the field added and the workspace left unbuildable for five siblings —
is the failure this round was restarted to avoid.
