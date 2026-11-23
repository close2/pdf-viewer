# The silent-repair inventory of `pdf-syntax`, `pdf-model` and `pdf-font` — round 2194

The first step of the owner's campaign of 2026-11-23 ([ADR 3477](../adr/3477-every-repair-the-reader-makes-reaches-the-validator-on-one-channel.md)):
every place the three reader crates repair, recover, substitute or tolerate instead of reading
what the file says, and which of those reach the PDF/A validator (`pdf-archive`) without a report.
Made in the shape of the owner's review 2038 ([`2038-specification-audit.md`](2038-specification-audit.md))
and of audits 2128 to 2197. Baseline: main at `da7f8269`. Evidence:
[`2194-silent-repairs/`](2194-silent-repairs/) — `findings.json` (2038's schema) and
`expected-failures.json` (the failing tests of each behaviour finding), also entered in
`2038-specification-audit/expected-failures-current.json`.

**This is an audit and fixes nothing.** The fix rounds take the findings in section 6's order.

## 1. Method

1. A census per crate, by grep and reading: every `unwrap_or*`, `.ok()`, `or_else`, `filter_map`
   and `.clamp(` on a value read from the file, every recovery path (cross-reference rebuild, a
   stream's extent, truncated data, missing keywords, damaged dictionaries, object streams), every
   substitution (fonts, encodings, colour spaces, boxes, resources) and every tolerance (lexer,
   operand types and counts, generation numbers, out-of-range values). A default ISO 32000-2
   states for an *absent* entry is not a repair and was not filed; a present entry of the wrong
   type read as absent was. `pdf-syntax` and `pdf-font` were read whole except their writers
   (`serialize`, `write`, `linearize`, the font embedders), which were sampled; `pdf-model` was
   read for the modules on the validator's path and the content interpreter (`content/`,
   `page.rs`, `inheritance.rs`, `inline_image.rs`, `type3.rs`, `xmp.rs`, `structure*`,
   `retrieval.rs`, `outline.rs`, `output_intent.rs`, `annotation.rs`, `jpeg2000.rs`, `image*`).
   The rest of `pdf-model` (forms, XFA, rich text, media, view, geospatial and the rest) is not in
   this inventory: none of it is on `pdf-archive`'s path, and it is the fix rounds' second pass.
2. `pdf-archive`'s intake was mapped: `check` takes a parsed `Document`; its population is the
   (possibly rebuilt) table's object numbers (`examination.rs:150`); seven rows re-read raw bytes
   (`file_header`, `nothing_after_the_end`, `stream_keyword_line_endings`,
   `indirect_object_syntax`, `cross_reference_keyword_line_endings`, `hexadecimal_string_digits`,
   the signature digest's byte range); the survey walks content *tokens* through
   `ContentReader::for_page` and never interprets.
3. Each site's reach was classified: **silent** (on the validator's path, and nothing it consumes
   records the repair), **reported** (the validator consumes the record), **caught by a raw row**
   (a raw-byte row sees the same damage on its own), **off-path** (the validator never runs the
   code — which is not harmless: it means the validator certifies a file the viewer had to repair,
   because it never looks where the repair is made).
4. Every behaviour finding was probed before it was filed: a twin file, and the same file with
   one damage, held to `check(…, Target::Two(Level::U))`; the finding is that the damaged file
   fails no requirement its twin does not. 45 damages were probed; the ones the validator does
   report are in section 3.
5. The census lines were gathered by three Sonnet helpers and classified by a fourth; the round
   read the sites behind every B finding and spot-checked others (`parser.rs:556` the stream
   extent, `objects.rs:79` the null on a failed load, `lexer.rs:828` the exponent read as a
   real, `decoding.rs:23` the dropped `Damage`, `sfnt.rs:471`, `program.rs:288`, `run.rs:268`,
   `colour.rs:609`, `file_structure.rs:2902`). A census line not behind a finding is the census's
   reading, marked as such by this paragraph; the fix round that takes a site reads it again.

## 2. What exists, and what reaches the validator

| signal | crate | consumed by `pdf-archive` |
|---|---|---|
| `filter::Damage` via `decode_with_parms_reported` | syntax | ASCII stages (`file_structure.rs:984`), font programs (`fonts.rs:1801`); every other decode calls `decoded_stream_data`, which drops it (`decoding.rs:23`) |
| `Lexer::integer_out_of_range`, `Lexer::salvaged` | syntax | content streams only, `Runs::AsWritten` (`survey/pages.rs:44`, `survey/xobjects.rs:171`) |
| `Lexer::hexadecimal_strings` | syntax | `file_structure.rs:2303` |
| `was_recovered`, `CompressedRecovery`, `objects_lost_to_damage`, `misfiled_objects`, `padded_stream_keywords`, `cross_reference_entries_lost`, `cross_reference_chain_was_cut`, `damaged_dictionaries` (`document/reports.rs`) | syntax | **none** |
| `Unsupported`, `StreamSyntaxFault`, `ContentIssue`, `DamagedStream` (`content/report.rs`, `page.rs:2003`) | model | **none** |
| `Page::substituted_media_box`, `Page::damaged_dictionary`, `Pages::found_by_scanning`, `CharProcsDamage` | model | **none** |
| `LoadedFont::is_substituted`, `FontError::Malformed`, `program_unparsed` | font | font rows |
| `FontDictRepair`, `sfnt::repaired_font_program`, `cmap_truncated`, `to_unicode_truncated` | font | **none** |

Section 5.1's row, `conformance/adheres-to-the-base-standard`, is `Check::Unchecked`
(`table/file_structure.rs:66`), and `Report::verdict` counts only `Failed` rows: every repair
whose specific row does not re-read the bytes leaves a verdict of `Conforms`.

## 3. What the validator already reports (probed, not findings)

A wrong `/Length` either way and a missing `endstream` (`stream-length-matches-the-data`, by the
parsed length against the data the parser kept); a number like `1.2.3` and an unknown operator in
content (`only-operators-the-base-standard-defines`); an undefined resource
(`named-resources-are-defined`); an odd hex string (`hexadecimal-string-digits`); a wrong page
`/Count` (`page-tree-node-count-counts-its-leaves`); a damaged page dictionary (the page-tree rows);
`/Widths` short, wrong, non-numeric or under a non-integer `/FirstChar` on an embedded font
(`widths-array-covers-character-range`, `widths-agree-with-the-program`).

Probed and silent, but not filed because the reader's behaviour is the clause's: an unknown escape
in a literal string and an octal escape's high-order overflow (§7.3.4.2 prescribes both).

## 4. Findings

Eight behaviour findings (each with failing tests in `crates/pdf-archive/tests/silent_repairs_spec_audit.rs`), six viewer-side findings, no documentation finding: no comment read for this inventory misdescribes the repair beside it.

### 2194-B01 — ISO 19005 6.1.13

An integer past 64 bits in a dictionary is read as the nearest real, and implementation-limits/integer-values reads the real. ADR 3393 carried Lexer::integer_out_of_range to the validator for content streams only (Runs::AsWritten); the parser path never asks it.

- Source: ISO 32000-2 §7.3.3 (an integer is a run of digits whatever its length); ISO 19005-2 section 6.1.13 and ISO 19005-4 section 6.1.13 bound integers (paraphrased).
- Code: crates/pdf-syntax/src/lexer.rs:822 (wide_integer set, Token::Real returned); crates/pdf-syntax/src/parser.rs (no read of integer_out_of_range); crates/pdf-archive/src/table/file_structure.rs integer-values
- Change: Log the class syntax/integer-past-i64 with the object, and map it to integer-values.
- Tests: `an_integer_past_sixty_four_bits_in_a_dictionary_is_an_integer_silent_repair`

### 2194-B02 — ISO 19005 5.1, 6.1.4, every content row

An indirect object the reader cannot load — bytes that do not parse, or a table entry whose offset holds another object's header — is cached as Object::Null, the value of an undefined reference. The page's content stream leaves the walk, and the damaged file fails fewer rows than its twin. misfiled_objects records the second case; no row asks it.

- Source: ISO 32000-2 §7.3.10 (null is the value of a reference to an object that does not exist, not of one that exists and is damaged); §7.5.4 (the entry's offset is the object's).
- Code: crates/pdf-syntax/src/document/objects.rs:79 (load(id).unwrap_or((id.generation, Object::Null))); crates/pdf-syntax/src/document/reports.rs:123 misfiled_objects
- Change: Log syntax/object-unreadable and syntax/object-misfiled; map both to 5.1 and to the content rows' completeness (a page whose content was lost is not certified).
- Tests: `an_unparsable_content_stream_object_is_not_an_empty_page_silent_repair`, `a_cross_reference_entry_at_another_objects_header_is_not_null_silent_repair`

### 2194-B03 — ISO 19005 6.1.9

An indirect object with no endobj is read whole, and file-structure/indirect-object-syntax's end_of_object returns when it finds no keyword to judge — the validator's own tolerance stacked on the reader's.

- Source: ISO 32000-2 §7.3.10 (an indirect object is bracketed by obj and endobj); ISO 19005-2 section 6.1.9 sets the endobj keyword's end-of-line markers (paraphrased).
- Code: crates/pdf-archive/src/table/file_structure.rs:2926 (rposition … else { return; }); crates/pdf-syntax/src/parser.rs parse_indirect_object (endobj optional)
- Change: Log syntax/endobj-missing; end_of_object records a finding where the region holds no endobj.
- Tests: `an_object_without_endobj_fails_the_object_syntax_silent_repair`

### 2194-B04 — ISO 19005 5.1, 6.1.4

Cross-reference repairs: offsets that miss their objects are followed to the objects without a rebuild or a record; a /Prev chain that loops is rebuilt by scanning (Document::was_recovered). The validator's population is the repaired table and no row asks either.

- Source: ISO 32000-2 §7.5.4 (each entry's offset is the object's), §7.5.6 (/Prev is the previous section's offset).
- Code: crates/pdf-syntax/src/document/objects.rs:277 (offset search); crates/pdf-syntax/src/xref.rs:1091 (/Prev cycle); crates/pdf-syntax/src/document/reports.rs:106 was_recovered; crates/pdf-archive/src/examination.rs:150
- Change: Log syntax/xref-offset-searched, syntax/xref-rebuilt, syntax/xref-chain-cut, syntax/xref-entries-lost; map to 5.1 (and 6.1.4 where the table's own syntax is the subject).
- Tests: `cross_reference_offsets_that_miss_their_objects_are_reported_silent_repair`, `a_prev_chain_that_loops_is_reported_silent_repair`

### 2194-B05 — ISO 19005 6.2.11.4.1 / 6.2.10.4.1

An embedded TrueType program the reader repaired (head.indexToLocFormat wrong: sfnt::repaired_font_program rewrites the loca format) or could only partly read (cut in half) loads without FontError::Malformed, so program_unparsed and every font row pass it. fonts/font-programs-conform-to-their-own-specifications is Unchecked.

- Source: ISO 32000-2 §9.9 (the embedded program is the font program in its own format); ISO 19005-2 section 6.2.11.4.1 (the embedded program is what a reader renders, and it defines every glyph used), paraphrased.
- Code: crates/pdf-font/src/sfnt.rs:471 repaired_font_program (with :183, :385, :519); crates/pdf-archive/src/table/fonts.rs:1840 program_unparsed
- Change: Log font/sfnt-loca-extent, font/sfnt-loca-format, font/sfnt-loca-order and font/program-truncated; map to font-programs-embedded and the conform row.
- Tests: `a_truetype_program_whose_loca_format_is_wrong_is_reported_silent_repair`, `a_truetype_program_cut_in_half_is_reported_silent_repair`

### 2194-B06 — ISO 19005 5.1

Base-standard syntax tolerated with no row: a repeated dictionary key (first kept), an unterminated literal string (runs to the end of the stream), a key that is not a name (skipped). Section 5.1's row is Unchecked, so the verdict is Conforms.

- Source: ISO 32000-2 §7.3.7: "Multiple entries in the same dictionary shall not have the same key" and every key is a name; §7.3.4.2 (balanced parentheses).
- Code: crates/pdf-syntax/src/parser.rs:384 (duplicate), :443 (non-name key); crates/pdf-syntax/src/lexer.rs:591 (unterminated string)
- Change: Log syntax/duplicate-key, syntax/key-not-a-name, syntax/string-unterminated; section 5.1's row becomes Implemented over the log (ADR 3477).
- Tests: `a_dictionary_with_a_repeated_key_is_reported_silent_repair`, `an_unterminated_literal_string_is_reported_silent_repair`, `a_dictionary_key_that_is_not_a_name_is_reported_silent_repair`

### 2194-B07 — ISO 19005 5.1

Page entries of the wrong type or value are substituted: a /MediaBox that is a name (Page::substituted_media_box), a /Rotate of 45 (read as 0). Neither record reaches the validator.

- Source: ISO 32000-2 §7.7.3.3, Table 31 (MediaBox a rectangle; Rotate "shall be a multiple of 90").
- Code: crates/pdf-model/src/page.rs:301 MediaBoxSubstitution, :281 (Rotate)
- Change: Log model/media-box-substituted and model/rotate-not-a-multiple; map to 5.1.
- Tests: `a_media_box_that_is_not_a_rectangle_is_reported_silent_repair`, `a_rotation_that_is_not_a_multiple_of_ninety_is_reported_silent_repair`

### 2194-B08 — ISO 19005 5.1, 6.2.4.3

Content operators given operands they do not take are repaired by the interpreter (an operator with a wrong-typed operand does nothing; one short of operands is dispatched with what is there; Q with nothing saved is ignored; Tj outside BT is drawn; three sc operands under DeviceGray are read as DeviceRGB). Several are reported to the viewer (Unsupported::OperandShortfall, StreamSyntaxFault::TextOperatorOutsideATextObject); the survey walks tokens and never interprets, so none reaches the validator — and the colour case paints in a space the validator never records.

- Source: ISO 32000-2 §8.2 and the operator tables (Table 58, Table 73), §8.4.2 ("Occurrences of the q and Q operators shall be balanced within a given content stream"), §9.4.1.
- Code: crates/pdf-model/src/content/run.rs:268; crates/pdf-model/src/content/run/graphics.rs:48, :66; crates/pdf-model/src/content/colour.rs:609; crates/pdf-archive/src/survey.rs (token walk)
- Change: Either the survey takes the interpreter's Unsupported/StreamSyntaxFault for each page (one walk, ADR 3477's log), or the content reader logs its classes as it tokenises; map to 5.1, and the colour case to 6.2.4.3's rows.
- Tests: `an_operand_of_the_wrong_type_is_reported_silent_repair`, `an_operator_short_of_its_operands_is_reported_silent_repair`, `a_restore_with_nothing_saved_is_reported_silent_repair`, `a_text_operator_outside_a_text_object_is_reported_silent_repair`, `colour_operands_that_do_not_fit_the_space_are_reported_silent_repair`

### 2194-T01 (viewer)

decoded_stream_data drops filter::Damage: every caller of the unreported API (images, xref streams, object streams, ICC profiles, metadata) takes a damaged prefix as a whole decode.

- Code: crates/pdf-syntax/src/document/decoding.rs:23; crates/pdf-syntax/src/document/images.rs:70; crates/pdf-syntax/src/xref.rs:1933
- Note: Inventory S040, S044 and the xref-stream row; the fix is the channel's first class.

### 2194-T02 (viewer)

Object-stream and xref-stream losses the existing reports miss: a member that fails to parse is dropped and not added to objects_lost_to_damage; a missing /N reads as 0 with short_by 0; non-integer /Index pairs are dropped and the subsections shift.

- Code: crates/pdf-syntax/src/document/object_streams.rs:67, :126; crates/pdf-syntax/src/xref.rs:1605
- Note: Silent to the viewer's own reports as well as to the validator.

### 2194-T03 (viewer)

Font maps read leniently with no record: a damaged ToUnicode keeps the prefix that parsed, an absent and a damaged one are one empty map, a damaged Encoding CMap skips entries, a malformed /W or /W2 stops at the first bad entry, a malformed /Differences is skipped item by item, an /Encoding name outside Table 112 becomes no encoding.

- Code: crates/pdf-font/src/tounicode.rs:190; crates/pdf-font/src/loading.rs:1369; crates/pdf-font/src/cmap.rs:389, :577, :615; crates/pdf-font/src/metrics.rs:737, :1113; crates/pdf-font/src/glyph_names.rs:112, :160
- Note: Probed: a damaged ToUnicode on a non-symbolic TrueType font passes (6.2.11.7.2 exempts the font, so 5.1 only).

### 2194-T04 (viewer)

CFF and Type 1 programs read leniently: an unreadable subroutine INDEX or Private DICT becomes none, a TrueType collection is cut to one face (face 0 where /FontName matches none), a failed hinting instance is dropped.

- Code: crates/pdf-font/src/cff.rs:2455, :2489, :2512; crates/pdf-font/src/program.rs:116; crates/pdf-font/src/collection.rs:58; crates/pdf-font/src/loading/drawing.rs:256
- Note: FontDictRepair is the one CFF repair already recorded.

### 2194-T05 (viewer)

Interpreter tolerances silent to the viewer's own Unsupported: extra operands before an operator dropped; EMC, EX and BX unbalanced; a BDC /OC naming no property list shows the content; an ExtGState entry of the wrong type ignored and an unparsable transfer function discarded; a form's malformed /Matrix read as identity and /BBox as no clip.

- Code: crates/pdf-model/src/content/run.rs:268, :454; crates/pdf-model/src/content/run/marked_ops.rs:77, :141; crates/pdf-model/src/content/run/compat_type3_ops.rs:11; crates/pdf-model/src/content/ext_gstate.rs:200, :289; crates/pdf-model/src/content/xobject.rs:151, :169
- Note: The viewer's report (trap 5) misses these whatever the validator does.

### 2194-T06 (viewer)

A /P that does not read as an integer becomes 0, which states permissions; a string that cannot be decrypted becomes empty.

- Code: crates/pdf-syntax/src/crypt.rs:404; crates/pdf-syntax/src/encryption.rs:268
- Note: PDF/A forbids encryption (6.1.3), so the validator fails such a file anyway; the viewer's silence is the finding.

## 5. The inventory

Every census site, in census order (S: `pdf-syntax`, M: `pdf-model`, F: `pdf-font`). Paths are under `crates/`. "reaches pdf-archive" is section 1 item 3's classification; the rule column paraphrases ISO 19005-2 by section number.

| # | site | kind | the file says | read instead | report | reaches pdf-archive | ISO 19005 rule |
|---|---|---|---|---|---|---|---|
| S001 | pdf-syntax/lexer.rs:466 | tolerance | lone `>` not part of `>>` | returned as Keyword(">") and the cursor advances | none | silent | 5.1 only |
| S002 | pdf-syntax/lexer.rs:472 | tolerance | unmatched `)` | returned as Keyword(")") | none | silent | 5.1 only |
| S003 | pdf-syntax/lexer.rs:554 | tolerance | `#` not followed by two hex digits in a name | `#` kept literally as byte 0x23 | none | silent | 6.1.8 (name escapes) |
| S004 | pdf-syntax/lexer.rs:591 | recovery | literal string unterminated (backslash at end or no closing paren) | string ends at end of input, bytes read so far returned as the string | none | silent | 5.1 only |
| S005 | pdf-syntax/lexer.rs:629 | not a repair | octal escape with high-order overflow (\777) | low byte kept (value & 0xff) | none | silent | ISO 32000-2 §7.3.4.2 (overflow ignored); 5.1 only |
| S006 | pdf-syntax/lexer.rs:632 | not a repair | backslash before a character that is not a defined escape | backslash dropped, the character stands for itself | none | silent | ISO 32000-2 §7.3.4.2 (backslash ignored); 5.1 only |
| S007 | pdf-syntax/lexer.rs:687 | tolerance | non-hex, non-whitespace byte inside <...> | byte skipped; digits before and after joined | HexadecimalStrings::strayed | reported | 6.1.6 (hex string digits) |
| S008 | pdf-syntax/lexer.rs:707 | not a repair | odd number of hex digits | final digit padded with 0 | HexadecimalStrings::completed | reported | ISO 32000-2 §7.3.4.3 (missing final digit is 0); 6.1.6 (even digit count) |
| S009 | pdf-syntax/lexer.rs:655 | recovery | hex string with no closing `>` at end of input | ends at end of input | none | silent | 5.1 only |
| S010 | pdf-syntax/lexer.rs:822 | tolerance | integer too large for i64 | read on as the nearest real | Lexer::integer_out_of_range | reported (content streams only) | 6.1.13 (integer bound) |
| S011 | pdf-syntax/lexer.rs:828 | tolerance | number spelled with an exponent or infinite magnitude (1e5, 1e999) | parsed as f64; infinity clamped to +-f64::MAX | none | silent | 6.1.13 (real bound) |
| S012 | pdf-syntax/lexer.rs:835 | tolerance | run like 1.2.3 or 12pt that is not a §7.3.3 number | number read off the front of the run (1.2 / 12), rest dropped | Lexer::salvaged | reported (content streams only) | 6.2.2 (operators) |
| S013 | pdf-syntax/lexer.rs:840 | tolerance | run with a digit but no number grammar before it (.-1, --5) | returned as a keyword, not a number | none | reported (content streams only) | 6.2.2 (operators) |
| S014 | pdf-syntax/parser.rs:277 | tolerance | `N G R` whose number or generation is out of range | read as plain integer N with the following tokens left to be parsed as separate objects | none | silent | 6.1.13 (indirect object bound) |
| S015 | pdf-syntax/parser.rs:370 | not a repair | dictionary entry with a direct null value | entry dropped as if absent | none | silent | ISO 32000-2 §7.3.9 (null entry equals absent); 5.1 only |
| S016 | pdf-syntax/parser.rs:384 | tolerance | duplicate key in a dictionary | first occurrence kept, later ones dropped | none | silent | 5.1 only |
| S017 | pdf-syntax/parser.rs:443 | tolerance | non-name token where a key belongs | token skipped, dictionary continues | none | silent | 5.1 only |
| S018 | pdf-syntax/parser.rs:518 | tolerance | spaces/tabs between `stream` keyword and its EOL | padding skipped, data starts after the EOL | Document::padded_stream_keywords | caught by raw row stream_keyword_line_endings | 6.1.7.1 (stream keyword EOL) |
| S019 | pdf-syntax/parser.rs:497 | tolerance | `stream` keyword followed by CR alone | CR taken as the end-of-line marker | none | caught by raw row stream_keyword_line_endings | 6.1.7.1 (stream keyword EOL) |
| S020 | pdf-syntax/parser.rs:556 | recovery | direct /Length absent, not an integer, negative, or not followed by `endstream` | data extent found by searching for `endstream` (EOL before it trimmed) | none | caught by raw row stream_length_matches_the_data | 6.1.7.1 (stream /Length) |
| S021 | pdf-syntax/parser.rs:560 | recovery | no `endstream` anywhere after the data | data runs to end of input | none | caught by raw row stream_keyword_line_endings | 6.1.7.1 (endstream EOL) |
| S022 | pdf-syntax/parser.rs:580 | tolerance | missing `endstream` keyword after the data | cursor rewound, object accepted | none | caught by raw row stream_keyword_line_endings | 6.1.7.1 (endstream EOL) |
| S023 | pdf-syntax/parser.rs:605 | tolerance | missing or misplaced `endobj` | cursor rewound, object accepted | none | caught by raw row indirect_object_syntax | 6.1.9 (endobj EOL) |
| S024 | pdf-syntax/parser.rs:649 | tolerance | generation number outside u16 | generation read as 0 | none | silent | 6.1.9 (indirect object header) |
| S025 | pdf-syntax/parser.rs:508 | tolerance | keyword_end beyond input | empty slice used as padding probe | none | silent | — |
| S026 | pdf-syntax/parser.rs:571 | recovery | computed data range outside the input | empty data slice | none | silent | — |
| S027 | pdf-syntax/document/parsing.rs:35 | recovery | indirect object at the stated offset fails to parse | parse error dropped, None returned; caller tries load_by_header, finally Null | none | silent | 6.1.9 (indirect object header) |
| S028 | pdf-syntax/document/parsing.rs:66 | recovery | indirect /Length that does not equal the parsed extent | extent re-taken from the stated length when endstream follows it, else the parser's scanned extent is kept | none | caught by raw row stream_length_matches_the_data | 6.1.7.1 (stream /Length) |
| S029 | pdf-syntax/document/parsing.rs:79 | tolerance | indirect /Length not an integer or beyond max_stream_len | scanned extent kept | none | silent | 6.1.7.1 (stream /Length) |
| S030 | pdf-syntax/document/objects.rs:79 | fallback | object cannot be loaded (parse failed, entry bad, header scan finds nothing) | Object::Null and cached as Null | none | silent | 5.1 only |
| S031 | pdf-syntax/document/objects.rs:157 | tolerance | reference chain longer than MAX_REFERENCE_DEPTH | Object::Null | none | silent | 5.1 only |
| S032 | pdf-syntax/document/objects.rs:77 | tolerance | object being loaded again on same thread (reference cycle through load) | Object::Null | none | silent | 5.1 only |
| S033 | pdf-syntax/document/objects.rs:277 | recovery | xref offset points at nothing or at another object's header | object found by scanning for its own header (load_by_header) | Document::misfiled_objects | silent | 6.1.4 (cross-reference offsets) |
| S034 | pdf-syntax/document/objects.rs:261 | recovery | section declared entries this reader never read (chain cut / entry unreadable) | object found by its own header | Document::misfiled_objects | silent | 6.1.4 (cross-reference offsets) |
| S035 | pdf-syntax/document/objects.rs:288 | substitution | compressed object absent from the object stream the entry names | no object, so Null | none | silent | 5.1 only |
| S036 | pdf-syntax/document/objects.rs:304 | not a repair | reference generation differs from the generation the file defines the number at | Object::Null | none | silent | ISO 32000-2 §7.3.10 (undefined object is null); 5.1 only |
| S037 | pdf-syntax/document/repair.rs:54 | recovery | disproved xref entry | object read from the header scan | Document::misfiled_objects | silent | 6.1.4 (cross-reference offsets) |
| S038 | pdf-syntax/document/repair.rs:31 | recovery | whole file cannot be held in memory | empty scan table, repair silently unavailable | Document::scan_refused | silent | 5.1 only |
| S039 | pdf-syntax/document.rs:333 | recovery | catalog unreadable after normal xref read | whole table rebuilt by scanning object headers; original document returned if the rebuild also has no catalog | XrefTable::recovered_by_scan | silent | 6.1.4 (cross-reference table) |
| S040 | pdf-syntax/document/decoding.rs:23 | fallback | stream decode refused or damaged | Option None on refusal; the Damage of a successful partial decode is discarded | none | silent | 6.1.7.2 (filters) |
| S041 | pdf-syntax/document/decoding.rs:380 | tolerance | /DecodeParms array shorter than /Filter array | missing filters get no parameters | none | silent | 6.1.7.2 (filters) |
| S042 | pdf-syntax/document/decoding.rs:331 | tolerance | single /DecodeParms dictionary with several filters | dictionary given to the one filter that takes parameters; Err(SharedParms) if ambiguous | none | silent | 6.1.7.2 (filters) |
| S043 | pdf-syntax/document/decoding.rs:248 | tolerance | DecodeParms value is a reference to a stream | entry left unresolved (a reference) and so treated as absent downstream | none | silent | 6.1.7.2 (filters) |
| S044 | pdf-syntax/document/images.rs:70 | recovery | filter prefix before an image codec decodes with damage | the damaged/truncated prefix data used, Damage dropped | none | silent | 6.2.8.1 (images) |
| S045 | pdf-syntax/document/images.rs:31 | fallback | image stream refused | Option None | none | silent | 6.2.8.1 (images) |
| S046 | pdf-syntax/document/encryption.rs:80 | tolerance | trailer /ID missing, not an array, or first element not a string | empty ID used for key derivation | none | silent | 6.1.3 (no Encrypt) |
| S047 | pdf-syntax/document/encryption.rs:268 | substitution | string cannot be decrypted (bad length, padding) | empty string | none | silent | 6.1.3 (no Encrypt) |
| S048 | pdf-syntax/document/encryption.rs:291 | substitution | stream cannot be decrypted | empty data, Stream.decryption_failed set | Stream::decryption_failed | silent | 6.1.3 (no Encrypt) |
| S049 | pdf-syntax/document/encryption.rs:466 | tolerance | stream's /Filter malformed when choosing its crypt filter | empty chain, so no /Crypt filter honoured, document default method used | none | silent | 6.1.3 (no Encrypt); 6.1.7.2 (Crypt filter) |
| S050 | pdf-syntax/document/object_streams.rs:19 | tolerance | header token pair missing or non-integer before N pairs are read | pairs read so far kept, the rest dropped | ObjectsLost::unnamed | silent | 5.1 only |
| S051 | pdf-syntax/document/object_streams.rs:41 | tolerance | negative or oversize object number or offset in the header | pair silently dropped | none | silent | 5.1 only |
| S052 | pdf-syntax/document/object_streams.rs:67 | tolerance | object stream /N missing or not an integer | N read as 0, so no objects at all | none | silent | 5.1 only |
| S053 | pdf-syntax/document/object_streams.rs:64 | recovery | object stream data refused by its filters | None, every member Null | none | silent | 5.1 only |
| S054 | pdf-syntax/document/object_streams.rs:79 | recovery | object stream data truncated or corrupt | members whose extent lies in the decoded part parsed, others lost | ObjectsLost | silent | 5.1 only |
| S055 | pdf-syntax/document/object_streams.rs:126 | recovery | a member that does not parse | member dropped, object reads as Null | none | silent | 5.1 only |
| S056 | pdf-syntax/document/object_streams.rs:72 | tolerance | object stream /First missing or not a non-negative integer | whole stream unreadable (None) | none | silent | 5.1 only |
| S057 | pdf-syntax/document/object_streams.rs:97 | tolerance | header offsets not ascending | offsets sorted and each member ends at the next larger offset | none | silent | 5.1 only |
| S058 | pdf-syntax/document/object_streams.rs:196 | recovery | xref chain cut or table rebuilt, compressed objects not listed | members enumerated from the object streams' own headers | CompressedRecovery | silent | 6.1.4 (cross-reference table) |
| S059 | pdf-syntax/document/object_streams.rs:210 | recovery | object streams past the decode budget or unreadable | skipped | CompressedRecovery::beyond_the_budget | silent | 5.1 only |
| S060 | pdf-syntax/xref.rs:747 | recovery | startxref chain unusable or empty table | whole file scanned for object headers (rebuild) | XrefTable::recovered_by_scan | silent | 6.1.4 (cross-reference table) |
| S061 | pdf-syntax/xref.rs:980 | recovery | xref unusable | table from header scan; last header per number wins | XrefTable::recovered_by_scan | silent | 6.1.4 (cross-reference table) |
| S062 | pdf-syntax/xref.rs:2136 | recovery | same object number written several times in the file | last header wins, objects that fail to parse are skipped | XrefTable::recovered_by_scan | silent | 6.1.4 (unreferenced objects exempt) |
| S063 | pdf-syntax/xref.rs:1002 | substitution | trailer has no /Root after rebuild | trailer found by scan, else xref-stream dictionary, else the first /Type /Catalog object invented as /Root | XrefTable::recovered_by_scan | silent | 6.1.3 (trailer); 6.1.4 |
| S064 | pdf-syntax/xref.rs:1029 | substitution | no trailer or /Root found at all | first object with /Type /Catalog becomes /Root | XrefTable::recovered_by_scan | silent | 6.1.3 (trailer); 6.1.4 |
| S065 | pdf-syntax/xref.rs:1091 | recovery | /Prev chain loops back on itself | chain ends, named_and_unread reset to false so the cut is NOT declared | none | silent | 6.1.4 (cross-reference table) |
| S066 | pdf-syntax/xref.rs:1142 | tolerance | /Prev present but not a non-negative integer | chain stops | XrefTable::chain_cut | silent | 6.1.4 (cross-reference table) |
| S067 | pdf-syntax/xref.rs:1096 | recovery | /Prev or startxref points past end of file or at an unreadable section | chain stops | XrefTable::chain_cut | silent | 6.1.4 (cross-reference table) |
| S068 | pdf-syntax/xref.rs:1124 | tolerance | /XRefStm not an integer or its section unreadable | hybrid stream ignored | none | silent | 6.1.4 (cross-reference table) |
| S069 | pdf-syntax/xref.rs:1190 | tolerance | /Size missing or not an integer when chain is cut | size 0, nothing declared unread | XrefTable::chain_cut | silent | 6.1.4 (cross-reference table) |
| S070 | pdf-syntax/xref.rs:1279 | recovery | object at a section offset not parseable as an xref stream | Ok(None), chain stops | XrefTable::chain_cut | silent | 6.1.4 (cross-reference table) |
| S071 | pdf-syntax/xref.rs:1310 | tolerance | subsection entry count negative or beyond u32 | count 0, subsection skipped | none | silent | 6.1.4 (cross-reference table) |
| S072 | pdf-syntax/xref.rs:1309 | tolerance | subsection first number negative or beyond u32 | whole table read as ending (None from `?`) then the section is dropped | none | silent | 6.1.4 (cross-reference table) |
| S073 | pdf-syntax/xref.rs:1336 | tolerance | entry generation field | generation read and discarded (never checked) | none | silent | 6.1.4 (cross-reference table) |
| S074 | pdf-syntax/xref.rs:1319 | recovery | entry line malformed (missing offset, generation or n/f) | subsection read so far kept, remaining entries declared unread | XrefTable::declared_and_unread | silent | 6.1.4 (cross-reference table) |
| S075 | pdf-syntax/xref.rs:1376 | tolerance | entry type other than n/f or offset beyond usize | entry declared unread | XrefTable::declared_and_unread | silent | 6.1.4 (cross-reference table) |
| S076 | pdf-syntax/xref.rs:1397 | tolerance | trailer keyword followed by a non-dictionary | empty trailer used | none | silent | 6.1.3 (trailer) |
| S077 | pdf-syntax/xref.rs:1489 | tolerance | subsection whose first number is wrong (entries one off) | whole subsection renumbered when two or more in-use entries agree on a displacement | none | silent | 6.1.4 (cross-reference table) |
| S078 | pdf-syntax/xref.rs:1548 | recovery | classic table ended without a `trailer` keyword | trailer searched for by scanning forward | none | silent | 6.1.3 (trailer); 6.1.4 |
| S079 | pdf-syntax/xref.rs:1580 | tolerance | /W entry not a non-negative integer | width 0 | none | silent | 6.1.4 (cross-reference stream) |
| S080 | pdf-syntax/xref.rs:1585 | tolerance | /W not an array or fewer than three entries | section unreadable (None) | XrefTable::chain_cut | silent | 6.1.4 (cross-reference stream) |
| S081 | pdf-syntax/xref.rs:1596 | tolerance | /Size missing or not an integer | 0 | none | silent | 6.1.4 (cross-reference stream) |
| S082 | pdf-syntax/xref.rs:1605 | tolerance | /Index contains non-integers or has odd length | non-integers dropped (pairs shift) and a trailing odd element ignored | none | silent | 6.1.4 (cross-reference stream) |
| S083 | pdf-syntax/xref.rs:1622 | recovery | xref stream data shorter than /Index declares | entries read up to the data's end | XrefTable::entries_lost | silent | 6.1.4 (cross-reference stream) |
| S084 | pdf-syntax/xref.rs:1875 | not a repair | xref stream record type field absent (W[0]=0) | type defaults to 1; if offset field also absent the record is dropped | none | silent | ISO 32000-2 §7.5.8.3 Table 17 (default type 1); 6.1.4 |
| S085 | pdf-syntax/xref.rs:1883 | not a repair | xref stream entry of unknown type, or type 2 with out-of-range fields | read as a free entry (Deleted) | none | silent | ISO 32000-2 §7.5.8.3 (unknown entry type is a reference to null; type 2 overflow is a tolerance); 6.1.4 |
| S086 | pdf-syntax/xref.rs:1877 | tolerance | type 1 offset beyond usize | read as free | none | silent | 6.1.4 (cross-reference stream) |
| S087 | pdf-syntax/xref.rs:1933 | recovery | xref stream data damaged (truncated flate etc.) | partial decoded output used, Damage dropped (decode_with_parms) | none | silent | 6.1.4 (cross-reference stream); 6.1.7.2 |
| S088 | pdf-syntax/xref.rs:1921 | tolerance | malformed /Filter or /DecodeParms chain | section unreadable | none | silent | 6.1.4 (cross-reference stream); 6.1.7.2 |
| S089 | pdf-syntax/xref.rs:1973 | recovery | startxref not in the last 2048 bytes | whole file searched backwards | none | silent | 6.1.4 (cross-reference table) |
| S090 | pdf-syntax/xref.rs:1980 | tolerance | startxref offset not an integer or negative | None, falls to rebuild | XrefTable::recovered_by_scan | silent | 6.1.4 (cross-reference table) |
| S091 | pdf-syntax/xref.rs:2003 | tolerance | trailer dictionary unparseable | None; trailer taken from other sections | none | silent | 6.1.3 (trailer) |
| S092 | pdf-syntax/xref.rs:2022 | recovery | no usable trailer from the chain | last `trailer` keyword in the file used | XrefTable::recovered_by_scan | silent | 6.1.3 (trailer); 6.1.4 |
| S093 | pdf-syntax/xref.rs:2046 | recovery | xref-stream dictionary as trailer | nearest preceding header of /XRef occurrences, newest first | XrefTable::recovered_by_scan | silent | 6.1.3 (trailer); 6.1.4 |
| S094 | pdf-syntax/xref.rs:374 | not a repair | same object number in several sections | newest section wins, older dropped | none | silent | ISO 32000-2 §7.5.6 (newest section wins); 6.1.4 |
| S095 | pdf-syntax/xref.rs:382 | not a repair | in-use entry in an older section than a free entry for it | entry removed (treated as deleted) only if the free cover is newer | none | silent | ISO 32000-2 §7.5.6 (a newer free entry deletes); 6.1.4 |
| S096 | pdf-syntax/xref.rs:747 | recovery | `%PDF-` header not at offset 0 | offsets shifted by the header's position (offset_origin) | XrefTable::offset_origin | caught by raw row file_header | 6.1.2 (header at byte zero) |
| S097 | pdf-syntax/xref.rs:790 | recovery | header beyond the search window | searched for far away | XrefTable::offset_origin | caught by raw row file_header | 6.1.2 (header at byte zero) |
| S098 | pdf-syntax/file.rs:432 | recovery | short or failed read from the file on disk | bytes read so far returned as the window | FileBytes::read_failure | silent | — |
| S099 | pdf-syntax/file.rs:447 | recovery | failed read of the whole file | partial bytes held as the whole file | FileBytes::read_failure | silent | — |
| S100 | pdf-syntax/file.rs:330 | tolerance | requested length past the end | clamped to what exists | none | silent | — |
| S101 | pdf-syntax/file.rs:408 | tolerance | range past the end | clamped to the end | none | silent | — |
| S102 | pdf-syntax/version.rs:103 | tolerance | header version digits not `N.N` or fewer than 3 bytes | None, version unknown | none | caught by raw row file_header | 6.1.2 (header version) |
| S103 | pdf-syntax/version.rs:186 | tolerance | catalog /Version not a name or not `N.N` | ignored | none | silent | 5.1 only |
| S104 | pdf-syntax/tree.rs:162 | tolerance | name/number tree deeper than MAX_DEPTH | search gives up (None) | none | silent | 5.1 only |
| S105 | pdf-syntax/tree.rs:169 | tolerance | /Names or /Nums not an array, or odd trailing element | ignored; pair tail dropped | none | silent | 5.1 only |
| S106 | pdf-syntax/tree.rs:214 | tolerance | /Limits missing, not an array, or shorter than two | node searched regardless | none | silent | 5.1 only |
| S107 | pdf-syntax/tree.rs:193 | tolerance | /Kids entry not a dictionary or visited twice | skipped | none | silent | 5.1 only |
| S108 | pdf-syntax/tree.rs:247 | tolerance | number-tree key not an integer | pair dropped | none | silent | 5.1 only |
| S109 | pdf-syntax/tree.rs:344 | tolerance | /Names array entries non-matching or odd | pair dropped | none | silent | 5.1 only |
| S110 | pdf-syntax/text_string.rs:38 | substitution | UTF-8 text string with invalid sequences | U+FFFD substituted | none | silent | 6.6.3 (information strings against XMP) |
| S111 | pdf-syntax/text_string.rs:153 | substitution | unpaired surrogate or odd trailing byte in UTF-16BE text string | U+FFFD substituted | none | silent | 6.6.3 (information strings against XMP) |
| S112 | pdf-syntax/text_string.rs:204 | substitution | byte with no PDFDocEncoding character | U+FFFD substituted | none | silent | 6.6.3 (information strings against XMP) |
| S113 | pdf-syntax/crypt.rs:400 | tolerance | /V missing or not an integer | 0 | none | silent | 6.1.3 (no Encrypt) |
| S114 | pdf-syntax/crypt.rs:401 | tolerance | /R missing or not an integer | 0 | SyntaxError::UnsupportedEncryption | silent | 6.1.3 (no Encrypt) |
| S115 | pdf-syntax/crypt.rs:402 | tolerance | /O or /U not a string | empty string | none | silent | 6.1.3 (no Encrypt) |
| S116 | pdf-syntax/crypt.rs:404 | tolerance | /P missing or not an integer | 0 (all permissions cleared) | none | silent | 6.1.3 (no Encrypt) |
| S117 | pdf-syntax/crypt.rs:407 | tolerance | /EncryptMetadata present but not a boolean | true | none | silent | 6.1.3 (no Encrypt) |
| S118 | pdf-syntax/crypt.rs:459 | tolerance | /OE or /UE missing or not strings (R5/R6) | empty | none | silent | 6.1.3 (no Encrypt) |
| S119 | pdf-syntax/crypt.rs:1269 | tolerance | /CF entry not a dictionary | skipped | none | silent | 6.1.3 (no Encrypt) |
| S120 | pdf-syntax/crypt.rs:1300 | not a repair | /StmF /StrF absent or not a name | Identity (no encryption) | none | silent | ISO 32000-2 §7.6.5.1 Table 20 (absent StmF/StrF is Identity; wrong type is a tolerance); 6.1.3 |
| S121 | pdf-syntax/crypt.rs:1306 | not a repair | crypt filter named Identity | Identity method | none | silent | ISO 32000-2 §7.6.6 (Identity crypt filter); 6.1.7.2 (Crypt filter only as Identity) |
| S122 | pdf-syntax/filter.rs:453 | recovery | PNG-predicted data with a short final row | the partial final row unfiltered and kept | none | silent | 6.1.7.2 (filters) |
| S123 | pdf-syntax/filter.rs:598 | recovery | TIFF predictor data whose length is not a whole number of rows | partial last row predicted and kept | none | silent | 6.1.7.2 (filters) |
| S124 | pdf-syntax/filter.rs:747 | substitution | /Crypt filter in a filter chain | data passed through undecoded (decryption done at object level) | none | silent | 6.1.7.2 (Crypt filter only as Identity) |
| S125 | pdf-syntax/filter.rs:1026 | tolerance | whitespace before the zlib stream | leading whitespace skipped | none | silent | 6.1.7.2 (filters) |
| S126 | pdf-syntax/filter.rs:1039 | recovery | FlateDecode data with no zlib header (raw deflate) | inflated as raw deflate | Damage::Unframed | reported (font programs only) | 6.1.7.2 (filters) |
| S127 | pdf-syntax/filter.rs:1090 | tolerance | flate data truncated exactly on a block boundary after a flush marker | reported as whole, no damage | none | silent | 6.1.7.2 (filters) |
| S128 | pdf-syntax/filter.rs:1100 | recovery | zlib stream whose only fault is the Adler-32 check value | output kept | Damage::CheckValue | reported (font programs only) | 6.1.7.2 (filters) |
| S129 | pdf-syntax/filter.rs:1106 | recovery | flate stream corrupt or truncated mid-block | output up to the fault kept, empty output is a refusal | Damage::Truncated | reported (font programs only) | 6.1.7.2 (filters) |
| S130 | pdf-syntax/filter.rs:917 | recovery | LZW data ends before the EOD marker or a bad code | output up to there kept | Damage::Truncated | reported (font programs only) | 6.1.7.2 (LZWDecode not allowed) |
| S131 | pdf-syntax/filter.rs:3062 | tolerance | non-hex character in ASCIIHexDecode data | character skipped | Damage::ForeignCharacter | reported | 6.1.7.2 (filters) |
| S132 | pdf-syntax/filter.rs:3081 | not a repair | odd number of hex digits | final digit padded with 0 | none | silent | ISO 32000-2 §7.4.2 (missing final digit is 0); 6.1.7.2 |
| S133 | pdf-syntax/filter.rs:3076 | recovery | missing `>` end marker | data to end of input accepted | Damage::Truncated | reported | 6.1.7.2 (filters) |
| S134 | pdf-syntax/filter.rs:3124 | tolerance | data begins with `<~` | prefix skipped | none | silent | 6.1.7.2 (filters) |
| S135 | pdf-syntax/filter.rs:3190 | recovery | missing `~>` end marker | data to the end accepted | Damage::Truncated | reported | 6.1.7.2 (filters) |
| S136 | pdf-syntax/filter.rs:3149 | tolerance | `~` not followed by `>` at end | stops as truncated | Damage::Truncated | reported | 6.1.7.2 (filters) |
| S137 | pdf-syntax/filter.rs:3322 | recovery | missing EOD (128) or a run cut short | output so far kept | Damage::Truncated | reported (font programs only) | 6.1.7.2 (filters) |
| S138 | pdf-syntax/filter.rs:1413 | tolerance | LZW /EarlyChange malformed | no streaming stage is produced (the whole-buffer decode refuses with Parameter) | none | silent | 6.1.7.2 (LZWDecode not allowed) |
| S139 | pdf-syntax/filter.rs:3011 | not a repair | damaged decode with no output | refused as Corrupt | FilterRefusal::Corrupt | silent | 6.1.7.2 (filters; refusal, no repair) |
| S140 | pdf-syntax/serialize.rs:855 | substitution | rewritten object holds a reference to an object not carried across | replaced by Null | Written::dangling | off-path | — |
| S141 | pdf-syntax/serialize.rs:847 | tolerance | object graph deeper than MAX_REWRITE_DEPTH | Null | none | off-path | — |
| S142 | pdf-syntax/serialize.rs:919 | tolerance | /Filter array entry that does not resolve to a name | entry dropped when the writer re-encodes the chain | none | off-path | — |
| S143 | pdf-syntax/serialize.rs:848 | tolerance | reference chain longer than MAX_REFERENCE_DEPTH or to an unplaced object | Null | none | off-path | — |
| S144 | pdf-syntax/linearize.rs:312 | tolerance | first object's linearization dictionary not well-formed (wrong-typed /L /H /O /E /N /T) | document reported State::Ordinary | none | off-path | 6.1.11 (linearization ignored) |
| S145 | pdf-syntax/linearize.rs:344 | tolerance | /P missing or not an integer | 0 | none | off-path | 6.1.11 (linearization ignored) |
| S146 | pdf-syntax/write.rs:455 | tolerance | trailer /Size missing or not an integer | 0 | none | off-path | — |
| S147 | pdf-syntax/write.rs:668 | tolerance | trailer /ID missing or not an array | the new digest used as both ID halves | none | off-path | — |
| M001 | content/run.rs:244 | tolerance | unknown operator keyword (not in Table 50) | keyword skipped, pending operands cleared, stream goes on | Unsupported::Operator | off-path | 6.2.2 (only defined operators) |
| M002 | content/run.rs:268 | tolerance | more operands than the operator takes (leftover operands in front) | only the last n operands are read, the leftovers dropped | none | off-path | 6.2.2 (operators) |
| M003 | content/run.rs:288 | tolerance | operator given fewer operands than Table 50 states | operator still run with what is there; each reader gives None and the operator does nothing | Unsupported::OperandShortfall | off-path | 6.2.2 (operators) |
| M004 | content/run/steps.rs:58 | tolerance | keyword (operator) written inside a [ ] array | keyword skipped, array kept | Unsupported::Operator | off-path | 6.2.2 (operators) |
| M005 | content/run/steps.rs:69 | tolerance | unbalanced ] with no open [ | array_depth saturates at 0, bracket token pushed as Null operand | none | off-path | 5.1 only |
| M006 | content/run/steps.rs:87 | tolerance | more operands than MAX_OPERANDS or unclosed [ | token dropped, array_depth reset to 0 | Unsupported::LimitReached | off-path | 6.1.13 (limits) |
| M007 | content/reader.rs:2020 | tolerance | array/dict bracket or hex-string-less token as operand | Object::Null placeholder (arrays flattened by design) | none | silent | 6.2.2 (operators) |
| M008 | content/reader.rs:1532 | tolerance | number followed by regular characters (12pt, 5.2.3, 1e5) not a §7.3.3 number | leading number kept, tail dropped (unless tail spells an operator then whole run is a keyword) | none | reported (content streams, runs as written) | 6.2.2 (operators) |
| M009 | content/reader.rs:2064 | tolerance | inline dictionary key that is not a name | token skipped, parse continues | none | silent | 6.1.10 (inline image dictionary) |
| M010 | content/reader.rs:2081 | tolerance | inline dictionary value that is a keyword other than true/false | Object::Null | none | silent | 6.1.10 (inline image dictionary) |
| M011 | content/reader.rs:2127 | tolerance | inline array element that is a keyword other than true/false | Object::Null element | none | silent | 6.1.10 (inline image dictionary) |
| M012 | content/reader.rs:2096 | tolerance | inline dictionary nested deeper than 8 | returned empty (tokens of it left to be read as content?) unsure | none | silent | 6.1.10 (inline image dictionary) |
| M013 | content/reader.rs:2118 | tolerance | inline array deeper than 8 or longer than 65536 | contents consumed and dropped / extra elements dropped | none | silent | 6.1.10 (inline image dictionary) |
| M014 | content/run/graphics.rs:48 | tolerance | Q with empty graphics state stack | ignored | none | off-path | 5.1 only |
| M015 | content/run/graphics.rs:39 | tolerance | q nesting deeper than MAX_STATE_DEPTH | q ignored (then the matching Q pops the wrong state) | Unsupported::LimitReached | off-path | 6.1.13 (q/Q nesting) |
| M016 | content/run/graphics.rs:66 | tolerance | cm operand not a number (name, string, null) | cm ignored | none | off-path | 5.1 only |
| M017 | content/run/graphics.rs:91 | not a repair | negative line width | clipped to 0 | none | off-path | ISO 32000-2 §8.4.1 (negative width clipped); 5.1 only |
| M018 | content.rs:3519 | not a repair | J operand outside 0..2 | clamped into 0..2 | none | off-path | ISO 32000-2 §8.4.1 (clipped to range); 5.1 only |
| M019 | content.rs:3529 | not a repair | j operand outside 0..2 | clamped into 0..2 | none | off-path | ISO 32000-2 §8.4.1 (clipped to range); 5.1 only |
| M020 | content.rs:3544 | not a repair | M operand below 1 | raised to 1 | none | off-path | ISO 32000-2 §8.4.1 (clipped to range); 5.1 only |
| M021 | content/run/operands.rs:216 | tolerance | integer operand written as a real | real with integral value below 1000 accepted as integer; other reals refused | none | off-path | 5.1 only |
| M022 | content/run/operands.rs:230 | tolerance | number operand non-finite or beyond f32 range | treated as no number (operator declined) | none | off-path | 6.1.13 (real bound) |
| M023 | content.rs:3440 | tolerance | d with non-number array element or phase | dash unchanged, operator ignored | Unsupported::Operator | off-path | 5.1 only |
| M024 | content.rs:3430 | tolerance | d with no array operand | dash unchanged | Unsupported::Operator | off-path | 5.1 only |
| M025 | content.rs:3559 | tolerance | dash array empty, all-zero, or has a negative element | solid line, phase 0 | none | off-path | 5.1 only |
| M026 | content.rs:3563 | not a repair | odd-length dash array / negative phase | repeated / rem_euclid | none | off-path | ISO 32000-2 §8.4.3.6 (odd-length dash array); 5.1 only |
| M027 | content/run/graphics.rs:130 | not a repair | ri operand not a name or an unknown intent name | RelativeColorimetric (unknown name) / unchanged (not a name) | none | off-path | ISO 32000-2 §8.6.5.8 (unrecognised intent is RelativeColorimetric); 6.2.6 (rendering intents) |
| M028 | content/path.rs:436 | tolerance | l/c/v/y with no current point (no m before it) | segment dropped | Unsupported::UndefinedCurrentPoint | off-path | 6.2.2 (content streams) |
| M029 | content/run/graphics.rs:314 | tolerance | g/G/rg/RG/k/K operand not a number | operator ignored, colour in force kept | none | off-path | 6.2.4.3 (device colour spaces) |
| M030 | content/colour.rs:400 | tolerance | cs/CS operand not a name | operator ignored, previous space kept | none | off-path | 6.2.4.1 (colour spaces) |
| M031 | content/colour.rs:661 | substitution | cs/CS names a colour space that does not parse (missing resource, bad array) | DeviceGray with its initial colour | Unsupported::Shading | off-path | 6.2.4.1 (colour spaces) |
| M032 | content/colour.rs:581 | tolerance | sc/scn operand that is not a number among numbers | non-number operands filtered out, count then judged on the rest | none | off-path | 6.2.4.1 (colour spaces) |
| M033 | content/colour.rs:528 | tolerance | scn pattern name with non-number tint operands | non-numbers dropped from the tint | none | off-path | 6.2.4.5 (pattern colour spaces) |
| M034 | content/colour.rs:603 | tolerance | sc/scn with no numeric operand | returns, colour in force stays | none | off-path | 6.2.4.1 (colour spaces) |
| M035 | content/colour.rs:609 | substitution | sc/scn component count disagrees with current space but is 1, 3 or 4 | read in DeviceGray/RGB/CMYK instead of the declared space (operands win) | none | off-path | 6.2.4.1 (colour spaces) |
| M036 | content/colour.rs:621 | tolerance | sc/scn component count matches neither the space nor 1/3/4 | operator ignored, colour in force kept | Unsupported::ColourOperands | off-path | 6.2.4.1 (colour spaces) |
| M037 | content/colour.rs:560 | tolerance | sc/scn numbers given in a Pattern space with no name | ignored, colour in force kept | Unsupported::PatternOperands | off-path | 6.2.4.5 (pattern colour spaces) |
| M038 | content/colour.rs:660 | not a repair | page's DefaultGray/RGB/CMYK colour space missing or unparsable | built-in Device space | none | off-path | ISO 32000-2 §8.6.5.6 (absent Default colour space is the device space); 6.2.4.3 |
| M039 | content/path.rs:521 | not a repair | path ends in a lone m | subpath dropped | none | off-path | ISO 32000-2 §8.5.3.2 (lone moveto paints nothing); 5.1 only |
| M040 | content/run/text_ops.rs:138 | tolerance | Tf with size operand missing or not a number | font size 0 | Unsupported::Operator | off-path | 6.2.11.1 (fonts) |
| M041 | content/run/text_ops.rs:131 | tolerance | Tf with first operand not a name | no current font, text then not shown | Unsupported::Operator | off-path | 6.2.11.1 (fonts) |
| M042 | content/run/text_ops.rs:148 | tolerance | Tc/Tw/Tz/TL/Ts operand not a number | operator ignored | Unsupported::Operator | off-path | 6.2.2 (content streams) |
| M043 | content/run/text_ops.rs:181 | tolerance | Tr mode outside 0..7 | operator ignored, previous mode kept | Unsupported::Operator | off-path | 6.2.2 (content streams) |
| M044 | content/run/text_ops.rs:69 | tolerance | BT inside a text object | previous object ended implicitly, new one begun | Unsupported::Operator | off-path | 6.2.2 (content streams) |
| M045 | content/run/text_ops.rs:89 | tolerance | ET with no BT | ignored | Unsupported::Operator | off-path | 6.2.2 (content streams) |
| M046 | content/run/text_ops.rs:240 | tolerance | Td/TD/Tm/T* outside BT | executed anyway | Unsupported::Operator | off-path | 6.2.2 (content streams) |
| M047 | content/run/text_ops.rs:256 | tolerance | Td/TD/Tm operand not a number | operator ignored | Unsupported::Operator | off-path | 6.2.2 (content streams) |
| M048 | content/run/text_ops.rs:305 | tolerance | Tj/TJ/'/" outside BT | text shown anyway | Unsupported::Operator | off-path | 6.2.2 (content streams) |
| M049 | content/run/text_ops.rs:310 | tolerance | Tj/'/" operand not a string | nothing shown (the ' still moves a line) | Unsupported::Operator | off-path | 6.2.2 (content streams) |
| M050 | content/run/text_ops.rs:334 | tolerance | TJ array with stray operands outside brackets or no array | elements of last array read, rest ignored | Unsupported::Operator | off-path | 6.2.2 (content streams) |
| M051 | content/run/text_ops.rs:356 | tolerance | TJ element neither string nor number | element skipped | Unsupported::Operator | off-path | 6.2.2 (content streams) |
| M052 | content/run/text_ops.rs:378 | tolerance | " operand aw/ac not numbers | spacing left unchanged | Unsupported::Operator | off-path | 6.2.2 (content streams) |
| M053 | content/run.rs:480 | tolerance | BT without ET at end of stream | text object closed implicitly | Unsupported::Operator | off-path | 6.2.2 (content streams) |
| M054 | content/run.rs:497 | tolerance | W/W* with no painting operator after it before stream end | clip dropped | Unsupported::Operator | off-path | 6.2.2 (content streams) |
| M055 | content/run.rs:512 | tolerance | BDC without EMC at stream end | sections closed implicitly | Unsupported::Operator | off-path | 6.7.3.1 (marked content) |
| M056 | content/run/xobject_ops.rs:35 | tolerance | sh with no name operand | empty name looked up, misses | Unsupported::MissingResource | off-path | 6.2.2 (content streams) |
| M057 | content/run/xobject_ops.rs:111 | tolerance | ID/EI outside an inline image | ignored | Unsupported::Operator | off-path | 6.1.10 (inline image dictionaries) |
| M058 | content/run/xobject_ops.rs:76 | tolerance | inline image dictionary lacks W/H | image not drawn | Unsupported::MalformedInlineImage | off-path | 6.1.10 (inline image dictionaries) |
| M059 | content/run/xobject_ops.rs:96 | recovery | inline image malformed or truncated | not drawn; resume point after scanned data | Unsupported::MalformedInlineImage | off-path | 6.1.10 (inline image dictionaries) |
| M060 | content/xobject.rs:39 | tolerance | Do with operand not a name | ignored | Unsupported::Operator | off-path | 6.2.9.1 (form XObjects) |
| M061 | content/xobject.rs:52 | substitution | Do names an XObject not in /XObject, or /XObject category not a dictionary, or entry not a stream | nothing drawn | Unsupported::MissingResource | off-path | 6.2.9.1 (form XObjects) |
| M062 | content/xobject.rs:90 | tolerance | XObject /Subtype missing or not a name | treated as empty, reported as Do on /label | Unsupported::Operator | off-path | 6.2.9.1 (form XObjects) |
| M063 | content/xobject.rs:151 | tolerance | form /Matrix not an array or fewer than six numbers; non-number elements dropped | identity matrix | none | off-path | 6.2.9.1 (form XObjects) |
| M064 | content/xobject.rs:169 | tolerance | form /BBox absent, not an array, or fewer than four numbers | no clip applied (form drawn unclipped) | none | off-path | 6.2.9.1 (form XObjects) |
| M065 | content/xobject.rs:204 | substitution | form /Resources absent or not a dictionary | page's resources used | none | off-path | 6.2.9.1 (form XObjects) |
| M066 | content/xobject.rs:141 | recovery | form XObject stream undecodable | form skipped | Unsupported::Operator | off-path | 6.2.9.1 (form XObjects) |
| M067 | content/run/marked_ops.rs:141 | tolerance | EMC with no open marked-content section | ignored | none | off-path | 6.7.3.1 (marked content) |
| M068 | content/run/marked_ops.rs:40 | tolerance | BMC/BDC tag operand missing or not a name | empty tag used, section still opened | none | off-path | 6.7.3.1 (marked content) |
| M069 | content/run/marked_ops.rs:77 | substitution | BDC /OC names a property list absent from /Properties (or not a dictionary) | content treated as visible | none | off-path | 6.9 (optional content) |
| M070 | content/run/marked_ops.rs:106 | tolerance | BDC property list operand neither dictionary nor resolvable name, or /MCID not an integer | no MCID, section kept without structure link | none | off-path | 6.7.3.1 (marked content) |
| M071 | content/run/compat_type3_ops.rs:11 | tolerance | EX without BX | depth stays 0, ignored | none | off-path | 6.2.2 (content streams) |
| M072 | content/run.rs:454 | tolerance | BX left open at end of stream | ignored | none | off-path | 6.2.2 (content streams) |
| M073 | content/run/compat_type3_ops.rs:30 | tolerance | d0/d1 outside a Type 3 glyph description (or d0 ignored) | ignored | none | off-path | 6.2.2 (content streams) |
| M074 | image/dispatch.rs:523 | tolerance | image /BitsPerComponent absent or not an integer (non-mask image, no filter-implied depth) | 8 bits | none | off-path | 6.2.8.1 (images) |
| M075 | image/space.rs:171 | tolerance | image /Decode with non-number elements or the wrong number of pairs | non-numbers dropped, missing pairs default, values clamped to permitted range | none | off-path | 6.2.8.1 (images) |
| M076 | image/unpack.rs:253 | recovery | decoded sample data shorter than Width x Height x components x bits | samples the file carries are drawn, the rest left unpainted | Unsupported::Image | off-path | 6.2.8.1 (images) |
| M077 | image/unpack.rs:488 | tolerance | soft/matte mask plane shorter than the image | missing alpha values read as 0 | none | off-path | 6.2.10 (transparency) |
| M078 | image/unpack.rs:886 | substitution | Indexed image sample beyond the palette | black | none | off-path | 6.2.8.1 (images) |
| M079 | image/mask.rs:146 | tolerance | /Mask neither a stream nor an array | mask ignored, image drawn unmasked | Unsupported::Image | off-path | 6.2.8.1 (images) |
| M080 | image/soft_mask.rs:121 | tolerance | /SMask unusable (an image mask, carries its own mask, not DeviceGray, wrong grid) | soft mask ignored, image drawn opaque | Unsupported::Image | off-path | 6.2.10 (transparency) |
| M081 | image/dispatch.rs:1032 | recovery | image stream filter data corrupt | image not drawn (Err Malformed) | Unsupported::Image | off-path | 6.2.8.1 (images) |
| M082 | content/xobject.rs:98 | substitution | image /Alternates entry unusable | primary image used | Unsupported::Image | off-path | 6.2.8.1 (Alternates not allowed) |
| M083 | content/image.rs:384 | tolerance | image's own frame contradicts its stream header (JPX/DCT size vs /Width /Height) | stream's frame drawn | Unsupported::Image | off-path | 6.2.8.1 (images) |
| M084 | content/reader.rs:878 | recovery | page content part decodes to damaged/truncated data | decoded prefix interpreted as the content | ContentIssue::Damaged | silent | 6.2.2 (content streams); 6.1.7.2 (filters) |
| M085 | content/reader.rs:914 | recovery | page content part whose filters cannot be decoded | part skipped, rest of /Contents read | ContentIssue::Undecodable | silent | 6.2.2 (content streams); 6.1.7.2 (filters) |
| M086 | content/reader.rs:573 | tolerance | /Contents array entry not a stream (or reference to missing object) | entry skipped | ContentIssue::NotAStream | silent | 6.2.2 (content streams) |
| M087 | content/reader.rs:1003 | tolerance | single token longer than the ceiling | token dropped | ContentIssue::TokenTooLong | silent | 6.2.2 (content streams) |
| M088 | content/run/nested.rs:41 | recovery | form/pattern/glyph stream whose NestedContent cannot be built (undecodable) | None: stream skipped | caller | off-path | 6.2.9.1 (form XObjects) |
| M089 | content/run/nested.rs:43 | recovery | nested stream damaged (truncated) | decoded prefix interpreted | Unsupported::DamagedContentStream | off-path | 6.2.9.1 (form XObjects) |
| M090 | content/font.rs:740 | substitution | Tf names a font resource the /Font dictionary lacks and the name is one of the fourteen standard names | built-in standard font dictionary standard_font_named | none | off-path | 6.2.11.4.1 (embedded program; standard fonts must be embedded) |
| M091 | content/font.rs:746 | substitution | Tf names a missing /Font resource (or one that is not a dictionary) | no font, text not painted; reader advances nothing | Unsupported::Font | off-path | 6.2.11.1 (fonts) |
| M092 | content/font.rs:687 | substitution | /Font entry resolves to null (reference to undefined object) and the host asked for a null-font substitute | Helvetica built in | Unsupported::Font | off-path | 6.2.11.1 (fonts) |
| M093 | content/font.rs:697 | tolerance | gs /Font entry not [indirect-reference size] | font unchanged | Unsupported::Font | off-path | 6.2.5 (extended graphics state) |
| M094 | content/font.rs:848 | recovery | font CMap / ToUnicode / CharProcs / widths damaged or truncated | what loaded is used | Unsupported::Font | off-path | 6.2.11.4.1 (embedded program) |
| M095 | content/font.rs:909 | substitution | font dictionary without /FontDescriptor | program loaded without descriptor | Unsupported::Font | off-path | 6.2.11.4.1 (embedded program) |
| M096 | content/ext_gstate.rs:251 | tolerance | gs operand not a name | operator ignored | none | off-path | 6.2.5 (extended graphics state) |
| M097 | content/ext_gstate.rs:260 | substitution | gs names an /ExtGState entry that is absent or not a dictionary | graphics state unchanged | Unsupported::MissingResource | off-path | 6.2.5 (extended graphics state) |
| M098 | content/ext_gstate.rs:289 | tolerance | ExtGState /ca or /CA not a number | entry ignored; a number outside 0..1 clamped to it | none | off-path | 6.2.10 (transparency) |
| M099 | content/ext_gstate.rs:298 | tolerance | ExtGState /D not an array of [array phase] | dash unchanged | none | off-path | 6.2.5 (extended graphics state) |
| M100 | content/ext_gstate.rs:321 | tolerance | ExtGState /LC /LJ /ML /LW /SM /OPM of the wrong type (real where integer, etc.) | entry ignored (as_integer/as_number None) | none | off-path | 6.2.5 (extended graphics state) |
| M101 | content/ext_gstate.rs:353 | tolerance | ExtGState /SA /TK /AIS /OP /op not a boolean | entry ignored | none | off-path | 6.2.5 (extended graphics state) |
| M102 | content/ext_gstate.rs:200 | tolerance | halftone /TransferFunction function does not parse | treated as unsaid (no transfer) | none | off-path | 6.2.5 (no TR) |
| M103 | content/ext_gstate.rs:156 | tolerance | /HT neither name, dictionary nor stream | /HT ignored | none | off-path | 6.2.5 (no HT with Default only) |
| M104 | content/ext_gstate.rs:459 | tolerance | /UseBlackPtComp name other than ON/OFF/Default | Default | none | off-path | 6.2.5 (extended graphics state) |
| M105 | content/ext_gstate.rs:569 | substitution | /BM name unknown (blend_mode fallback) or array with no known name | Normal | Unsupported::UnknownBlendMode | off-path | 6.2.10 (transparency; blend mode) |
| M106 | content/ext_gstate.rs:546 | tolerance | /SMask entry unusable | soft mask cleared | Unsupported::SoftMask | off-path | 6.2.10 (transparency; soft mask) |
| M107 | content/ext_gstate.rs:413 | not a repair | /FL flatness | read and discarded | none | off-path | ISO 32000-2 §10.7.2 (flatness may be ignored); 6.2.7 (flatness) |
| M108 | page.rs:788 | recovery | catalog unreadable (document.catalog() error) | no root; page count 0, then scan_for_pages looks for /Type /Page objects | Pages::found_by_scanning | silent | 5.1 only |
| M109 | page.rs:831 | tolerance | root /Count absent, not an integer, <= 0 or > MAX_NODES_VISITED | leaves counted by walking the tree instead | none | silent | 5.1 only |
| M110 | page.rs:900 | recovery | page tree reaches no page (Count 0 or no leaf under a believed Count) | every object that declares /Type /Page is taken as a page, in object-number order | Pages::found_by_scanning | silent | 5.1 only |
| M111 | page.rs:1279 | recovery | page object dictionary damaged (stops part-way) | entries read before the damage used, later ones absent (Table 31 defaults) | Page::damaged_dictionary | silent | 5.1 only |
| M112 | page.rs:1431 | tolerance | /Kids element that is neither reference nor dictionary | skipped | none | silent | 5.1 only |
| M113 | page.rs:1766 | tolerance | page-tree node whose /Kids is absent or not an array and not declared /Type /Pages | treated as a page leaf (no /Type /Page needed) | none | silent | 5.1 only |
| M114 | page.rs:1796 | tolerance | /Count of a kid disagreeing with its leaves | Count not trusted, page indexing exact flag cleared | none | silent | 5.1 only |
| M115 | page.rs:1750 | tolerance | page tree deeper than MAX_TREE_DEPTH, or more than MAX_NODES_VISITED nodes | truncated, remaining pages absent | none | silent | 6.1.13 (indirect object bound) |
| M116 | page.rs:96 | tolerance | page tree cycle (kid is an ancestor) | kid skipped | none | silent | 5.1 only |
| M117 | page.rs:277 | tolerance | /Resources on a page-tree node not a dictionary | parent's resources inherited | none | silent | 5.1 only |
| M118 | page.rs:269 | substitution | /MediaBox not four finite numbers or with no area | parent's MediaBox, else Page::DEFAULT_MEDIA_BOX (substituted_media_box) | MediaBoxSubstitution | silent | 6.1.13 (page boundary size) |
| M119 | page.rs:280 | tolerance | /CropBox not four finite numbers | treated as absent (parent's, else MediaBox) | none | silent | 6.1.13 (page boundary size) |
| M120 | page.rs:281 | tolerance | /Rotate not an integer (a real, a name) | treated as absent (parent's, else 0) | none | silent | 5.1 only |
| M121 | page.rs:1927 | tolerance | /Rotate not a multiple of 90 | rounded down to a multiple of 90 | none | silent | 5.1 only |
| M122 | page.rs:1879 | tolerance | CropBox/BleedBox/TrimBox/ArtBox empty or partly outside MediaBox | clamped to the MediaBox / fall back to crop box; empty becomes a point | none | silent | 6.1.13 (page boundary size) |
| M123 | page.rs:1936 | tolerance | /UserUnit not a number, <= 0, > 1000, or not finite | 1.0 | none | silent | 6.1.13 (page boundary size) |
| M124 | page.rs:1946 | substitution | no /Resources anywhere on the ancestry | empty dictionary | none | silent | 5.1 only |
| M125 | inheritance.rs:285 | tolerance | field /Parent chain forms a cycle | chain cut at the repeat; entries above treated as empty | Resolved::cyclic | off-path | — |
| M126 | type3.rs:307 | tolerance | Type 3 /FirstChar absent or not an integer | 0 | none | off-path | 6.2.11.5 (widths) |
| M127 | type3.rs:331 | tolerance | Type 3 /Widths element not a number | 0 width | none | off-path | 6.2.11.5 (widths) |
| M128 | type3.rs:326 | tolerance | Type 3 /Widths absent or not an array | empty widths, every advance 0 | none | off-path | 6.2.11.5 (widths) |
| M129 | type3.rs:329 | tolerance | /Widths longer than LastChar-FirstChar+1 | tail ignored | none | off-path | 6.2.11.5 (widths) |
| M130 | type3.rs:426 | tolerance | Type 3 character code outside /Widths range | advance 0 | none | off-path | 6.2.11.5 (widths) |
| M131 | type3.rs:343 | substitution | /ToUnicode absent, not a stream, or undecodable | empty CMap | none | off-path | 6.2.11.7 (Unicode character maps) |
| M132 | type3.rs:681 | tolerance | Type 3 /Encoding neither a known base-encoding name nor a dictionary | empty encoding table (no glyph names) | none | off-path | 6.2.11.6 (character encodings) |
| M133 | type3.rs:685 | tolerance | Encoding /BaseEncoding unknown name | no base table | none | off-path | 6.2.11.6 (character encodings) |
| M134 | type3.rs:704 | tolerance | /Differences code not an integer in 0..255, or a name before any code, or other element types | element skipped, code unset (u8::try_from fails) | none | off-path | 6.2.11.6 (character encodings) |
| M135 | type3.rs:612 | recovery | /CharProcs dictionary damaged | entries before the damage used | Type3Font::char_procs_damage | off-path | 6.2.11.2 (font types; Type 3) |
| M136 | type3.rs:242 | tolerance | /FontMatrix not six numbers (non-numbers dropped before counting) | font refused (Type3Error::NoFontMatrix) | error | off-path | 6.2.11.2 (font types; Type 3) |
| M137 | inline_image.rs:171 | recovery | inline image dictionary malformed (token other than a name where a key belongs, unparsable value, limit) | image dropped; resume after the next EI terminator found by search | InlineImageError::Malformed | silent | 6.1.10 (inline image dictionaries) |
| M138 | inline_image.rs:277 | tolerance | inline image dictionary key that is not one of Table 92's keys | key dropped (only the abbreviation table's keys are inserted) | none | silent | 6.1.10 (inline image dictionaries) |
| M139 | inline_image.rs:283 | tolerance | inline image key written twice, abbreviated and in full | first abbreviated or first full one kept, others dropped | none | silent | 6.1.10 (inline image dictionaries) |
| M140 | inline_image.rs:397 | substitution | inline /CS names a space absent from /ColorSpace resources | left as the name (later refused as unsupported space) | Unsupported::Image | silent | 6.1.10 (inline image dictionaries) |
| M141 | inline_image.rs:499 | recovery | inline image /L length (or computed unfiltered length, or filter's own end) does not end at EI | next candidate extent used; finally a search for EI preceded by white space, else an EI without white space | none | silent | 6.1.10 (inline image dictionaries) |
| M142 | inline_image.rs:583 | tolerance | inline image /BPC absent when computing data length | 8 bits (length guess only) | none | silent | 6.1.10 (inline image dictionaries) |
| M143 | inline_image.rs:210 | recovery | inline image with no EI before the end of content | image dropped, rest of content consumed | InlineImageError::NoTerminator | silent | 6.1.10 (inline image dictionaries) |
| M144 | output_intent.rs:177 | substitution | /OutputIntents entry not a dictionary, no /DestOutputProfile stream, or ICC profile that does not parse | entry skipped; next usable intent used, else none | none | off-path | 6.2.3 (output intent) |
| M145 | output_intent.rs:115 | tolerance | catalog unreadable | no output intent | none | off-path | 6.2.3 (output intent) |
| M146 | outline.rs:114 | tolerance | catalog unreadable or /Outlines not a dictionary | empty outline | none | silent | 5.1 only |
| M147 | outline.rs:235 | tolerance | outline /First or /Next not an indirect reference (direct dictionary) or points to a non-dictionary | sibling chain stops there | none | silent | 5.1 only |
| M148 | outline.rs:237 | tolerance | outline sibling chain cycles back or exceeds MAX_ITEMS / MAX_DEPTH | chain cut | none | silent | 5.1 only |
| M149 | outline.rs:268 | tolerance | outline item /Title not a string | empty title | none | silent | 5.1 only |
| M150 | outline.rs:265 | tolerance | outline item /F not an integer | 0 (plain) | none | silent | 5.1 only |
| M151 | outline.rs:273 | tolerance | outline item /Count not an integer | closed (open=false); stated count otherwise not checked against visible items | none | silent | 5.1 only |
| M152 | outline.rs:296 | tolerance | outline item /Dest unreadable and /A not a GoTo with a readable /D | no destination | none | silent | 6.5.1 (actions) |
| M153 | outline.rs:316 | tolerance | outline item /C not an array or elements not numbers | black / 0 per component; values clamped to 0..1 | none | silent | 5.1 only |
| M154 | retrieval.rs:196 | tolerance | /QuadPoints elements not numbers, or count not a multiple of 8 | non-numbers dropped; empty result when count not multiple of 8 | none | off-path | 6.3.2 (annotation dictionaries) |
| M155 | retrieval.rs:344 | tolerance | /Annots not an array, element not a dictionary, or a reference listed twice | skipped / de-duplicated | none | off-path | 6.3.2 (annotation dictionaries) |
| M156 | annotation.rs:2012 | tolerance | appearance /Matrix not exactly six finite numbers | identity | Decision::Draw | off-path | 6.3.3 (annotation appearances) |
| M157 | annotation.rs:2053 | tolerance | annotation /Rect or appearance /BBox not exactly four finite numbers | treated as absent; placed by the other box, or nothing drawn | shape_faults | off-path | 6.3.3 (annotation appearances) |
| M158 | annotation.rs:1309 | substitution | appearance stream has no /BBox | box of /Rect's size at origin (§12.7.4.3 default) | missing_bbox | off-path | 6.3.3 (annotation appearances) |
| M159 | annotation.rs:2042 | not a repair | /Rect or /BBox with corners in swapped order | normalised | none | off-path | ISO 32000-2 §7.9.5 (rectangle corners in any order); 6.3.3 |
| M160 | annotation.rs:345 | tolerance | annotation /H not a name or an unknown name | Invert | none | off-path | 6.3.2 (annotation dictionaries) |
| M161 | annotation.rs:1590 | tolerance | annotation /CA or /ca outside 0..1 | clamped | none | off-path | 6.3.2 (annotation dictionaries) |
| M162 | annotation.rs:1672 | not a repair | /AP lacks the /R or /D entry for the state being shown | normal (/N) appearance | none | off-path | ISO 32000-2 §12.5.5 (N used when R or D absent); 6.3.3 |
| M163 | annotation.rs:1683 | tolerance | /AP /N neither stream nor dictionary | no stored appearance (appearance synthesised per §12.7.4.3 where the subtype has one, else nothing) | none | off-path | 6.3.3 (annotation appearances) |
| M164 | annotation.rs:1696 | tolerance | /AP /N dictionary but no /AS and no field value | Decision::Unsupported(no_state) | Decision::Unsupported | off-path | 6.3.3 (annotation appearances) |
| M165 | annotation.rs:1707 | tolerance | /AS names a state the /N dictionary lacks | nothing drawn | none | off-path | 6.3.3 (annotation appearances) |
| M166 | annotation.rs:1214 | tolerance | annotation /Subtype not one of the standard subtypes | no appearance drawn | none | off-path | 6.3.1 (annotation types) |
| M167 | annotation.rs:2090 | tolerance | annotation numeric array (e.g. /C, /QuadPoints, /InkList) with non-number elements | non-numbers dropped | none | off-path | 6.3.2 (annotation dictionaries) |
| M168 | structure/tables.rs:40 | tolerance | table cell /RowSpan or /ColSpan not an integer, zero, or negative | 1 | none | silent | 6.7.3.4 (structure types) |
| M169 | structure/roles.rs:127 | tolerance | /RoleMap value neither name nor array (or array whose first element is not a name) | mapping chain stops there, element keeps the last name | none | silent | 6.7.3.4 (structure types; role map) |
| M170 | structure/roles.rs:129 | tolerance | /RoleMap chain cycles or exceeds MAX_DEPTH | chain cut where it returns | RoleMapEnd::circular | silent | 6.7.3.4 (structure types; role map) |
| M171 | structure.rs:337 | tolerance | /K array longer than MAX_CHILDREN or with entries of the wrong type | excess dropped; non-dictionary/unknown entries skipped | none | silent | 6.7.3.3 (structure hierarchy) |
| M172 | structure.rs:408 | tolerance | /K entry with a /Type other than MCR/OBJR | read as a structure element | none | silent | 6.7.3.3 (structure hierarchy) |
| M173 | structure.rs:405 | tolerance | OBJR without a reference /Obj | child dropped | none | silent | 6.7.3.3 (structure hierarchy) |
| M174 | structure.rs:96 | tolerance | page /StructParents not an integer, or /StructTreeRoot /ParentTree not a dictionary | no structure association for the page's marked content | none | silent | 6.7.3.3 (structure hierarchy) |
| M175 | structure/owners.rs:132 | tolerance | /Annots or parent-tree entries that are not references/dictionaries | skipped | none | silent | 6.7.3.3 (structure hierarchy) |
| M176 | structure/attributes.rs:43 | tolerance | /A class name not a name or its /ClassMap entry not a dictionary | class attributes ignored | none | silent | 6.7.3.4 (structure types) |
| M177 | structure/attributes.rs:300 | tolerance | form field checked state unreadable | Checked::Off | none | silent | 5.1 only |
| M178 | structure/inline.rs:132 | tolerance | numeric layout attribute (e.g. /LineHeight, /BaselineShift) not a number | 0.0 / default | none | silent | 5.1 only |
| M179 | structure.rs:1490 | substitution | table header cell with no /Scope | HeaderScope::assumed(row, column) | none | silent | 6.7.3.4 (structure types) |
| M180 | xmp.rs:322 | tolerance | /Metadata entry present but not a stream | no XMP (None) | none | silent | 6.6.2.1 (metadata streams) |
| M181 | xmp.rs:324 | tolerance | /Type /Metadata and /Subtype /XML absent or wrong on the metadata stream | stream still read as XMP | none | silent | 6.6.2.1 (metadata streams) |
| M182 | xmp.rs:2380 | tolerance | XMP element/attribute prefix not declared by any xmlns in scope | empty namespace used; property kept under it | Xmp::undeclared_prefixes | silent | 6.6.2.2 (namespaces and prefixes) |
| M183 | xmp.rs:3088 | tolerance | XMP element carrying character data where §7.2 allows none | data ignored, element kept | Xmp::stray_character_data | silent | 6.6.2.1 (metadata streams) |
| M184 | xmp.rs:3184 | tolerance | XMP packet with more than MAX_PROPERTIES properties or fields | excess properties/fields dropped | none | silent | 6.6.2.1 (metadata streams) |
| M185 | jpeg2000.rs:310 | substitution | JPX CIELab colr box with no (or short) illuminant field | D50 | none | silent | 6.2.8.3 (JPEG2000) |
| M186 | jpeg2000.rs:1272 | recovery | JPX codestream whose tile-part length (Psot) is zero or unreadable, or SOD with no SOT | marker walk stops; headers read so far are kept | none | silent | 6.2.8.3 (JPEG2000) |
| M187 | jpeg2000.rs:1342 | recovery | JPX data with no box structure (a bare codestream) | read for SIZ alone | none | silent | 6.2.8.3 (JPEG2000) |
| M188 | content/pattern/tiling.rs:119 | tolerance | tiling pattern /TilingType not 1, 2 or 3 (or not an integer) | no spacing adjustment (None) | none | off-path | 6.2.4.5 (pattern colour spaces) |
| M189 | content/pattern/tiling.rs:148 | tolerance | tiling pattern /XStep or /YStep absent, zero or not a number | pattern not painted | Unsupported::Shading | off-path | 6.2.4.5 (pattern colour spaces) |
| M190 | content/pattern/tiling.rs:170 | tolerance | tiling pattern /BBox not four finite numbers (non-numbers dropped before counting) | cell_box None: cell not clipped to a box; swapped corners normalised | none | off-path | 6.2.4.5 (pattern colour spaces) |
| M191 | content/pattern/tiling.rs:188 | substitution | tiling pattern /Resources absent or not a dictionary | empty resource dictionary (deliberately not the invoking stream's) | MissingResource | off-path | 6.2.4.5 (pattern colour spaces) |
| M192 | content/pattern/tiling.rs:197 | tolerance | tiling pattern /PaintType not 1 or 2 | treated as coloured (PaintType 1) | none | off-path | 6.2.4.5 (pattern colour spaces) |
| M193 | content/pattern/tiling.rs:212 | substitution | uncoloured pattern selected from a bare /Pattern space with 3 or 4 tint operands | DeviceRGB/DeviceCMYK chosen from the operand count, else DeviceGray | none | off-path | 6.2.4.5 (pattern colour spaces) |
| M194 | content/pattern.rs:420 | substitution | pattern /PatternType not 1 or 2 | pattern not painted | Unsupported::Shading | off-path | 6.2.4.5 (pattern colour spaces) |
| M195 | content/pattern.rs:441 | tolerance | shading pattern with no /Shading | Null shading, not painted | Unsupported::Shading | off-path | 6.2.4.5 (pattern colour spaces) |
| M196 | content/pattern.rs:397 | substitution | scn names a pattern not in /Pattern or not a dictionary/stream | no pattern (nothing painted) | Unsupported::MissingResource | off-path | 6.2.4.5 (pattern colour spaces) |
| M197 | content/transparency.rs:470 | tolerance | /Group dictionary with /S other than /Transparency, or /S absent or not a name | no transparency group | none | off-path | 6.2.10 (transparency) |
| M198 | content/transparency.rs:475 | tolerance | Group /I or /K not a boolean | false | none | off-path | 6.2.10 (transparency) |
| M199 | content/transparency/soft_mask.rs:316 | substitution | soft mask group form /Resources absent or not a dictionary | page resources | none | off-path | 6.2.10 (transparency) |
| M200 | content/marked.rs:583 | tolerance | /StructParents negative | treated as absent | none | off-path | 6.7.3.3 (structure hierarchy) |
| M201 | content/marked.rs:507 | tolerance | BDC /MCID not an integer | no MCID | none | off-path | 6.7.3.3 (structure hierarchy) |
| M202 | image/dct.rs:60 | tolerance | /DecodeParms /ColorTransform not an integer | treated as absent (transform decided from the Adobe marker / component count) | none | off-path | 6.2.8.1 (images) |
| M203 | image/dct.rs:592 | recovery | JPEG frame header height 0 with a DNL marker (or DNL contradicting it) | frame height patched from the DNL value before decoding | none | off-path | 6.2.8.1 (images) |
| M204 | image/dct.rs:962 | recovery | JPEG data the restart-interval banded decoder cannot read | alternative 'cut' banded decoder, else whole-frame decode | none | off-path | 6.2.8.1 (images) |
| M205 | image/jpx.rs:97 | tolerance | image /SMaskInData not an integer | 0 (no opacity channel used) | none | off-path | 6.2.8.3 (JPEG2000) |
| M206 | image/jpx.rs:366 | tolerance | JPX image /Decode with non-number elements | non-numbers dropped | none | off-path | 6.2.8.3 (JPEG2000) |
| M207 | image/ccitt.rs:69 | recovery | CCITT scan lines damaged (within /DamagedRowsBeforeError) or data stopping early | damaged row drawn as the row above (or white); stop short and rest unpainted | Parts::shortfall | off-path | 6.2.8.1 (images) |
| M208 | image/jbig2.rs:80 | not a repair | /JBIG2Globals null or undefined object | decoded with no global segments | none | off-path | ISO 32000-2 §7.3.9 (null entry equals absent); 6.2.8.1 |
| M209 | image/convert.rs:443 | tolerance | image with no /Decode array or fewer than 6 entries for a 3-component space | default decode used | none | off-path | 6.2.8.1 (images) |
| M210 | content/marked.rs:476 | tolerance | BDC property list /ActualText, /Alt, /E not a string (or Alt/E empty), /Lang not a string | treated as absent; the structure element's own entries read instead | none | off-path | 6.7.7 (replacement text); 6.7.5 (alternate descriptions) |
| M211 | content/path.rs:202 | tolerance | h (closepath) with no open subpath | ignored (current point set to subpath start) | none | off-path | 5.1 only |
| M212 | content/path.rs:241 | not a repair | path painting or n operator with no path under construction | no mark, clip pending dropped | none | off-path | ISO 32000-2 §8.5.3 (no error stated); 5.1 only |
| M213 | content/run/graphics.rs:206 | tolerance | re operand not four numbers (width/height negative allowed) | ignored when operands are not numbers; negative extents drawn as given | none | off-path | 5.1 only |
| F001 | pdf-font/loading/opening.rs:65 | substitution | simple font names no usable descriptor/embedded program (FontDescriptor missing, or no FontFile/2/3 or empty stream) | a substitute face chosen by BaseFont/descriptor (Request::derive) and drawn with the file's /Widths | LoadedFont::substituted | reported (is_substituted) | 6.2.11.4.1 (embedded program) |
| F002 | pdf-font/loading/opening.rs:340 | substitution | composite font whose descendant embeds no usable program | composite_substitute face; glyph reached through ToUnicode/collection (substitute face by character) | LoadedFont::substituted | reported (is_substituted) | 6.2.11.4.1 (embedded program) |
| F003 | pdf-font/loading.rs:1239 | substitution | embedded font program unreadable (Malformed refusal, e.g. TrueType/CFF/Type1 bytes damaged) | the font is reloaded with its FontFile* keys removed (without_program) so a substitute face is drawn | LoadedFont::substituted | reported (is_substituted, FontError::Malformed) | 6.2.11.4.1 (embedded program) |
| F004 | pdf-font/loading.rs:1268 | substitution | descriptor/descendant carrying FontFile keys | descriptor copy with FontFile, FontFile2, FontFile3 removed | none | silent | 6.2.11.4.1 (embedded program) |
| F005 | pdf-font/loading.rs:493 | recovery | CID-keyed CFF whose Font DICTs/FDArray (or Private DICT) cannot be read | Font DICTs replaced by ones with an empty Private DICT; glyphs under them drawn against it | LoadedFont::font_dicts | silent | 6.2.11.2 (font programs conform) |
| F006 | pdf-font/cff.rs:492 | recovery | CFF FDArray offset inside FDSelect / Font DICT states no Private DICT | rewritten CFF with fresh FDArray, empty Private for unreadable Font DICTs, Top DICT offsets re-encoded | cff::FontDictRepair | silent | 6.2.11.2 (font programs conform) |
| F007 | pdf-font/cff.rs:521 | recovery | Top DICT states no FDArray offset | offset 0 / empty Font DICT list (fd_array_at.unwrap_or(0), index_items(..).unwrap_or_default()) | none | silent | 6.2.11.2 (font programs conform) |
| F008 | pdf-font/cff.rs:2489 | tolerance | CFF Header/Name/Top/String INDEX not readable | empty global subroutine list | none | off-path | 6.2.11.2 (font programs conform) |
| F009 | pdf-font/cff.rs:2512 | tolerance | Private DICT / Subrs INDEX unreadable | empty local subroutine list | none | off-path | 6.2.11.2 (font programs conform) |
| F010 | pdf-font/cff.rs:2455 | tolerance | Private DICT entries unreadable or nominalWidthX absent/odd | nominalWidthX 0 (filter_map(Result::ok) drops undecodable entries) | none | silent | 6.2.11.5 (widths) |
| F011 | pdf-font/cff.rs:316 | tolerance | charstring for a glyph does not evaluate, or subfont unreadable | that glyph's width is None (skipped); absent width taken as 0 (defaultWidthX) | none | silent | 6.2.11.5 (widths) |
| F012 | pdf-font/cff.rs:138 | tolerance | CFF charset glyph name not UTF-8 | name through String::from_utf8_lossy (replacement chars) | none | silent | 6.2.11.2 (font programs conform) |
| F013 | pdf-font/cff.rs:155 | tolerance | CFF encoding/charset names a SID absent from the string table | code maps to no glyph (flatten drops it) | none | silent | 6.2.11.2 (font programs conform) |
| F014 | pdf-font/cff.rs:1430 | tolerance | CFF Top/Font/Private DICT states an offset twice, misstated, or overlapping | returns Misstated reason (consumed by the writer paths) - not repaired here | Misstated | off-path | 6.2.11.2 (font programs conform) |
| F015 | pdf-font/sfnt.rs:471 | recovery | TrueType/OpenType with loca extent short of maxp numGlyphs+1, wrong indexToLocFormat (e.g. 0x0100), or non-monotone loca offsets | bytes rewritten (loca extended / format field corrected / glyf+loca rebuilt) before skrifa reads them | none | silent | 6.2.11.2 (font programs conform) |
| F016 | pdf-font/sfnt.rs:183 | recovery | head.indexToLocFormat is neither 0 nor 1 | format chosen from loca length and last offset vs glyf length (both must agree), else skrifa's own reading | none | silent | 6.2.11.2 (font programs conform) |
| F017 | pdf-font/sfnt.rs:385 | recovery | loca table directory length shorter than 2/4*(numGlyphs+1) | loca length field in directory raised to the needed extent | none | silent | 6.2.11.2 (font programs conform) |
| F018 | pdf-font/sfnt.rs:519 | recovery | loca offsets out of order / overlapping glyf entries | glyf and loca rebuilt with ordered, non-overlapping data | none | silent | 6.2.11.2 (font programs conform) |
| F019 | pdf-font/program.rs:116 | recovery | /FontFile3 or FontFile2 holds a TrueType Collection (not a font program, Table 124) | single face extracted by descriptor /FontName else first face (collection::extract) | none | silent | 6.2.11.4.1 (embedded program) |
| F020 | pdf-font/collection.rs:58 | tolerance | collection face named by /FontName not found | face 0 used (or_else(collection.get(0))) | none | silent | 6.2.11.4.1 (embedded program) |
| F021 | pdf-font/program.rs:101 | tolerance | /FontFile3 /Subtype disagrees with the bytes (Type1C vs OpenType) | the leading bytes decide; /Subtype ignored | none | silent | 6.2.11.2 (font programs conform) |
| F022 | pdf-font/program.rs:90 | tolerance | /FontFile2 or /FontFile3 stream decodes to zero bytes (or no program) | treated as the descriptor stating no program: next key tried, finally NotEmbedded then substitution | substituted | reported (is_substituted) | 6.2.11.4.1 (embedded program) |
| F023 | pdf-font/program.rs:193 | tolerance | /FontFile stream empty | treated as absent | substituted | reported (is_substituted) | 6.2.11.4.1 (embedded program) |
| F024 | pdf-font/program.rs:288 | recovery | Type 1 /FontFile stream states /Length3 0 (no 512 zeros + cleartomark) | trailer of 512 zeros and cleartomark restored before parsing | none | silent | 6.2.11.2 (font programs conform) |
| F025 | pdf-font/program.rs:441 | tolerance | truncated font program stream (decode ends early) whose length reaches the stated /Length1..3 extent | the decoded prefix is used as whole when it reaches the stated extent; else refused Malformed | FontError::Malformed | reported (FontError::Malformed on refusal) | 6.2.11.2 (font programs conform) |
| F026 | pdf-font/program.rs:159 | not a repair | sfnt whose table directory overruns the data | refused FontError::Malformed 'the font program is truncated' | FontError::Malformed | reported (FontError::Malformed) | 6.2.11.2 (font programs conform; refusal, no repair) |
| F027 | pdf-font/loading.rs:1369 | tolerance | /ToUnicode absent, not a stream, undecodable, or UseCMap nesting deeper than 4 | an empty ToUnicode (no mapping) | none | silent | 6.2.11.7 (Unicode character maps) |
| F028 | pdf-font/tounicode.rs:190 | recovery | /ToUnicode CMap damaged or truncated mid-section; bad bfchar/bfrange operands | mappings read before the damage kept; malformed entries skipped, token stride stepped to resynchronise | ToUnicode::truncated | silent | 6.2.11.7 (Unicode character maps) |
| F029 | pdf-font/tounicode.rs:598 | tolerance | /ToUnicode destination string with unpaired UTF-16 surrogate | U+FFFD replacement character | none | silent | 6.2.11.7 (Unicode character maps) |
| F030 | pdf-font/cmap.rs:389 | recovery | embedded CMap stream malformed or truncated | everything understood before the damage kept; loader refuses only when no codespace ranges or no mappings | CMap::truncated | silent | 6.2.11.3.3 (CMaps) |
| F031 | pdf-font/cmap.rs:577 | tolerance | cidchar/notdefchar entry whose source is not a string or whose target is not a CID/number | entry skipped, stride advances one/two operands | none | silent | 6.2.11.3.3 (CMaps) |
| F032 | pdf-font/cmap.rs:615 | tolerance | cidrange with non-string bounds, bounds of different length, or low>high | range skipped (or realigned at the matching ]) | none | silent | 6.2.11.3.3 (CMaps) |
| F033 | pdf-font/cmap.rs:441 | tolerance | /WMode integer outside 0..255 | WMode 0 | none | silent | 6.2.11.3.3 (CMaps) |
| F034 | pdf-font/cmap.rs:758 | not a repair | string bytes match no codespace range, or string ends mid-code | code length chosen by §9.7.6.2 partial-match rule; at least one byte, length clamped to 1..=4 | none | silent | ISO 32000-2 §9.7.6.2 (partial-match code length); 6.2.11.3.3 (CMaps) |
| F035 | pdf-font/loading/codes.rs:32 | tolerance | a code's length exceeds the bytes left in the string | code length clamped to what is left (1..=rest.len()) | none | silent | 6.2.11.3.3 (CMaps) |
| F036 | pdf-font/composite.rs:142 | tolerance | /WMode in the CMap stream dict absent, wrong type or outside 0..1 | 0 (and clamp(0,1)) | none | silent | 6.2.11.3.3 (CMaps; WMode) |
| F037 | pdf-font/composite.rs:367 | tolerance | /CIDToGIDMap stream longer than MAX_MAP | stream truncated to MAX_MAP bytes | none | silent | 6.2.11.3.2 (CIDFonts; CIDToGIDMap) |
| F038 | pdf-font/loading/glyphs.rs:28 | not a repair | code maps to no CID in the CMap and no notdef mapping | CID 0 (§9.7.6.3, quoted) | none | silent | ISO 32000-2 §9.7.6.3 (unmapped code is CID 0); 6.2.11.8 (.notdef) |
| F039 | pdf-font/loading/glyphs.rs:252 | not a repair | CID has no glyph in the CIDFont (or /CIDToGIDMap stream too short) | notdef CID then CID 0's glyph | none | silent | ISO 32000-2 §9.7.6.3 (CID without glyph is .notdef); 6.2.11.8 (.notdef) |
| F040 | pdf-font/loading/advances.rs:21 | not a repair | code with no entry in /Widths or /W | font's default width (/MissingWidth or /DW) | none | silent | ISO 32000-2 Table 109 and 115 (default width for an absent entry); 6.2.11.5 (widths) |
| F041 | pdf-font/metrics.rs:100 | tolerance | /Widths not an array; items that are not numbers; /FirstChar not an integer; negative code | non-array = no widths; non-number items skipped; FirstChar 0; negative codes dropped; stops at /LastChar if it is an integer | none | silent | 6.2.11.5 (widths) |
| F042 | pdf-font/metrics.rs:737 | tolerance | /W array malformed (non-integer CID, non-number width, wrong shape) | parsing stops at the first malformed entry (break); entries before it kept; non-number list items become gaps | none | silent | 6.2.11.5 (widths) |
| F043 | pdf-font/metrics.rs:1113 | tolerance | /W2 malformed (list not a multiple of 3, non-numbers) | parsing stops at first bad item; partial triples dropped (as_chunks) | none | silent | 6.2.11.5 (widths) |
| F044 | pdf-font/metrics.rs:1053 | fallback | /DW2 missing, not an array, or items not numbers | [880, -1000] | none | silent | 6.2.11.5 (widths) |
| F045 | pdf-font/metrics.rs:832 | fallback | /MissingWidth present but not a number (or no descriptor) | DEFAULT_WIDTH | none | silent | 6.2.11.5 (widths) |
| F046 | pdf-font/loading.rs:275 | fallback | /DW present but not a number | DEFAULT_CID_WIDTH (1000) | none | silent | 6.2.11.5 (widths) |
| F047 | pdf-font/loading/opening.rs:480 | fallback | /DW present but not a number | 1000 | none | silent | 6.2.11.5 (widths) |
| F048 | pdf-font/metrics.rs:824 | fallback | /Flags absent, not an integer, or negative/overflowing | 0 (no flags) | none | silent | 6.2.11.6 (character encodings; symbolic flag) |
| F049 | pdf-font/metrics.rs:999 | fallback | /Ascent or /Descent absent, wrong type, non-finite, or outside a reasonable line height | (1.0, 0.0) em | none | silent | 6.2.11.2 (font programs conform) |
| F050 | pdf-font/glyph_names.rs:112 | tolerance | /Encoding (or /BaseEncoding) names something outside the permitted encoding names (e.g. /NULL) | treated as no encoding: Ok(None), program's own encoding / StandardEncoding used | none | silent | 6.2.11.6 (character encodings) |
| F051 | pdf-font/glyph_names.rs:160 | tolerance | /Differences not an array; entries neither integer nor name; name before any code; code out of range | non-array ignored; foreign items skipped (_ => {}); names before the first integer dropped; out-of-range codes dropped | none | silent | 6.2.11.6 (character encodings) |
| F052 | pdf-font/glyph_names.rs:194 | tolerance | /Differences glyph name with non-UTF-8 bytes | String::from_utf8_lossy (replacement characters) | none | silent | 6.2.11.6 (character encodings) |
| F053 | pdf-font/truetype.rs:138 | not a repair | /Encoding dictionary leaves codes undefined for a non-symbolic TrueType font | StandardEncoding glyph names | none | silent | ISO 32000-2 §9.6.5.4 (undefined entries filled from StandardEncoding); 6.2.11.6 |
| F054 | pdf-font/truetype.rs:255 | tolerance | symbolic TrueType font whose /Encoding is also present | the encoding's names are 'ignored' and only used as a last recovery when no cmap route reaches a glyph | LoadedFont::reader_chosen | silent | 6.2.11.6 (character encodings) |
| F055 | pdf-font/truetype.rs:335 | tolerance | no usable cmap subtable in the TrueType program | code used as the glyph index (code -> GID) | LoadedFont::reader_chosen | silent | 6.2.11.6 (character encodings) |
| F056 | pdf-font/truetype.rs:322 | substitution | code reached through none of the spec's routes (names, (3,1), (1,0), (3,0)) | character from the font's own ToUnicode (stated_character) or a Unicode cmap (as_character) is used | LoadedFont::reader_chosen | silent | 6.2.11.6 (character encodings) |
| F057 | pdf-font/truetype.rs:239 | tolerance | OpenType 'CFF ' table inside a TrueType program does not parse | ignored (.ok()): no charset used for name lookup | none | silent | 6.2.11.2 (font programs conform) |
| F058 | pdf-font/name_keyed.rs:135 | substitution | encoding names a glyph the program lacks | code has no glyph (None, final); empty slot of a named encoding names .notdef | none | silent | 6.2.11.8 (.notdef) |
| F059 | pdf-font/type1.rs:342 | tolerance | Type 1 program's Encoding array built other than by 'dup <int> /name put' | read as mentioning no codes (program's resolved map used) | none | silent | 6.2.11.6 (character encodings) |
| F060 | pdf-font/type1.rs:138 | not a repair | Type 1 program parse errors | returned as Type1Error (error propagation) - listed for completeness | Type1Error | reported (FontError::Malformed) | 6.2.11.2 (font programs conform; refusal, no repair) |
| F061 | pdf-font/loading/drawing.rs:63 | fallback | glyph outline cannot be evaluated (damaged charstring / glyf) | None (glyph drawn blank); refusal cached in the outline cache | outline_refusal | off-path | — |
| F062 | pdf-font/loading/drawing.rs:256 | fallback | hinting instance cannot be built for the program (bad hinting data) | unhinted outline | none | off-path | — |
| F063 | pdf-font/loading/drawing.rs:403 | tolerance | degenerate one-point contour in a glyph path | dropped or kept as stated | none | off-path | — |
| F064 | pdf-font/substitute.rs:208 | substitution | descriptor /FontWeight, /StemV, /ItalicAngle, /FontStretch, /Flags absent, wrong type or unusable; /FontName names no weight | weight 400, width normal, italic from name/flags - request for the installed face | LoadedFont::substituted | reported (is_substituted) | 6.2.11.4.1 (embedded program) |
| F065 | pdf-font/substitute.rs:955 | substitution | no PANOSE, no FixedPitch/Serif flags, no recognisable name | Family::SansSerif | LoadedFont::substituted | reported (is_substituted) | 6.2.11.4.1 (embedded program) |
| F066 | pdf-font/substitute.rs:114 | substitution | /BaseFont absent or odd; subset prefix | name folded to lower-case, subset tag stripped, then matched to a family | none | reported (is_substituted) | 6.2.11.4.1 (embedded program) |
| F067 | pdf-font/loading/opening.rs:520 | substitution | requested style has no installed face on this machine | nearest installed face, else FontError::NoSubstitute | FontError::NoSubstitute | reported (is_substituted) | 6.2.11.4.1 (embedded program) |
| F068 | pdf-font/substituted.rs:460 | tolerance | /FirstChar or /LastChar absent, wrong type, FirstChar>LastChar, LastChar>255 | all codes 0..=255 (and LastChar clamped to 255) | none | silent | 6.2.11.5 (widths) |
| F069 | pdf-font/substituted.rs:38 | substitution | substitute face lacks glyphs for the declared codes | FontError::NoSubstitute (refused) when none; partial coverage silent | FontError::NoSubstitute | silent | 6.2.11.4.1 (embedded program) |
| F070 | pdf-font/loading.rs:1324 | tolerance | /Encoding is a name with non-UTF-8 bytes | None | none | silent | 6.2.11.6 (character encodings) |
| F071 | pdf-font/composite.rs:415 | fallback | descendant /CIDSystemInfo absent or lacks Registry/Ordering strings | None: no collection table; no ToUnicode-by-collection | none | silent | 6.2.11.3.1 (CIDSystemInfo) |
| F072 | pdf-font/loading.rs:257 | substitution | /DescendantFonts absent or not a usable array (composite font without a descendant) | descendant treated as absent: defaults of /W and /DW apply; may become metrics-only | none | silent | 6.2.11.3.1 (composite fonts) |
| F073 | pdf-font/loading/opening.rs:233 | tolerance | /Encoding of a simple non-embedded font unreadable (substitute_encoding_names fails) | .ok(): metrics answer no code; every code takes /MissingWidth | none | silent | 6.2.11.6 (character encodings) |
| F074 | pdf-font/embed.rs:454 | tolerance | embedded font program name not UTF-8 | String::from_utf8_lossy | none | off-path | — |
| F075 | pdf-font/cff_cids_given.rs:145 | tolerance | CID-keyed CFF whose Font DICT's Private DICT unreadable | empty Private (unwrap_or_default) while writing given CIDs | none | off-path | — |
| F076 | pdf-font/cff_stretch.rs:123 | tolerance | CFF FDArray unreadable or Font DICT without Private | offset 0 / empty list (unwrap_or_default); subfont 0 | none | off-path | — |
| F077 | pdf-font/stretch.rs:633 | tolerance | glyf data unreadable | skipped glyph / stretch refused as None | StretchError::Unreadable | off-path | — |
| F078 | pdf-font/post.rs:65 | tolerance | 'post' table missing/unreadable or glyph-name strings invalid | no post names (ok()), invalid strings flatten to absent | none | silent | 6.2.11.6 (character encodings) |
| F079 | pdf-font/predefined.rs:269 | tolerance | predefined CMap name unknown or cache lock poisoned | lock error ignored (cache miss) | none | silent | 6.2.11.3.3 (CMaps) |
| F080 | pdf-font/subset_tag.rs:121 | tolerance | name-table entries unreadable | empty list | none | silent | 6.2.11.2 (font programs conform) |
| F081 | pdf-font/loading/substitution.rs:148 | tolerance | substitute face bytes do not parse | None | none | silent | 6.2.11.4.1 (embedded program) |
| F082 | pdf-font/vertical.rs:125 | tolerance | font program 'vmtx'/'vhea' unreadable | None: font's own vertical metrics not used | none | off-path | 6.2.11.5 (widths) |
| F083 | pdf-font/loading/program.rs:79 | tolerance | program cannot be re-read | None | none | silent | — |
| F084 | pdf-font/loading/opening.rs:443 | tolerance | OpenType 'CFF ' table in Type1C-tagged font does not read | treated as a plain sfnt (ok()?) | none | silent | 6.2.11.2 (font programs conform) |
| F085 | pdf-font/metrics.rs:658 | tolerance | glyph index beyond maxp numGlyphs or hmtx unreadable | None: width from /Widths or default instead | none | silent | 6.2.11.5 (widths) |
| F086 | pdf-font/pairs.rs:155 | tolerance | GPOS scripts/lookup/coverage tables unreadable or odd | dropped (.ok(), flatten): no pair adjustment | none | off-path | — |

### Counts

| crate | sites | not a repair | reported (census column) | reaching pdf-archive silently | reported to it | caught by raw rows | off-path |
|---|---|---|---|---|---|---|---|
| pdf-syntax | 147 | 13 | 47 | 115 | 14 | 10 | 8 |
| pdf-model | 213 | 13 | 78 | 62 | 1 | 0 | 150 |
| pdf-font | 86 | 7 | 22 | 62 | 12 | 0 | 12 |
| total | 446 | 33 | 147 | 239 | 27 | 10 | 170 |

## 6. The channel, and the fix rounds in order

**The channel** (ADR 3477): `pdf_syntax::repair::Repair { class: &'static RepairClass, at:
Location }`, logged on the `Document` beside the object cache; `RepairClass` a static descriptor
(stable identifier, the ISO 32000-2 clause the file broke, one sentence of what was read
instead); one `CLASSES` slice per crate; the existing reports kept as the detail, each also
logging its class. `pdf-archive` maps every class to section 5.1's row (made `Implemented` over the
log) and to the specific row it touches, or to "no row" by name with a reason.

**The fix rounds**, each taking its findings' tests from `expected-failures-current.json`:

1. **`pdf-syntax`, the channel** — the `repair` module and its log; classes for the lexer
   (S001–S013), the parser (B06; the stream extent and the missing `endstream`, which
   `stream-length-matches-the-data` catches today only by accident), objects and object streams
   (B02, T02), the cross-reference reader (B04), the filters (T01: `decoded_stream_data` keeps
   `Damage` as a class), encryption (T06). `pdf-archive` maps them and section 5.1's row becomes
   `Implemented`; B01, B03 (with `end_of_object`), B04 and B06 pass.
2. **`pdf-model`, the content reader and the page tree** — the survey takes the reader's faults
   (B08: either the interpreter's `Unsupported`/`StreamSyntaxFault` per page, or classes logged by
   `ContentReader` as it tokenises — the round decides by the cost of the second walk); `page.rs`'s
   substitutions (B07); the inline image reader. B07 and B08 pass.
3. **`pdf-font`** — the sfnt repairs, a truncated program, the CFF and Type 1 leniencies, the
   CMap, `ToUnicode`, `/W` and `/Differences` readers (B05, T03, T04). B05 passes.
4. **`pdf-model`, the interpreter's own report** — T05: the viewer's `Unsupported` learns the
   tolerances it does not report today (trap 5), each logged as a class too.
5. **The closing gate** — `tools/conformance/tests/repair_classes.rs`: every class in the three
   slices has a mapping in `pdf-archive`, every mapping names a class and a row that exists, and
   the grep census (every `unwrap_or*`, `.ok()`, `.clamp(` on a file value in the three crates sits
   beside a class or a `// not a repair:` comment naming its clause) is zero.
6. **The rest of `pdf-model`** (forms, XFA, rich text, media, view), inventoried as round 2's
   second half, off the validator's path today.

## 7. Ledger

No ledger row changes status: an audit fixes nothing, and the rows the findings touch (§7.3,
§7.5, §7.7.3.3, §8.2, §9.9 and their neighbours) keep their state until the fix rounds change the
code. ISO 19005's own rows are `pdf-archive`'s table, not the ledger.
