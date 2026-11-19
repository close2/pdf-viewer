# 3138 — A Type 0 font's own `/Encoding` is not asked first for its name's bytes

Session 2109. Status: **accepted; nothing built.** Context: ADR 3117 (a name not in UTF-8 read through its
collection's byte encodings), `crates/pdf-font/src/substitute/native_name.rs`. Clauses: ISO 32000-2 §7.3.5, §9.7.5.2
(Table 116), §9.7.6.

## The question

ADR 3117 reads a `/BaseFont` that is not UTF-8 through each byte-encoding `CMap` Table 116 lists for the font's
collection, and keeps a reading only where it names a style and no other reading names a different one. It does not
read the Type 0 font's own `/Encoding` first. Should the producer's `CMap` be tried first, or break a tie?

## The reading

Nothing in the standard ties the two. §9.7.6 makes `/Encoding` the map from the *show strings'* codes to CIDs;
§7.3.5 says a name read as text is UTF-8. A producer that wrote its font's name in its system's code page wrote the
name in that code page whatever its content encoding was, and most composite fonts' `/Encoding` is `Identity-H` or a
UCS-2 or UTF-16 `CMap`, none of which is a byte encoding of a name at all.

Where the named `CMap` is a byte encoding, it is one of the readings already or a narrower form of one (`GB-EUC-H` of
GBK's, `KSC-EUC-H` of UHC's, `83pv-RKSJ-H` and `Ext-RKSJ-H` of Shift-JIS's), which reads a style word the same.
So it could matter only as a tie-break, where two readings name different styles. Japanese is the one collection with
two unrelated encodings, and a style word in either reads as no style in the other: EUC-JP's bytes are A1–FE, where
Shift-JIS reads half-width katakana or level-2 kanji, and the style words' kanji and kana are all level 1 or full-width
kana with Shift-JIS lead bytes 0x81–0x9F, which EUC-JP has no reading for. The two Big Five readings agree on the
base set the style words are in. The tie cannot arise for these words, so a tie-break would decide nothing.

## Decision

Not built. `Request::derive` keeps being handed the descendant (ADR 3117's reason: the window and the archive's
routes read one name alike). `native_name`'s test `a_japanese_style_word_names_a_style_in_its_own_encoding_only`
holds the Japanese half of the argument; a collection gaining a second unrelated byte encoding reopens it.
