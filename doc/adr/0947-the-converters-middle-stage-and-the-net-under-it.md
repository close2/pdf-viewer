# 0947 — The converter's middle stage, and the net under it

Session 947. Status: **accepted**. The first slice of `quorra-transform archive`
(`doc/questions/A22`), built on the validator `pdf-archive` already is.

## What was decided

Five decisions, and only the first was obvious from `doc/pdf-a-conversion-limits.md`.

### 1. Three stages, and the middle one is a table

`crates/pdf-transform/src/archive.rs` runs **validate → decide → apply**, and the decision stage is
the product. `doc/pdf-a-conversion-limits.md` sections 2, 3 and 4 are *three different answers to a
failed requirement* — a refusal, a loss somebody has to authorise, and a default that is right —
and a converter that ran them together could not say which one a given file got. So `Decision` has
four variants (`Mechanical`, `Authorised`, `Unauthorised`, `Refused`), `Because` says which of
three kinds of *no* a refusal is, and the mapping from a requirement identifier to an answer is a
`const` table beside the requirements it answers.

The consequence worth naming: **the validator is the reading, and nothing in the converter re-reads
ISO 19005.** A requirement's clause, its sentence and its places all come from `pdf_archive`, so a
converter's report cites what a validator's verdict cites. That is why `pdf-transform` now depends
on `pdf-archive`, and its `Cargo.toml` says so.

### 2. Nothing is changed that no failed requirement asked for

The rule that makes "a document already conforming is not rewritten" true by construction rather
than by a special case. It settles the one row of
section 4.7's table this slice does not take: **an
`Identity` `Crypt` filter is left alone.** The requirement permits it — ISO 19005-2 section 6.1.7.2
and ISO 19005-4 section 6.1.6.2 forbid the filter *unless* its decode parameters name `Identity` —
so removing one would be a change with no requirement behind it. What that row is actually about
is section 3.5's encryption removal, which this slice does not do; a non-`Identity` `Crypt`
filter is therefore refused with the rest of encryption. The filter chain a stream's `LZWDecode`
rewrite re-encodes drops its `Crypt` stages anyway, because a re-encoded chain states one filter.

### 3. The whole-file rewrite is a remedy, and it has a limit nobody had written down

Six of section 4.7's rows say "the serializer emits this shape anyway": hexadecimal digit
counts, the `stream` and `endobj` keyword line endings, `/Length`, the indirect object syntax, the
cross-reference keyword's line endings, and what follows the last `%%EOF`. They are answered by
`Rewrite::WholeFileRewritten` rather than by a per-object rewrite, because there is no place to
count when the rewrite is the file.

**But that remedy stops at a content stream, and the hexadecimal string rule reaches inside one.**
ISO 19005 states the rule of the file's syntax, and a content stream is syntax; `pdf-archive`
records an object's own syntax at `Where::object` and a content stream's at `Where::page`, so the
converter asks the report which kind it is. A failure on a page is refused under ADR 0816's fence,
because correcting it would edit the producer's marks. The corpus witness is veraPDF's
`6-1-5-t01-fail-a`, whose odd digit count is in the page's `/Contents`: the first version of this
verb "fixed" it and wrote a file that still failed.

### 4. A file leaves the verb only if it conforms — and the output is validated to find out

Stage 3 assembles the output in memory, opens it, and holds it to the same target again. Two things
come out of that, and the second is why it earns its cost:

- **The verdict in the report is a measurement rather than a promise.** Section 7 of
  `doc/pdf-a-conversion-limits.md` says a clean report means "every requirement this program checks
  was met"; this is the program checking.
- **It catches what the decision table cannot see.** ISO 19005-2 section 6.2.9.3 forbids a
  PostScript `XObject`, and dropping one is lossless — ISO 32000-1's 8.8.2 says a PostScript
  fragment has no effect when the document is viewed on screen or printed to a non-PostScript
  device. But if a content stream *invokes* it, the removal leaves a named resource undefined,
  which ISO 19005-2 section 6.2.2 forbids, and the `Do` that names it may not be edited. The
  decision table cannot know that; the net does, and it names the exact requirement the conversion
  would have broken. veraPDF's `6-2-9-3-t01-fail-a` is the witness.

The cost is stated where it is paid: this verb's peak memory is the whole output file, where every
other writer in the crate hands the sink an `Arc` and keeps nothing.

### 5. Raising a version is allowed and lowering one is not

`version_for` decides what the header states. A PDF 1.x source converted to a part 4 target gets
`%PDF-2.0`: every construct ISO 32000-1 defines, ISO 32000-2 still defines, and part 4's own
prohibition on deprecated features is a requirement the validator reports as *not checked* either
way, so the raise asserts nothing the report conceals. A PDF 2.x source converted to a part 2
target is **refused**: section 6 of `doc/pdf-a-conversion-limits.md` states the cost — every PDF
2.0-only construct is translated or refused one at a time — and this converter translates none of
them, so a `%PDF-1.7` header over 2.0 constructs would be a file whose own header disowned its
contents.

## What this slice does not do, and why that is not a stub

The output intent and colour, fonts, metadata and the identification schema, the structure tree,
encryption, attachments, the `.notdef` and DeviceN constructions: each is refused **by name**, with
the requirement's own identifier and clause, and no file is written. `doc/adr/0927`'s four
permissions are not exercised by this slice at all.

The report they are conditional on is built now regardless, because **a report designed after the
writing has begun is a log.** `Conversion` carries, per document: the target, what conformed
already, every failed requirement with the decision taken about it and how many places the rewrite
touched, every requirement the validator did not check with its reason word for word
(`doc/questions/A20`: the reason may never be softened), and the output's own verdict. It reaches a
caller through `pdf_transform::Report::archive` whether or not a file was written — the interesting
case is often the one where none was — and `--report=json` carries all of it.

## What it cost elsewhere

`pdf_syntax::serialize::flate_encode` is new and public. The `LZWDecode` rewrite must re-encode
whether or not the result is smaller, which is the one thing `Streams::Recompress` will not do, and
a second zlib configuration for one file format would be a second set of settings to keep in step.

`optimize`'s `catalog_of` and `refuse_a_document_only_recovery_reads` are now `pub(crate)` and
shared: a converted file has to be a file a producer could have written, so the same §C.4 refusal
applies.

## What constrains the slices after this one

- **The rewriter walks the closure `/Root` and `/Info` reach**, so an object nothing reaches is not
  carried. That is the one way the output holds less than the input without a decision saying so,
  and it is tested (`an_object_the_catalog_cannot_reach_is_not_carried_by_a_rewrite_and_is_by_a_copy` — renamed when ADR 1006 made a conforming source the identity conversion).
- **An object the conversion changes is `replace`d, and its replacement is built in the source's
  numbering** and renumbered when it is placed. That is what lets the walk follow the references
  the *rewritten* object holds rather than the ones the source held, so a key a conversion removes
  cannot leave an object in the file that nothing refers to. Any later rewrite must be built the
  same way.
- **The test fixture is a hand-built conforming document**, not a corpus file:
  `crates/pdf-transform/tests/archive.rs` writes five objects, an XMP identification packet per
  part, and one empty page, and `pdf_archive::check` finds it conforming. A slice that adds the
  output intent will have to extend it rather than reach for a corpus, because the corpus is not
  part of a checkout.
- **`Authorisations` has one field per `Loss` rather than a set**, so adding a loss to section 3's
  list is a compile error everywhere it has to be answered rather than a word nobody matched.
