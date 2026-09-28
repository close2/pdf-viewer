# Q168 — Are the corpus's four decryptable public-key documents A66's trigger?

Source: round 1265, re-reading the `reported` rows (§7.6.5's note).
Status: **open** — answered when `A168-are-the-corpus-s-decryptable-public-key-documents-a66-s-trigger.md`
exists beside this file.

## Why it needs the owner

`A66` deferred §7.6.5 until "a real trigger": a document whose recipient list could match a
certificate the user holds, or a host asking to supply a private key. Q66 and ADR 1134 also said that
none of the five documents naming the handler could be decrypted by a finished implementation.
That premise is false for four of them. `doc/corpora/pdfbox`'s `AESkeylength128.pdf`,
`AESkeylength256.pdf`, `AES128ExposedMeta.pdf` and `AES256ExposedMeta.pdf` are in the same directory
as `PDFBOX-4421-keystore.pfx` (`CN=testnutzer`, serial `5F609C62`) and `PDFBOX-5249.p12` (`CN=test`,
serial `60FFD550`). Those are the issuer and serial each document's `EnvelopedData` names, and
PDFBox's own test states each file's plain text. Whether a published test key counts as "a
certificate the user holds" is a reading of the owner's own words.

## What the tree does meanwhile

§7.6.5 stays `reported` and refused by name. §7.6.5's note records the witness, so the build, when
it comes, is tested end to end against four third-party files.

## Recommendation

**Not a trigger.** No reader of this program is the recipient of a published test key, so the
robustness gain for a real user is still nil, and that was A66's reason. The four files are a
witness that a later build can be tested against. They should not start the build on their own.
