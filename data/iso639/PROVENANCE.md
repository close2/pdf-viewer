# ISO 639-2's code list, and where this copy came from

`ISO-639-2_utf-8.txt` is the ISO 639-2 Registration Authority's list of language codes, one language
per line: the bibliographic code, the terminology code where it differs, the ISO 639-1 two-letter
code where there is one, and the English and French names, separated by `|`. A BCP 47 language tag
writes a language by its two-letter code where it has one, and OpenType's language system registry
(`data/opentype/language-tags.txt`) by its three-letter code; `crates/pdf-font/build.rs` reads this
list for the one column between them, the three-letter code of each two-letter one — the terminology
code where the two differ, which is ISO 639-3's (ADR 1708).

## Where it came from

<https://www.loc.gov/standards/iso639-2/ISO-639-2_utf-8.txt>, the Library of Congress's file as the
Registration Authority publishes it, fetched on 2026-10-08 and kept byte for byte, its leading byte
order mark included.

    sha256  42b71885e4dc885559fda5ad059fd81838cf4782cedb5fd08cc3431f7d067371  ISO-639-2_utf-8.txt
