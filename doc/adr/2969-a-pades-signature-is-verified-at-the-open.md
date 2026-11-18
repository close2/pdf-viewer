# 2969 — A PAdES signature is verified at the document's open, once page one is shown

Session 2073. Status: **implemented.** Context: review 2038 (finding B17), ADR 1044 (whose deferral this narrows),
`CLAUDE.md` principle 2. Clause: ISO 32000-2 §12.8.3.4.5.

§12.8.3.4.5: "when the user opens a signed document or requests verification of these signatures present in the
PDF, a PDF processor, as the verifier, shall perform the following steps". ADR 1044 deferred the document's report,
signatures included, to `Command::Report`, because verifying digests the signed byte ranges — on a signed document,
the whole file — and none of it draws a page. That met the second trigger and not the first.

The two requirements are both kept by putting the verification at the first `Command::Presented` of a document
holding an `ETSI.CAdES.detached` signature: the open is not over until the host has shown it, and page one has
already been drawn, so time-to-first-page pays nothing. The report arrives as `Command::Report`'s does, and
`Open::about`'s `OnceLock` makes a later `Command::Report` free. Finding the signature reads the signature fields
and digests nothing (`Open::holds_pades_signature`). An unsigned document, or one signed under another
`/SubFilter`, is unchanged.
