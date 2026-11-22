# 3357 — An associated file stating no media type states the one the clause names for an unknown type

Session 2164. Status: accepted. Context: ISO 19005-4 section 6.9; ISO 32000-2 §14.13.2, §7.11.4.1
(Table 44); Internet RFC 2046; ADR 1270 (the same sentence, found at the `attach` preserve); ADR 2536;
R15 of `doc/pdfa-blocking-issues.md`.

## The question

`issue17056` and `issue18030` (pdf.js) stop at part 4 under both shipped profiles. Printed at the
refusal: "ISO 19005-4 section 6.9 asks the embedded file stream for a Subtype that is a MIME media
type, and nothing in a file specification states one: a file name's extension is a convention rather
than a declaration". Each holds one PDF in the catalog's `/AF` with no `/Subtype`; both shipped
profiles answered the site `discard`, which this version did not carry out.

## What the specification says

The refusal's premise was a silence, and §14.13.2 is not silent:

> The embedded file stream dictionary shall include a valid MIME type value for the Subtype key.
> If the MIME type is not known, the value " application/octet-stream " shall be used.

ADR 1270 found the sentence for the bytes an `attach` preserve files and wrote the value there; the
refusal of the producer's own associated files was never revisited. This converter does not read an
attachment to learn its type, so the type is not known to it, and the clause names the value.
RFC 2046 makes `application/octet-stream` a body of arbitrary binary data: it asserts nothing about
the bytes, which is why it is no guess from an extension.

## Decision

- `embedded-files/associated-file-media-type` is answered `Mechanical` by
  `Rewrite::AssociatedFileMediaType`: each stream under the `/EF` of a specification the findings
  name that states no `/Subtype` is given `application/octet-stream` (`sites::media_types`).
- A stream stating a `/Subtype` that is no media type keeps the row refused, by name: the producer
  wrote something, and the clause's value is for a type nobody stated. So does a specification or
  stream written directly into another object, which this verb cannot rewrite.
- An operator's `supply` still outranks it. Where the table names no type for an attachment,
  `unlisted = "stop"` keeps the site refused (`remedies::UNLISTED_STOPS`) rather than letting the
  default answer it beside the supplied types, which would be one answer for the listed files and
  another for the rest.
- The shipped profiles' `discard` at the site is now an answer already carried out.

## What it costs

Nothing a reader sees, and no loss: an absent type and `application/octet-stream` both say "a reader
must find out what this is". Both documents convert at part 4 under both shipped profiles and pass
veraPDF 1.30.3; page 1 at 72 dpi is identical to the source's.
