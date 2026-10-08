# 1772 — The text rows' claims, read against their lines

Session 1468. Status: **accepted** — an audit, with one defect fixed and eleven stale sentences
corrected. Context: ADRs 1754 and 1765, which did the same for the colour, pattern, shading and
image rows. Rows: the 65 `implemented` rows under `9.`, and §10.5's note. Code:
`crates/pdf-font/src/metrics.rs`, `crates/pdf-font/src/lib.rs`,
`crates/pdf-model/src/rich_text/layout.rs`, `crates/pdf-model/tests/rich_text.rs`.

## 1. Why this is coverage work on `implemented` rows

Every `partial` leaf waits on an owner's answer (Q308, Q271, Q348, A66's trigger, a policy syntax no
signature names), so no round can move one this batch; a false claim in an `implemented` row is a
false `implemented` (ADR 1754 §1).

## 2. The method

ADR 1765's: the notes split on `[[clause]]`, every `\b(is|are) (read|applied|executed)\b` counted —
61 in 65 rows (`scratchpad/r1468/claims.py`) — each matched to the function and line that makes it
true; then every backticked name checked to exist (`names.py`), and each qualified name to be a
member of its type or module (`members.py`). Line numbers are this session's tree.

## 3. The claims

| row | claim | where it holds | verdict |
|---|---|---|---|
| 9.2.4 | the glyph bounding box read by nobody | `redact.rs` `declared_glyph_box` reads `d1`'s box | **false** (§4) |
| 9.3 | Table 102's nine parameters, Table 103's seven operators | `run.rs` 779–830; `/TK` `ext_gstate.rs` 447 | holds |
| 9.3.6 | stroke parameters in user space | `text.rs` `stroke_glyph` 1506; the clip at `ET` 1548 | holds |
| 9.3.8 | `/TK` read, ignored inside `BT`…`ET` | `ext_gstate.rs` 447, `!in_text` | holds |
| 9.3.8 | each glyph read under its `/AIS` | `text.rs` 1702, `open_knockout_reading_scope` | holds |
| 9.4 | the ten text-object operators executed (two claims) | `run.rs` 752–904; vertical `advance_step` `text.rs` 1407 | holds |
| 9.5 | the reader chosen by the leading bytes | `program.rs` `is_bare_cff` 606, asked at 100 and 226 | holds |
| 9.6 | single-byte codes, descriptor read; MacExpert from D.4 | `Code::single_byte` `cmap.rs` 113; `encoding.rs` 43, 424 | holds |
| 9.6.2 | Type1 and MMType1, both formats | `simple_font_subtype_tests` `loading.rs` 3697; `program.rs` 175 | holds |
| 9.6.4 | `/FontBBox` read by nobody | `redact.rs` `type3_codes` 2020 | **false** (§4) |
| 9.6.4 | `d1`'s colour operators ignored; a description read as a source | `run.rs` 485; `content/reader.rs` 145 (ADR 0427) | holds |
| 9.6.5.2 | built-in encoding read in both formats; `.notdef` unanswered | `Type1Font::code_to_glyph` 149, `NameKeyed::new` 54; `substitute_face` 213 | holds |
| 9.6.5.3 | `/BaseEncoding` under `/Differences` | `glyph_names.rs` 124 | holds |
| 9.6.5.4 | the two conditions | `substitute::states_latin_codes` 673 | holds |
| 9.7, 9.7.1, 9.7.2, 9.7.5.4 | two mappings; the first descendant only (five claims) | `loading.rs` 1147; `CidToGlyph::glyph` `composite.rs` 57 | holds |
| 9.7.3 | `/CIDSystemInfo` read by one function, three readers | `collection_names` 403, called from four places | holds; **a fourth reader** |
| 9.7.4 | read only for §9.7.5.2's compatibility rule | no compatibility check exists; the rule is §9.7.3's | **false** |
| 9.7.4.1 | Table 115's entries read; `/BaseFont` of the descendant | `metrics.rs` 416, 710, 784; `loading.rs` 562 | holds |
| 9.7.4.2 | `/CIDToGIDMap` two bytes at 2c; written into the descendant | `composite.rs` 63–69; `archive/fonts.rs` (ADR 1222) | holds |
| 9.7.4.3 | both arrays read without a program | `LoadedFont::metrics_only` 1365 (ADR 1094) | holds |
| 9.7.5 | 239 carried files | `data/cmaps/`, 239 after `build.rs`'s three exclusions | holds |
| 9.7.5.1 | `/WMode` both ways, `Vertical::read`; every requirement (two claims) | `composite.rs` 138; `metrics.rs` 708; `CMap::cid` 763 | holds |
| 9.7.5.3 | Table 118 in full | `read_cmap` `composite.rs` 189; `/WMode` 138; `/UseCMap` 225 | holds |
| 9.7.5.4 | an array one selector per code | `CMap::take_ranges` 582 (ADR 1092) | holds |
| 9.7.6 | Table 119 but `/BaseFont` | no read of the Type 0 dictionary's `/BaseFont` | holds |
| 9.8, 9.8.3, 9.8.3.1, 9.8.3.3 | the descriptor's entries; `/FD` applied; `/Lang`, `/Panose` (twelve claims) | `class_faces` `loading.rs` 600; `substitute::overridden` 524, `language` 629, `panose` 643; `glyph_class.rs` 401 | holds |
| 9.8.1 | four requirements; zero-byte program; the rest read by nothing | `no_program`; `/FontFamily` `rich_text/layout.rs` 617, `/CharSet` `pdf-archive` | **the last false** |
| 9.8.2 | `metrics::flag`, the word's one reader | `rich_text/layout.rs` 635 read the integer itself | **false** (§5) |
| 9.8.2 | Script, AllCap, SmallCap read by nobody | no read of bits 4, 17, 18 | holds |
| 9.9, 9.9.1 | Table 124's keys; `ttcf`; `/Length3` 0 executed (four claims) | `program.rs` 80–232; `collection::extract` 115; `with_fixed_content` 285 | holds |
| 9.10.1 | the replacement applied at `EMC` | `run.rs` 1067 | holds |
| 9.10.2 | `NoSubstitute`; the encoding read | `loading.rs` 1226 | holds |

