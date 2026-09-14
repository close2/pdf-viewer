# 1071 — A timestamp is established by what comes after it

Session 1057. Status: **accepted**.

ADR 1067 read §12.8.4's revocation material out of the file and applied it to every certificate on
a path. This one does the same for §12.8.5's document timestamp: what RFC 3161's token asserts,
what has to hold before this program will call that assertion an instant, and what a *sequence* of
them says that one of them cannot. It also records a defect the work found — a conforming timestamp
authority's certificate was refused by this tree's path validation, on a clause that requires the
extension that refused it.

## 1. A time is established, never read

`genTime` is a number a stranger wrote in a file, and four things hold before
`pdf_signature::timestamp::Time::Established` names an instant:

1. `/Contents` reads as a CMS `SignedData` whose `eContentType` is `id-ct-TSTInfo` and whose
   content is RFC 3161 section 2.4.2's `TSTInfo`;
2. the signer's `message-digest` signed attribute is the digest of that content. RFC 5652
   section 5.6: "For the signature to be valid, the message digest value calculated by the
   recipient MUST be the same as the value of the messageDigest attribute included in the
   signedAttributes of the SignedData signerInfo." **This step was missing for every encapsulating
   signature in this tree** and it is the one that binds a token's contents to the signature over
   it: without it, a `TSTInfo` can be swapped under a signature that still verifies;
3. that signature verifies under the key in the certificate the token carries;
4. a certification path from that certificate reaches an anchor somebody supplied, with §12.8.4's
   material applied to every certificate on it (ADR 1067).

Step 4 is `Trust::NoAnchorSupplied` in this tree, because no host here names an anchor (ADR 1039).
So every real document's timestamp is `Time::Unknown`, and that is the answer rather than a gap:
§12.8.5.2's authority is a *trusted* one, and nothing here decides whom to trust. What the token
*claims* is still reported, as a claim, with `genTime` shown in the token's own characters the way
`/M` is.

**`Judgement` gains no variant and no sentence gains the word *valid*.** That is ADR 1039's
decision unchanged, and this ADR is where a later round is told not to re-open it for a timestamp
in particular — a timestamp is the construct that most looks like it settles something.

## 2. A timestamp's own certificate is checked at the *next* timestamp's instant

§12.8.5.3 says why a document carries more than one token and what the later one is for: a token
"may expire due to the expiry of its certificate or the cryptographic strength of some of their
algorithms may not be resistant any longer to some new cryptographic attack", so a new one is
applied "before the expiry of the certificate", over the material proving the earlier authority's
certificates were current — "[t]he certificates, CRLs or OCSP responses used to demonstrate that
the certificates related to the certification path of the previous timestamp token was not revoked
for a reason which is either \"keyCompromise\" or \"cACompromise\" just before the time the new
timestamp token has been requested shall be included into the DSS dictionary. Then the new
timestamp token shall be placed into a new document timestamp dictionary which will protect the
whole structure."

So the sequence is load-bearing in one place, and it is RFC 5280 section 6.1.1's input (b):
**the instant at which one timestamp's authority is validated is the next timestamp's `genTime`,
where there is a next one and it is itself established.** Validating a token's certificate at its
own `genTime` would be asking the file to vouch for itself — a stolen key set to any instant inside
its own validity period would validate — and the outermost token is the only one for which nothing
better exists, which is why it, and only it, spends the caller's instant. `timestamp::AskedAt` is
what a reader sees that in, and the chain is therefore established from the outside in.

## 3. Coverage is decided at a revision boundary, or not at all

A later token covers an earlier one, and covers the store's material, if its `/ByteRange` names the
bytes those sit in. This program decides that by offset, and the decision is sound only because
§12.8.1 fixes where a signed range ends: "to the end of the \"%%EOF\" comment, possibly followed by
an optional EOL marker, terminating the incremental update that adds the digital signature
dictionary to the document". A range ending there covers a whole number of revisions, so an object
beginning below the boundary ends below it too.

A range that ends anywhere else is refused by name — `ChainRefusal::RangeDoesNotEndARevision` —
and nothing is claimed about what it covers. The alternative, guessing an object's extent by
parsing forward from its offset, would put a reader's arithmetic where a producer's `%%EOF` already
is.

**§12.8.5.2's "shall cover the entire PDF file" is asked of the outermost timestamp only**, and
that is §12.8.5.3's doing rather than a softening: that clause builds documents in which every
earlier token covers less than the finished file. What no token covers is what nothing protects,
and that is what the refusal names.

## 4. `extKeyUsage` is processed by whoever has a use, and this is what a purpose is

**A conforming timestamp authority's certificate could not validate here at all.** RFC 3161
section 2.3: "The corresponding certificate MUST contain only one instance of the extended key
usage field extension as defined in [RFC2459] Section 4.2.1.13 with KeyPurposeID having value:
id-kp-timeStamping. This extension MUST be critical." `pdf_signature::trust` refused every critical
extension it did not recognise — RFC 5280 section 4.2's `MUST` — and `extKeyUsage` was on the
unrecognised side with a comment explaining why: "this reader does not check a purpose against a
use". The clause that makes the extension mandatory is the clause that made the refusal certain,
and the first real timestamp token this tree validated is what found it.

The fix is the shape ADR 1039 already established for an anchor and an instant: **the purpose is an
input.** RFC 5280 section 4.2.1.12 makes `extKeyUsage` a statement about use — "If the extension is
present, then the certificate MUST only be used for one of the purposes indicated" — so only a
caller with a use can process it. `trust::Purpose` is that input, `validate_for` takes it, and
`Purpose::TimeStamping` names RFC 3161's `id-kp-timeStamping`.

Three narrowings, each deliberate:

- **`Purpose::Unstated` changes nothing.** A critical `extKeyUsage` stays
  `UnrecognisedCriticalExtension`, which is section 4.2's branch for a system that cannot process
  an extension, and it is what every existing caller passes. No answer this tree already gave moves.
- **The check is on the target certificate only.** Section 4.2.1.12: "In general, this extension
  will appear only in end entity certificates." A CA on the path that states a critical
  `extKeyUsage` restricts a use this program has no name for, and that stays a refusal.
- **An absent extension refuses too, under a stated purpose.** Section 4.2.1.12 permits it —
  "Certificate using applications MAY require that the extended key usage extension be present and
  that a particular purpose be indicated" — and RFC 3161 section 2.3 requires it, so a timestamp
  authority's certificate without one is not one.

`anyExtendedKeyUsage` satisfies any purpose, on section 4.2.1.12's own terms, and the reader that
decides it is `x509::indicates_purpose` — one function, which `revocation`'s RFC 6960
section 4.2.2.2 check now calls instead of keeping its own copy.

## 5. What this does not close

The anchor. Every link of every real chain is `Time::Unknown(AuthorityNotEstablished(
NoAnchorSupplied))`, which four documents out of the crawl demonstrate at one, two, three and five
stacked tokens. §12.8.3.4.8's clock — §12.8.4.2's "the UTC time included in that timestamp token
shall be used as the time reference to check the revocation status of the signer's certificate" —
is now *derivable* rather than merely owed: `AskedAt` is the mechanism and an established timestamp
is its source, so what is missing there is the same anchor and not a second thing.

How many documents carry a `/DocTimeStamp`, and how many carry more than one, is
`examples/signature_algorithm_census` over every document the tree holds; the numbers are its
output and not this file's.
