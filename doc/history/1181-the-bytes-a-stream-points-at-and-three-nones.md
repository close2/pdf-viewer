# 1181 — The bytes a stream points at, and three *none*s re-read

## What was built (ADR 1199)

- **`crates/pdf-transform/src/archive/external.rs`, new.** §7.3.8.2's Table 5 as one rewrite: the
  external file's bytes where the stream's own were — which the table says a reader ignores while
  `/F` stands — `/Filter` and `/DecodeParms` from the `F`-prefixed pair, `/Length` restated, the
  forbidden keys removed. `ArchivePlan::external_data` carries what the caller resolved, and
  `Conversion::external_data` names what a conversion needs, in §7.11's own two forms, as
  `Report::requested` names tool invocations.
- **`quorra-transform archive --resolve-external-data`**, inside the two-pass loop `A54` already
  built, so one command converts. The rule is ADR 1155's, applied to §7.11.2.1's components rather
  than this platform's: one component, not absolute, beside the document. The §7.11 reading is
  `pdf_model::file_spec::FileSpec`'s, not a second reading written here.
- **Filter keys with no `/F` need nothing fetched**: Table 5 gives them meaning only through
  `/F`, so no reader consults them and the removal alone answers the requirement.
- **Both spellings of the third key are reported.** ISO 19005 spells it `FDecodeParams`, which
  names nothing in ISO 32000; the rule forbids the *presence* of the keys it lists, so Table 5's
  `FDecodeParms` is the one it is about, and `EXTERNAL_DATA_KEYS` is the one list both sides read.

## What was read, and one *none* that had decayed (ADR 1200)

- **`implementation-limits/page-boundary-sizes` is not *none*.** Table 31 makes four of §14.11.2's
  five boxes optional and §14.11.2.1 gives each a default that is another box in the same file, so
  removing an out-of-range optional entry moves no mark: *mechanical* where §14.11.2.1 has already
  made the box its intersection with the media box, a stated loss where it has not. Only the media
  box keeps the original *none*. Not built — a wrong geometry rule changes what a page shows.
- **The two font *none*s hold**, and the clauses differ: section 6.2.11.4.1's NOTE 2 exempts a font
  shown solely in §9.3.6's rendering mode 3, section 6.2.11.8 forbids a `.notdef` reference
  *regardless of text rendering mode*, and the validator holds each to its own.
- **`--font` is a flag this tree describes in seven places and does not accept.**
## Measured

`archive_corpus` under the lock, before and after: **PDF/A-2b** 471 converted, 139 refused;
**PDF/A-4** 207 and 152 — unchanged at every target, and the sweep now prints why. Its new census
counts how each stream that keeps its data outside the file names it, and not one witness names a
plain file: at PDF/A-4 the three state `/FS /URL`, at PDF/A-2b the three state `/FS` as a *string*,
which Table 43 makes no file system at all, leaving an `http:` locator to be read as a §7.11.2
string of several components — refused either way. `tool = "resolve-external"` is what those need.
