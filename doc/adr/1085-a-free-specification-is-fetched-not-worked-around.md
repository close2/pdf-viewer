# 1085 — A free specification is fetched, not worked around

Session 1071. Status: **accepted**.

Four rounds — 1020, 1048, 1053 and 1062 — each left the same residue in §12.8's ledger rows and
each gave the same reason. §12.8.3.4.3 (b), (c) and (j) "state their rule only by reference to ETSI
EN 319 122-1", §12.8.3.4.4's profiles are "ETSI EN 319 122-2's rather than this standard's", and
each row ended: *that document is not one this tree holds*. The reason was true and the conclusion
was wrong. **ETSI publishes its European Norms at no charge**, at
`https://www.etsi.org/deliver/etsi_en/319100_319199/`, and EN 319 122-1 V1.3.1 (2023-06) and
EN 319 122-2 V1.1.1 (2016-04) were fetched in a minute each.

## 1. What holding them cost, and what it did not buy

They are prepared the way every other specification here is — `tools/spec-md.py`, which reads a PDF
with this tree's own `quorra-retrieve` — and they are tracked by nothing: `/doc/*.pdf` and `/doc/md`
were already ignored for ADR 0187's reason, so the rule that was written for the PDF Association's
documents covered ETSI's without a line of change.

**Their notice is stricter than ISO's, and it changes how the tree cites them.** It permits no
reproduction in any form without ETSI's written permission. So: clause numbers are cited and the
rules are paraphrased, in source and in the ledger alike, and nothing from either document appears
between quotation marks or after a `>`. ISO 32000-2's own sentences are quoted as always — the
conformance checker verifies those against `doc/md/` — and the two are kept visibly apart. A
paraphrase is prose without quotation marks, which is a rule this tree already had.

## 2. What the two texts together turned out to require

Each referenced clause states its rules in the same three parts, and every part is decidable from
the file: which attribute set the attribute belongs to, that its `SET OF AttributeValue` holds
exactly one component, and what the value has to be. Four attributes, twelve rules — clause 5.3's
signature timestamp, 5.2.8's content timestamp, 5.2.5's signer-location, 5.2.6.1's
`signer-attributes-v2` — plus clause 5.2.9.1's prohibition on the implied signature policy and ETSI
EN 319 122-2's Table 1 cardinalities and its requirements (a) and (c).

`cms::Attribute` is what made them checkable at all. The reader recorded attribute *identifiers* and
two attribute values; a rule about which set an attribute is in, or about how many components it
has, could not be written against that. It now records every attribute of a `SignerInfo` with its
set, its component count and its first value, which is one walk rather than a special case per rule.

**The census is what says whether any of it is about real files, and it is.** Over 90 763 documents
there are hundreds of `ETSI.CAdES.detached` signatures, and rules that did not exist before this
round fire on real ones — a `signing-certificate-v2` stating SHA-1 where Table 1's requirement (a)
reserves it for the other spelling, a `signature-policy-identifier` stating the alternative clause
5.2.9.1 forbids, a `commitment-type-indication` beside a `/Reason`. Two of those were confirmed by
`openssl asn1parse` on the named file before this ADR was written. The census also prints which
attributes the population *states*, because a rule that produced no departure did so either because
every file met it or because no file reaches it, and those are opposite facts (trap 8): the content
timestamp and the signer-location have no witness in the corpus at all, and their only calibration
is the fixture pair.

## 3. One copy of the five verifying constructions

§12.8.3.4.8 asks for a signature to be verified at the instant its timestamp attribute indicates,
and *indicated* was what held the row: an instant a token merely states is a number a stranger
wrote. Establishing it means running ADR 1071's four steps over a CMS object found **inside** a
signature rather than inside a document — which needs the whole of `Signature::authenticity` and
`Signature::trust_for`, and neither had a shape that could be reached without a `Signature` and a
document.

The alternative to extracting them was a second copy of the arm-per-construction match that pairs a
`signatureAlgorithm` with a key, and that pairing is exactly the thing this crate must not have two
of. So `signature::authenticity_of(cms, Detached)` and `signature::cms_trust(...)` are free
functions and `Detached` is the one thing that differs: a document's `/ByteRange`, or nothing at
all. RFC 3161 section 2.4.2 makes the second arm a refusal rather than a gap — a conforming token
encapsulates its `TSTInfo`, so a token reaching it signed nothing this reader can name.

`viewer_core::notes` then takes the instant, and §12.8.3.4.5 (b) puts its two sources in order: the
document timestamp chain first, because it is a statement about the document, and this attribute
second, because it is a statement about one signature. Both need two answers rather than one —
clause 5.3 puts the imprint over the `SignerInfo`'s `signature` field, so a token that establishes
an instant about somebody else's signature says nothing about this one.

**Nothing a person sees has changed about trust.** An instant is established only through a path to
an anchor a host supplied, and no document supplies its own, so every file in this tree still falls
back to the host's clock. The mechanism was what was owed; the anchor was never this row's to give
(ADR 1039, ADR 1076).

## 4. The rule this leaves behind

A row that says *this tree does not hold that document* is a claim with a price attached, and the
price is worth checking before the row is written a second time. Four rounds repeated it without
anybody spending the minute. The rule is the owner's: **a specification that is free to obtain is
obtained.** What a paid one costs is a question for `doc/questions/`; what a free one costs is a
`curl`.