The names: one did not exist — 9.10.2's `collection_meaning`, which is `composite::collection_table`.
Three file names held no such text: 9.6.4's and 9.3.1's `content.rs`, which are `content/run.rs` and
`content/ext_gstate.rs`. The rest exist and are members of what qualifies them.

## 4. The two boxes have a reader

9.2.4 and 9.6.4 said no code reads a Type 3 glyph's `d1` box or the font's `/FontBBox`. The redaction
reads both, as the extent a glyph's marks lie within (ADRs 1351, 1363), and an all-zero font box is
read as no box, so the marks are measured instead — Table 110's "shall make no assumptions" kept. No
drawing path reads either, so the conclusion stands; the sentences now say who reads them.

## 5. The defect: a flags word outside the range was read for its bits

§9.8.2: "The value of the Flags entry in a font descriptor shall be an unsigned 32-bit integer". The
note said `metrics::flag` was the word's one reader and read a negative or over-wide word as no
entry. `rich_text::layout`'s `family_of`, which decides whether a rich text face is italic or bold,
read `/Flags` as an `i64` and tested bits 7 and 19 itself, so `-64` read as bold italic. A bold run
under such a `/DA` face then asked for nothing the face did not already claim and was set in the
regular face. `pdf_font::descriptor_flags` is now the word, every bit clear outside the range;
`metrics::flag` and `family_of` both ask it. The fixture is `rich_text.rs`'s
`a_flags_word_outside_thirty_two_unsigned_bits_states_no_style`: on the old reader `Wide` comes out
in Helvetica where `/HeBo` is owed, and on the new one it draws the same page as `/Flags 32`.

`pdf-archive`'s `table/fonts.rs` `is_symbolic` still tests the integer's bit 3 itself. It judges a
file rather than drawing one, and it is outside this round's crates; the note names it.

## 6. §10.5's note

It still described ADR 0479's design: the function applied in `fill_paint`, `stroke_paint` and an
image's samples, mapped inside a shading's ramp, `mesh::transferred_corners` (no such function), a
shading built under a transfer "not cached" and a type 1 shading's device program withdrawn. Since
ADR 1266, no colour carries the function. `Interpreter::mark_transfer` puts it on the mark and the
transfer channel maps the finished pixel (ADR 1279). The shading cache keys by the resources'
`/ColorSpace` entry, a named one included (ADR 1765), and the program stays. The note now says that.
The brief described one stale sentence; it was a passage of five.
