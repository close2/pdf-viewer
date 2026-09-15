# The case-folding data, and where these bytes came from

ISO 32000-2 §12.3.5.2 restricts the names inside a portable collection and then compares two of
them with the case taken out:

> In addition to the restriction on naming folders, as just described, it is further required that
> two file names in the same folder do not map to the same string following case normalization.

It does not define that mapping. It names the document that does:

> See "Unicode Standard Annex #21, Case Mappings" for information on case normalization.

That annex defines caseless matching as the comparison of two strings after the operation it calls
toCasefold, and names `CaseFolding.txt` in the Unicode Character Database as the data for it. This
directory is that data.

## What is here

`CaseFolding.txt`, 87 KB, the file as published — 1618 mappings under four statuses, of which the
1585 with status C or F are the *full* case folding this program applies.
`crates/pdf-model/build.rs` turns those into one sorted `static` table and
`pdf_model::case` is what reads it. Nothing is parsed at startup, which is `CLAUDE.md` principle
2's rule for compiled-in data and the reason this is a build script rather than a reader
(ADR 1086).

The two statuses left out are stated rather than assumed: **S** is the simple folding, for
implementations that cannot let a string grow, and taking it would leave U+00DF folding to itself —
the defect the table exists to remove; **T** is the pair of Turkic mappings, which the file's own
usage note says to exclude by default, and a folder name in a PDF carries no language that could
ask for them.

## Where it came from

<https://www.unicode.org/Public/UCD/latest/ucd/CaseFolding.txt>, fetched on 2026-09-15. The file
names its own version on its first line: **CaseFolding-17.0.0.txt**, dated 2025-07-30.

    sha256  ff8d8fefbf123574205085d6714c36149eb946d717a0c585c27f0f4ef58c4183

It is committed rather than fetched at build time, for the reason
`data/standard-fonts/PROVENANCE.md` gives about the submodule and `data/cmaps/PROVENANCE.md`
about the system copy: what a document's names compare equal to must not be a property of which
machine built the binary, or of whether that machine had a network.

## Its terms

Unicode's data files are published under the Unicode licence, which is on the allow-list
`cargo deny check licenses` reads as `Unicode-3.0` and whose one obligation is that its copyright
and permission notice travel with any copy. `/NOTICE` section 5 carries that notice verbatim,
which is the same surface the fonts and the CMaps use — `cargo deny` reads Cargo metadata and
cannot see vendored data.
