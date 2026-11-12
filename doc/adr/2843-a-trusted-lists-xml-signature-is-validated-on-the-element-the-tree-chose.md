# 2843 — A trusted list's XML signature is validated on the element the tree chose

Session 2012, a signatures round (`doc/todo/77` item 1). Status: implemented. Rows: §12.8.3.4.4,
§12.8.3.4.5. Context: ADRs 2644, 2710, 2842; W3C XML Signature Syntax and Processing 1.1 sections
3.2, 4.4.3 and 6; ETSI TS 119 612 V2.3.1 clause 5.7.1 and Annex B; ETSI EN 319 132-1 V1.3.1 clause
5.2.2.

## Context

TS 119 615 PRO-4.1.4-07 and PRO-4.2.4-08 validate a list's `ds:Signature` with its certificate as
the trust anchor, which leaves XML Signature's core validation: each reference's digest over the
node set it names, then the signature over the canonical `ds:SignedInfo`. The canonicaliser
(ADRs 2644, 2710) drew a subset around the first element of a name at a depth, which is enough for
a policy document and not for a signature: an attacker who wraps the signed element and places a
second of the same name where a reader looks first makes the digested element and the believed one
differ.

## Decision

1. **The element is chosen by reading the tree, and handed to the canonicaliser by its ordinal**
   (`canonical_xml::Target::Ordinal`), the index `crate::xml::Tree` gives the same start tag. The
   enveloped signature transform removes the element of that ordinal (`Subset::excluded`).
2. **Ambiguity is refused, never resolved**: a second `ds:Signature` among the root's children,
   or an identifier two elements carry, stops the validation by name.
3. **Annex B is held strictly**: `ds:SignedInfo` canonicalised by exclusive canonicalisation
   alone, a reference covering the list with exactly the enveloped transform and exclusive
   canonicalisation, one certificate in `ds:KeyInfo` (clause 5.7.1). A list signed otherwise is not
   authenticated, with the rule named.
4. **What is computed**: RSA PKCS#1 v1.5, RFC 6931's RSASSA-PSS without parameters and ECDSA with
   section 6.4.3's `r || s` value; SHA-1 and SHA-2 digests; the three canonicalisations and the
   enveloped transform; same-document references (the empty URI, a bare name). Anything else —
   XPath, XSLT, an external URI, an `xpointer` — is refused by name rather than approximated.
5. **The signed `SigningCertificateV2`** (EN 319 132-1 clause 5.2.2, which Annex B.1.1 says secures
   the operator's identity) is compared with the `ds:KeyInfo` certificate where a signed
   `SignedProperties` states one.

## Consequences

Comments are excluded from every reference's node set, as section 4.4.3.3 makes both URI forms
do, whatever the transform's `#WithComments`. A processing instruction outside the root is
rendered for the empty URI with Canonical XML's line break, so a list carrying one digests as
written.
