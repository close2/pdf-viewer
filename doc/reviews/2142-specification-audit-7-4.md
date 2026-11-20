# Specification audit of ISO 32000-2 §7.4 and §7.3.8's stream entries — round 2142

The project's fourth specification audit, made beside the other lanes in the shape of the owner's
review 2038 ([`2038-specification-audit.md`](2038-specification-audit.md)) and of audits 2128, 2131
and 2136 ([`2136-specification-audit-7-5.md`](2136-specification-audit-7-5.md)). Baseline: main at
`3c3ed860`. Evidence: [`2142-specification-audit-7-4/`](2142-specification-audit-7-4/) —
`findings.json` (2038's schema) and `expected-failures.json` (one failing test per behaviour
finding). The same entries are in `2038-specification-audit/expected-failures-current.json`.

**This is an audit and fixes nothing.** A fix round takes the findings by the groups at the end.

## Method

1. Every `shall` and `should` of §7.4.1 to §7.4.10 (Tables 6 to 14 — this edition numbers the
   filter tables 6 to 14, not 5 to 16) and Table 5's stream entries (`/Length`, `/Filter`,
   `/DecodeParms`, `/F`, `/FFilter`, `/FDecodeParms`, `/DL`) was read in
   `doc/md/ISO_32000-2_sponsored_EC3.md`, lines 1506 to 2052.
2. The reader was traced through `pdf_syntax`'s `filter.rs` (`decode_reported`,
   `decode_with_parms_reported`, `apply_predictor`, `tiff_predictor`, `lzw`/`Lzw::step`,
   `flate`/`inflate`, `ascii_hex`, `ascii85`, `run_length`, the windowed stages and `stage`),
   `document/decoding.rs` (`chain_over`, `decode_parms`, `with_values_resolved`, `filter_chain`),
   `document/sources.rs` (`pumping`, `filtered_extent`), `document/images.rs` (`image_stream`,
   `is_external`), `document/encryption.rs` (`stream_filter`) and `xref.rs`'s own chain; the image
   codecs through `pdf-model/src/image/` (`ccitt.rs`, `jbig2.rs`, `dct.rs`, `jpx.rs`) and
   `pdf-sandbox/src/decode.rs` (a Sonnet census, its line numbers checked where a finding rests on
   them); the writer's re-encoding through `serialize.rs`'s stream recompression.
3. The lens was the three habits of the earlier audits and the brief's focus: a parameter read
   from the wrong place or as the wrong type, a default applied where the table states another, a
   parameter applied to a filter it does not belong to, each filter's end-of-data, and what is
   returned when data stops short (trap 5).
4. Each B finding has a failing `*_spec_audit` test in
   `crates/pdf-syntax/tests/filters_spec_audit.rs`, built from bytes the test writes, with a
   control in the same test that passes on main; each failure was read and is the finding's own
   assertion, not a control's.

