# 2318 — The 2005 edition's other structured types are held to their fields, and part 4 finds a stream that is no profile short of its signature

Session 1742, the archive slot of batch one hundred and thirteen. Status: implemented. Context: ADR
2295 (the Media Management types held to their fields; part 2's tag row on a stream with no
profile file signature), ADR 1991 (the September 2005 XMP edition), `doc/history/1730`'s "Found, not
fixed".

## 1. ISO 19005-2 section 6.6.2.3.1: every structured type of the edition, field by field

ADR 2295 read a property using a predefined schema as carrying a value of the type the schema gives
it, and a structure stating a field its type does not define as no value of that type. It held the
three Media Management types to their tables and left the rest. The September 2005 edition was
fetched again from the address ISO 19005-2's bibliography gives, archived, and kept in the round's
scratch only; its chapter 4 was read type by type. Every property of the edition whose value type is
a structure, or an array of one, now names its type in `pdf_archive::table::metadata`'s
`STRUCTURED`, and each type carries its field namespace and the fields its table lists:
`Thumbnail` (`xmp:Thumbnails`), `Job` (`xmpBJ:JobRef`), `Dimensions` (`xmpTPg:MaxPageSize`,
`xmpDM:videoFrameSize`), `Font` (`xmpTPg:Fonts`), `Colorant` (`xmpTPg:Colorants`,
`xmpDM:videoAlphaPremultipleColor`), the Dynamic Media schema's `Time`, `Timecode`, `Marker`,
`Media`, `ProjectLink` and its three stretch types, and the EXIF schema's `Flash`, `OECF/SFR`,
`CFAPattern` and `DeviceSettings`. A field whose own type is one of these — a marker's `startTime`
and `duration`, a medium's, a beat splice's `riseInTimeDuration` — is held to it in turn. An
alternative array's items are items here as a sequence's are, since `xmp:Thumbnails` is one.

**Two readings were needed, and each is a choice written down.**

- **The `Font` type's namespace is printed without the `//` after `http:`.** The January 2004
  edition printed `Dimensions`' the same way, and the September 2005 edition's own list of changes
  names that one a correction to `http://`. So the misprint is the edition's, not the type's: a
  structure in either spelling is a `Font`, and neither is refused over a defect of the text.
  Admitting both refuses nothing either reading admits.
- **A marker's `startTime` is a `Time`.** Later editions make it a frame count written as text;
  this edition's table types it `Time`, a structure, and a simple value there is no value of that
  type. veraPDF 1.30.3 accepts any value there, which is a reading of later editions and decided
  nothing.

veraPDF's own lists of these fields (its `XMPConstants`) agree with the tables read here, the
`Font` namespace spelled `http://`, and `Colorant` without the later editions' `tint`. **One field
was first misread**: `timeScaleStretch`'s table prints `frameOverlappingPercentage` across two
lines of its field column, and the first build took the first line for the name. The validator over
the veraPDF corpus caught it — `6-6-2-3-1-t02-pass-z`, built to conform at 2b, came back failing —
and the table was read again before the field was changed, not the file matched.

**The corpus.** Held to their fields, the pdf.js and `doc/corpora` documents gave no finding for
any of the new types: their `Thumbnail`, `Dimensions`, `Font` and `Flash` structures state the 2005
fields alone. The later editions' `xmpG:tint` that four pdf.js documents carry sits inside
`xmpTPg:SwatchGroups`, a property the 2005 edition does not define, which needs an extension
schema's description and is that row's business, not this one's.

## 2. ISO 19005-4 section 6.2.4.2: a stream that is no profile is short of its signature

Part 4 defers to ISO 32000-2 §8.6.5.5, whose stream "shall contain the ICC profile" (verbatim), and
whose closing sentence selects the edition a profile is held to by the version in its header. ADR
2295 made part 2's row report a stream without `acsp` at bytes 36 to 39 as short of the profile
file signature; part 4's row read no header there and passed the stream. Every ICC text this tree
holds — ICC.1:1998-09, ICC.1:2001-12, ICC.1:2022 and ICC.2:2023 — places the signature at those
bytes, so such a stream holds no profile of any of them, and no header version exists to select a
text that could excuse it. Both parts now report it in the same words, and
`icc_version_tag_shortfall` answers it for the converter as `icc_tag_shortfall` does: the existing
route for a profile this reader does not draw through, the `/Alternate` written where the array was
(ADR 1979), takes it at part 4 as at part 2.

What would reopen either section: an edition of the XMP Specification or of ICC.1 that ISO 19005
names and this tree does not hold.
