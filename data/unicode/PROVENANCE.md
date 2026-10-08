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

## The shaping and ordering data (ADRs 1413 and 1414)

§12.7.4.3 has this program write the content stream that shows a field's value, and §7.9.2.2.1
makes that value Unicode text. Drawing one in a right-to-left, cursive script takes Unicode Standard
Annex #9's order and the Unicode Standard's cursive joining, and five more files of the UCD are
that data. All five were fetched from <https://www.unicode.org/Public/UCD/latest/ucd/> on 2026-09-29 —
`DerivedJoiningType.txt` from its `extracted/` directory — and each names version **18.0.0** on its
first line. They are shipped unchanged.

    sha256  8ccde4ebd070500a68e8bcb9d5514522fb0bebe57b9ebad3ff925b8e1e669f83  ArabicShaping.txt
    sha256  e2408ff2c92b175b0f7bf62c989bbb54c7b077528fe31f8d69b96fa09e7d61ed  DerivedJoiningType.txt
    sha256  0736451de439ae7baf1425136617da495e09ee5afbe6e394374db7009ea08950  UnicodeData.txt
    sha256  cd54810ebf52f0e61a730c8b9cb25975de6c85f6d788a559b416afd548923fd6  BidiMirroring.txt
    sha256  045b24d2c8ab066951bd32fe8c6b4de34647f72b5b1c7df0265f24ab53573e01  BidiCharacterTest.txt

What is used, field by field, by `crates/pdf-font/build.rs`, which compiles each into a `static`
table so that nothing is parsed at launch:

- `DerivedJoiningType.txt`, both fields — every code point whose Joining_Type is not U, the
  transparent class included, which is why this file rather than `ArabicShaping.txt` is the
  source of the types.
- `ArabicShaping.txt`, fields 0 and 3 — the members of the joining groups LAM and ALEF, which the
  obligatory lam-alef ligature is stated over.
- `UnicodeData.txt`, fields 0 and 5, over U+FB50..U+FDFF and U+FE70..U+FEFF only — each
  presentation form's `<isolated>`, `<final>`, `<initial>` or `<medial>` decomposition, which is
  the table from a letter and a position to the code point that draws it.
- `BidiMirroring.txt`, both fields — rule L4's mirror image of a character.
- `BidiCharacterTest.txt` is **not compiled in**: `crates/pdf-font/tests/bidi_character_test.rs`
  reads it at test time and runs every line through `pdf_font::shaping`.

## The script data (ADR 1696)

A rich text run asking for pair kerning is kerned by the `GPOS` script table its characters'
script selects, and two more files of the UCD say what that script is. Both were fetched from
<https://www.unicode.org/Public/UCD/latest/ucd/> on 2026-10-08, each names version **18.0.0** on
its first line, and both are shipped unchanged.

    sha256  0071fd81b6aeae25f6e8bce8efec3066a6476a91b49bdb2f52dc76e817862a6a  Scripts.txt
    sha256  06c4c8eaf7b0bf34abe73b113da1215bd784ac254d4c223600b90267caa4bbbd  PropertyValueAliases.txt

- `Scripts.txt`, both fields — every range's `Script` value but `Common` and `Inherited`, which
  the reader resolves from the characters beside them; compiled by `crates/pdf-font/build.rs`.
- `PropertyValueAliases.txt`, the `sc` rows' first two value fields — each `Script` value's short
  alias, the ISO 15924 code the table is written in.

The bidirectional algorithm's own class data is `unicode-bidi`'s, a Cargo dependency whose licence
`cargo deny` reads; these files are what the same notice in `/NOTICE` section 5 covers.