What holds is not listed one by one. Among it: each stage of a chain takes the `/DecodeParms`
array entry at its own index, the entry and its indirect values resolved (2038-B54's fix), a
`null` entry read as defaults; `/EarlyChange` read only by `LZWDecode`, with Table 8's default 1;
LZW's 9-to-12-bit codes, clear-table, EOD 257, the code one past the table, and the width change
one code early; RunLength's 128 EOD and a missing EOD reported `Truncated`; ASCII85's
alphabet, `z` only between groups, a group above 2³²−1 and a final group of one character
refused; ASCIIHex's odd final digit; Flate's truncation, corruption and check value told apart;
every expansion held to `max_stream_len` and refused as `TooLarge` rather than clamped, the
windowed route costing a fixed window per stage; the TIFF predictor at every depth Table 8 lists
and none other; an image codec only as the last filter, with its own index's parameters;
`/ColorTransform`'s APP14 precedence and its default by component count; `/JBIG2Globals`
decoded through its own filters; CCITT's `/Columns` checked against `/Width`, `/EndOfBlock`
overriding `/Rows`; `/F` refused as `External` by name; `/DL` read by nothing (it is a hint).

**One premise of the brief did not hold**: 2038-B56 is the Warichu split finding (Annex L); the
shape the brief describes — an indirect value inside `/DecodeParms` read as a default — is
2038-B54, fixed by `Document::decode_parms`. That fix holds for the stream route and the image
route; `xref.rs`'s own chain reads its parameters directly, which §7.5.8.2's direct-only rule for
a cross-reference stream permits.

## Counts

| kind | count |
|---|---|
| B (behaviour) | 7 |
| T (test) | 0 |
| D (documentation) | 3 |

## Findings

**2142-B01 — a `/Filter` element that is not a name is skipped.** Table 5: `/Filter` is "the
name, or an array of zero, one or several names, of filter(s) that shall be applied"; §7.4.1: "A
PDF reader shall invoke the corresponding decoding filter or filters". `Document::filter_chain`
keeps only the elements that resolve to names (`filter_map`), and a `/Filter` that is neither a
name nor an array is an empty chain. So `[/ASCIIHexDecode 7]` decodes as the hex stage alone and
`/Filter 7` hands back the raw bytes, both as whole decodes; and because the chain is
renumbered, every later stage takes the `/DecodeParms` entry of the stage before it. The image
route and the windowed route read the same list. Test:
`a_filter_entry_that_is_not_a_name_is_not_skipped_spec_audit`.

**2142-B02 — ASCII data without its end-of-data marker is reported whole.** §7.4.2's `>` and
§7.4.3's `~>` are each filter's EOD, and §7.3.8.2 makes the filters "self-limiting" and the
constraints "consistent". `Decoded::damage` is documented `None` only "where it reached the
filter's end-of-data", and RunLength, LZW and Flate report `Damage::Truncated` for a missing
marker; `ascii_hex` and `ascii85` (and the windowed `AsciiHex`, whose comment states the choice)
report `414243` and `5sdq,` as whole. A cut ASCII-armoured stream is drawn as if complete, and
an odd final hex digit is padded by a rule the clause states only at an EOD marker. Test:
`ascii_data_without_its_eod_marker_is_reported_short_spec_audit`.

**2142-B03 — a Table 8 entry of the wrong type or range is read as its default.**
`decode_with_parms_reported` reads `/Predictor` with `as_integer`, so `/Predictor 12.0` is read as
absent, default 1, and the PNG rows come back with their tag bytes as the decoded data — the
outcome `apply_predictor`'s own comment calls "silently wrong". The same reading turns a negative
`/Colors`, `/BitsPerComponent` or `/Columns` into its default (`usize::try_from(..).ok()`), an
`/EarlyChange` of 2 into 1, and, in `pdf-model/src/image/ccitt.rs`, a non-boolean
`/EndOfBlock`, `/BlackIs1`, `/EndOfLine` or `/EncodedByteAlign` and a non-integer `/K` into
Table 11's defaults. Test: `a_predictor_that_is_not_an_integer_is_not_read_as_no_predictor_spec_audit`
(the predictor case; the rest share the reading and have no test of their own).

**2142-B04 — the PNG predictor un-predicts at a depth Table 8 does not list.** Table 8:
`/BitsPerComponent` "Valid values are 1, 2, 4, 8, and (PDF 1.5) 16". `tiff_predictor` refuses any
other depth (ADR 2729); `apply_predictor`'s PNG arm takes the depth into its bytes-per-pixel and
row length unchecked, so `/Predictor 12 /BitsPerComponent 3` decodes and is reported whole. Test:
`a_png_predictor_refuses_a_depth_table_8_does_not_list_spec_audit` (control: the TIFF predictor
at the same depth is refused).

**2142-B05 — a predictor is applied after filters that take no parameters.** Table 6 lists
`ASCIIHexDecode`, `ASCII85Decode` and `RunLengthDecode` with "Parameters: no", and §7.4.4.3 and
§7.4.4.4 make `/Predictor` a parameter of "LZWDecode and FlateDecode". `decode_with_parms_reported`
applies a predictor after any stage that has one in its parameters, so RunLength's `1 2 3` comes
out as `1 3 6`. A single `/DecodeParms` dictionary given for a chain (D01) carries a predictor to
every stage this way. Test:
`a_predictor_is_not_applied_after_a_filter_that_takes_no_parameters_spec_audit`.

**2142-B06 — an `/F` naming an undefined object makes the stream external.** Table 5: if `/F` "is
present, the bytes between stream and endstream shall be ignored". §7.3.10: a reference to an
undefined object "shall be treated as a reference to the null object", and §7.3.9: a null value
"shall be equivalent to omitting the entry entirely". `Document::is_external` asks
`stream.dict.get("F").is_some()`, and its comment calls a reference "equally present", so `/F 9 0 R`
with no object 9 refuses a stream whose own bytes are its data. The habit of 2131: a rule keyed
on a name (§7.3.9's null) applies wherever that name is read. Test:
`an_external_file_entry_that_is_null_is_absent_spec_audit`.

**2142-B07 — a Crypt filter naming no defined crypt filter passes its bytes through.** Table 14's
`/Name` "shall correspond to an entry in the CF entry of the encryption dictionary ... or one of
the standard crypt filters" (Table 26: `Identity`). In an encrypted document a name `CF` lacks is
refused (`named_method` answers `None`, `decryption_failed`); in a document with no encryption
dictionary `stream_filter` is never asked and `decode_reported`'s `Crypt` arm passes the bytes
through, so `/Filter /Crypt /DecodeParms << /Name /StdCF >>` hands ciphertext over as data
(trap 5). Test: `a_crypt_filter_naming_no_defined_filter_is_not_a_pass_through_spec_audit`.

**2142-D01** — `decode_parms` (and `xref.rs`'s `parms_for`) gives a single `/DecodeParms`
dictionary to every stage of a multi-filter chain. Table 5 requires an array there ("If there are
multiple filters and any of the filters has parameters set to nondefault values, DecodeParms shall
be an array"), so the input is malformed and the choice is the reader's, but it is made silently
and documented nowhere; with B05 it runs a predictor after every LZW or Flate stage.
**2142-D02** — ledger 7.4.10's note says the Crypt filter's position is "a requirement on the
writer that this reader does not need: a pass-through commutes with everything". The decryption
runs on the stream's raw bytes before the first stage wherever `/Crypt` stands, so
`[/ASCIIHexDecode /Crypt]` decrypts the hex text, not its output; nothing commutes. The note should
say that a misplaced `/Crypt` is decrypted first regardless, and that this is a choice.
**2142-D03** — `jbig2_bilevel`'s comment says a globals stream that will not decode "is reported
rather than skipped", because without it the page "would decode to blank or to nothing"; a
`/JBIG2Globals` present but not a stream (an undefined reference, a dictionary) is skipped without
a word (`_ => None`). The comment should name that case, or the code report it.

## Ledger rows a behaviour finding contradicts

Of the eighteen rows audited (7.3.8, 7.3.8.1, 7.3.8.2, 7.4 and 7.4.1 to 7.4.10 with 7.4.4.1 to
7.4.4.4), 7.4.9 is `departed` (CIEJab) and seventeen are `implemented`. The findings contradict
**7.4.1** (B01, B05), **7.4.2** and **7.4.3** (B02), **7.3.8.2** (B02), **7.4.4.3** (B03, B04,
B05), **7.4.4.4** (B04), **7.4.6** (B03's CCITT half), **7.3.8** (B06) and **7.4.10** (B07, and
D02's note), and through them the aggregate 7.4. They are not moved here; the fix round decides.

## Suggested fix groups

| group | findings | files |
|---|---|---|
| A — the chain as stated | B01, D01 | `pdf-syntax/src/document/decoding.rs` (`filter_chain`, `decode_parms`), `document/sources.rs`, `document/images.rs` (`codec_position`), `xref.rs` (`parms_for`) |
| B — the ASCII filters' end-of-data | B02 | `pdf-syntax/src/filter.rs` (`ascii_hex`, `ascii85`, `AsciiHex::turn`, `Ascii85::turn`), ledger 7.4.2, 7.4.3 |
| C — Table 8's parameters | B03, B04, B05 | `pdf-syntax/src/filter.rs` (`decode_with_parms_reported`, `apply_predictor`, `early_change`); `pdf-model/src/image/ccitt.rs` for B03's CCITT half |
| D — the stream's own entries | B06, B07, D02 | `pdf-syntax/src/document/images.rs` (`is_external`), `filter.rs` (`decode_reported`'s `Crypt` arm) or `document/decoding.rs`, ledger 7.4.10 |
| E — JBIG2 globals | D03 | `pdf-model/src/image/jbig2.rs` |

A, B and C touch different functions and can run as one round each; C is the largest and should
decide once what a present-but-invalid parameter is (refused, or reported) and apply it to every
Table 8 and Table 11 entry. The most consequential are B01 and B03, which hand back bytes the file
did not state as a whole decode; B07 hands over ciphertext as data; B02 hides a cut stream.
