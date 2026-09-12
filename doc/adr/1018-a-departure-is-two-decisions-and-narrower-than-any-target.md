# 1018 — A departure is two decisions, and narrower than any target

Status: accepted. Session 992.
Context: `crates/pdf-transform/src/archive/{config,mod,prepare,report}.rs`,
`crates/pdf-transform/tests/archive.rs`, `doc/profiles/factur-x.toml`, `doc/rfc/0007` section 4.7,
`doc/questions/A59`, `A60`; ISO 19005-2 section 6.8, ISO 32000-2 §7.11.3 (Table 43), §7.11.4.2,
§14.13. Companion to ADR 1012 (the configuration format).

## What a departure is, and why it is not a remedy

`doc/rfc/0007` section 4.7, asked for by the owner: *go against the spec and for instance accept xml
(and only xml) attachments when targeting pdf/a 2*. Every remedy in the catalogue produces a file
that conforms to the target; a departure produces one that does not, on purpose and by name. Putting
them in one table would make the configuration's most important distinction invisible, so a
departure is a `[depart."<requirement>"]` block, not a `[site."…"]` one.

`A60`: part 3 is not bought, so no target holds the XML-attachment case — part 3 and PDF/A-4f both
admit *any* embedded file, where an archive that mandates PDF/A-2's discipline wants XML and nothing
else. A departure can be **narrower than any target**, which is the whole argument for it.

## The narrowing predicate, and why it is asked of every file

The predicate is `media-type` (and optionally `relationship`), declarative — the reader compares a
key. The departure covers a document only where **every** embedded file the requirement is about —
ISO 19005-2 section 6.8's population, every file specification carrying an `/EF` — matches. That is
the difference between *accept XML attachments* and *accept XML and only XML*: one non-XML attachment
beside the invoice leaves the requirement refused, exactly as it would with no departure. The media
type is the embedded stream's own `/Subtype` (§7.11.4.2 a MIME media type); the relationship is
Table 43's `/AFRelationship`. The population is walked the same way the validator's predicate walks
it, so *only XML* means only XML by the same reckoning the requirement was failed on.

## Two decisions, taken twice (`A59`)

Departing from a clause and claiming conformance anyway are two decisions. By default a departed
conversion **omits the PDF/A identification**: the packet is still written (so
`metadata/catalog-metadata-stream` is met) but with no `pdfaid:*`, and any identification the source
stated is stripped by restating the schema with no properties. The output is a PDF that meets the
target in every respect but the departed one, and it does not claim to be PDF/A. `--claim-conformance`
is the second, separate switch: it keeps the claim, which a downstream validator fails either way —
the only difference is whether the file lied before it failed, which the operator owns explicitly.

## What it did to the verb, and the one rule it narrows

Stage three's net is **narrowed, never switched off** (section 4.7.3). The output is validated as
always; a departed conversion is written when the *only* requirements it still fails are the ones
departed from and — where the identification was omitted — the identification-schema requirements
that omission costs (`decision::IDENTIFICATION_CLAIM`). Anything else still failing is a real
failure and the file is refused, as it would be with no departure. A conversion with no departures
reduces to the plain conformance gate, so nothing about the existing verb changed.

Departures are computed once over the source (`departures_over`), before decisions, so a departed
requirement is reported as a departure rather than decided — it does not block `proceeds()`, and no
rewrite is wanted for it. The departure is recorded in the file's own `xmpMM:History` (section
4.7.3), and the invocation must carry `--depart-from-the-standard` (the CLI's, not the library's) so
the operator's intent to go against the standard sits at the call site, not only in a file that can
be inherited or copied between teams.

## The proof

`crates/pdf-transform/tests/archive.rs` builds a PDF/A-2b document from the clauses that is
conforming but for one XML embedded file, converts it three ways, and revalidates each independently:
without the departure it is refused; with it and no claim the file is written, states no `pdfaid`,
records the departure in its history, and fails only the embedded-file rule and the omitted
identification; with `--claim-conformance` it states `pdfaid:part 2` and fails only the embedded-file
rule. A fourth case gives the document a non-XML attachment and confirms the predicate does not cover
it, so the requirement stays refused. `doc/profiles/factur-x.toml` is the shipped example, and the
whole flow was run end to end through `quorra-transform` on the same fixture.

## What is not built

Departures from any requirement other than the two embedded-file ones — the catalogue (section 4.7.5)
argues every requirement is a candidate, each its own small argument, and that reading is the next
departure round's. A computed predicate (*depart only where removing the transfer function changes no
pixel*) is section 14 point 4's finding and needs the document examined; only the declarative
media-type form is built. The `relationship` predicate narrows by Table 43's names as strings; a
producer's registered `/AFRelationship` outside the eight is compared verbatim.
