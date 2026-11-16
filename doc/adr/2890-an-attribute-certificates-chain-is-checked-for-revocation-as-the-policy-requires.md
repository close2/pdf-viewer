# 2890 — An attribute certificate's chain is checked for revocation as the policy requires

Session 2037, a signatures round. Status: implemented. Rows: §12.8.3.4.4. Context: ADRs 1067, 1085,
2720, 2775, 2782, 2785, 2797; ETSI TR 102 272 V1.1.1 clauses 6.6.2 and 6.9; ETSI TS 119 172-2 V1.1.1
clauses 4.23 and 4.26; TS 119 172-3 V1.1.1 clause 4.26; RFC 5755 sections 4.3.6, 4.5, 5 and 6; RFC
5280 sections 6.1.3 and 6.3.3; RFC 6960 section 4.2.2.2. The ETSI texts are cited and paraphrased
(ADR 1085).

## Context

ADR 2797 judged a certified signer attribute by RFC 5755 section 5 and left the certificate's own
revocation: a policy stating `attrRevReq` (TR 102 272 clause 6.9, TS 119 172-3 clause 4.26) or
`AttributeRevocationTrust` (TS 119 172-2 clause 4.26) left every such certificate undecided. ADR 2782
named it as work, needing only the authority's revocation material read for it. The premise held:
nothing here waits on the owner, and nothing else in the tree checked it.

## Decision

1. **The requirement is read whole in all three syntaxes**, as the signing certificate's is: TR 102
   272's `CertRevReq`, and TS 119 172's `CertificateRevTrust` with its freshness and retention.
   `signer_attributes::AttributeTrust::revocation` and `SetConstraints::revocation` carry a
   `revocation_checks::Requirement` where they carried a flag.
2. **The chain is the attribute certificate, its authority's certificate, and the CA certificates
   of the authority's path below its trust point.** RFC 5755 section 5 condition 2 verifies the
   authority's whole path by RFC 5280, whose section 6.1.3 (a)(3) asks each certificate's status,
   and the requirement is the policy's minimum for the material that answers. Clause 6.6.2 splits
   it between end certificates and CA certificates. Section 4.5 forbids the authority's certificate
   to be a CA's, so it is an end certificate, and the end-entity check covers it beside the
   attribute certificate; the CA check covers the rest. The trust point is not checked, as for the
   signer's path (ADR 2720).
3. **Material names an attribute certificate as it names any other**: a CRL by issuer and serial,
   an OCSP `CertID` by hashes of the issuer's name and key and the serial. RFC 5755 section 6 lets a
   verifier take the status from any source. `revocation::Subject` now holds the issuer's name and
   the serial number rather than a public-key certificate, so one check serves both kinds. The
   authority's CRL is held to RFC 5280 section 6.3.3 (f): section 4.5 asks the authority's key usage
   only to admit signatures, so a list under a key without `cRLSign` answers nothing.
4. **Revoked is invalid; unanswered is undecided.** RFC 5755 section 5's additional check 1 lets a
   verifier's configuration reject a certificate, and the policy is that configuration. So a check
   that finds a certificate of the chain revoked at or before the instant makes it invalid, as a
   revoked certificate on the signer's path breaks that condition. A check the file's material does
   not answer leaves it undecided: EN 319 102-1's indeterminate, and absent evidence is not
   revocation (ADR 1067). A path that did not validate leaves the authority's key untrusted, so
   neither kind is answered then.
5. **`noRevAvail` does not lift the requirement.** RFC 5755 section 4.3.6 is the issuer's word that
   no revocation information will exist. TS 119 172-2 clause 4.23.2.1 gives the policy's
   requirement precedence over what a certificate says, so a mandated check stands. Unanswered, it
   is undecided, and the sentence names the extension.
6. **The material and its time are the signer's.** The checks read the facts' material, which is
   §12.8.4's store and the signer's archival attribute, with the same freshness and caution period
   as the signing certificate's (ADR 2775). The retention of TS 119 172-2 clause 4.23.3.1 item 2 is
   defined for the signing certificate alone, so on a set's requirement it is named and not
   evaluated (ADR 2785).

## Consequences

`revocation_checks::attribute_checks` makes the two checks, with `Kind::AttributeEndEntities` and
`Kind::AttributeAuthorities`. `certified::Invalid::Revoked` and `certified::Undecided::Revocation`
carry the check that decided. The fixtures are a CA added under the attribute root, an authority
under it, two attribute certificates (one stating `noRevAvail`), and CRLs and OCSP responses from
each issuer. All were minted with `openssl` from keys this tree made, and each expected value is
read off the clause its test names. Signed assertions and value constraints are still left
(ADR 2782).
