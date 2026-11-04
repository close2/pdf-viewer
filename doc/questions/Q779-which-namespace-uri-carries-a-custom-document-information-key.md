# Q779 — Which namespace URI carries a custom document information key?

Source: round 1929 (single round), ADR 2680, the R14 paragraph of `doc/pdfa-blocking-issues.md`.

## What the round found

At part 4, ISO 19005-4 section 6.1.3 removes every `/Info` entry except `/ModDate`. ISO 32000-2
§14.3.3 sends any other document-level metadata to the metadata stream. Table 349 names an XMP
property for each of its own keys, but none for a key it does not define (`/WPS-ARTICLEDOI`,
`/Company`, `/PTEX.Fullbanner`, …). Unlike ISO 19005-2, part 4 does not require a described
extension schema, so the packet may hold such a value under any namespace. 140 documents per
stating profile were refused for this at part 4.

ADR 2680 carries the value unchanged, as a simple property whose local name spells the key. The
namespace belongs to this project, and it states one fact: the value came from the source's
dictionary under that key. It says nothing about what the key means, which is ADR 1269's objection
to inventing one. The URI is written into every archived file that carries a key, so it is a
permanent, public choice. That makes it the owner's choice.

## The question

Which URI should the namespace be?

- **a.** `https://github.com/close2/pdf-viewer/ns/document-information/1.0/`, prefix `docinfo`. It
  sits under the repository the owner controls. ISO 19005-4 section 6.7.2.2 says a namespace URI
  identifies and need not resolve. **This is what the tree writes now.**
- **b.** A URI under a domain the project registers for the application (`quorra`). It is stabler
  than a hosting provider's path, but no such domain exists yet.
- **c.** Adobe's `http://ns.adobe.com/pdfx/1.3/`, which Acrobat uses for this purpose. ADR 1269
  section 5 declined it because no standard defines it, and presenting a vendor convention as
  derived is what principle 5 forbids.

## Recommendation

**a.** It is stable for as long as the repository is, and it is visibly this project's choice. The
`1.0/` segment leaves room for a later definition without reusing the URI.

## What the tree does meanwhile

It writes **a** under `state-every-loss` and `no-data-loss` at part 4. Changing the URI later is a
one-constant edit (`CARRIED_INFORMATION_NAMESPACE` in
`crates/pdf-transform/src/archive/prepare/metadata.rs`). Files already archived keep the old URI.
