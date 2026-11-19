# 3221 — An encapsulated digest counts only where the signature binds it, and a value with no digest of the document says so

Session 2130. Status: accepted. Context: review 2128 findings B01, B02, T01, D01, D02, D04; ISO 32000-2
§12.8.1 and §12.8.3.3.1; RFC 5652 sections 5.4, 5.6 and 11.2; ADR 0229 (the three questions).

## Decision

1. **Both links of the encapsulating formats are compared in `Signature::integrity`.** For
   `adbe.pkcs7.sha1` the document's digest is the encapsulated content, and for a document
   timestamp it is the `TSTInfo`'s `messageImprint`. Where the signer states signed attributes,
   RFC 5652 section 5.4 puts the signature over the attributes, and section 11.2's `message-digest`
   is the digest of the encapsulated content. `content_binding` compares that attribute with the
   content under the signer's `digestAlgorithm`. A mismatch is `Integrity::Changed` with the first
   link's algorithm. No attribute at all is `Integrity::NoDigestOfTheDocument`. An unknown algorithm
   is `Integrity::UnknownDigest`.
2. **A new `Integrity::NoDigestOfTheDocument`** covers a value whose signature is not over the
   document's bytes and that records no digest of them. Two shapes reach it: a signer with no signed
   attributes that signs an encapsulated content under any other `/SubFilter` (§12.8.3.3.1 for
   `adbe.pkcs7.detached`: "No data shall be encapsulated in the CMS SignedData field."), and signed
   attributes without a `message-digest`. `Integrity::UnderTheSignersKey` is kept for the two shapes
   whose signature is over the document's own bytes (§12.8.3.2's PKCS #1 value, and a detached signer
   with no signed attributes and no content).
3. **`Verdict::reached` checks the premise it relied on.** It accepts `UnderTheSignersKey` with a
   `Verified` only where `Signed::binds_the_document()` holds. Any other pairing is
   `Reservation::ChangeNotDecided`.

## Why

- **The comparison belongs to question 1, not question 2.** `integrity` already makes RFC 5652
  section 5.6's comparison for a detached value, where the content is the document. The
  encapsulating formats add one link to the same chain: document, content, attribute, signature.
  Putting the second link in `authenticity_of` would have reported a signature that "does not
  verify" when its arithmetic does verify. It would also have left `integrity` reading
  `Unchanged` for a digest nobody signed, which is what audit 2128 found.
- **`Changed` is the honest answer to a mismatch.** The attribute is the digest the signer
  committed to. If the content no longer hashes to it, the digest the value carries was written
  after signing, and the document's bytes are not under the signer's commitment. `timestamp`'s
  own check of a token's binding is the same comparison (`ContentNotBoundToTheSignature`).
- **A named variant, not a reused one.** A value without a digest of the document is readable, so
  `Unreadable(MalformedSignedData)` would misstate it. The value also states nothing about whether
  the document changed, so `Changed` would misstate it too.
- **The verdict's check is the premise of the sentence it already carried.** The comment said that
  question 2 answers question 1 where nothing records a digest. That holds only for a signature over
  the document's bytes. Writing it as a guard makes it true by construction rather than by
  `integrity`'s current behaviour.

## Cost

- One public variant: `pdf-transform`'s archive report and `viewer-core`'s notes each gain a
  sentence, and no corpus signature reaches it (`signatures.rs` holds `unreadable` empty).
- An `adbe.pkcs7.sha1` or timestamp producer that wrote a wrong `message-digest` and never edited
  anything now reads `Changed`. RFC 5652 section 5.6 makes that signature invalid in any case
  ("For the signature to be valid, the message digest value calculated by the recipient MUST be the
  same as the value of the messageDigest attribute").
