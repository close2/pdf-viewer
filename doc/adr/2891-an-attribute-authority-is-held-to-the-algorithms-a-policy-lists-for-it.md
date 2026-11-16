# 2891 — An attribute authority is held to the algorithms a policy lists for it

Session 2037, a signatures round. Status: implemented. Rows: §12.8.3.4.4. Context: ADRs 1085, 2560,
2580, 2604, 2720, 2782, 2797; ETSI TR 102 272 V1.1.1 clause 6.10; ETSI TS 119 172-2 V1.1.1 clause
4.29; TS 119 172-3 V1.1.1 clause 4.29; RFC 5755 section 4.1. The ETSI texts are cited and
paraphrased (ADR 1085).

## Context

TR 102 272 clause 6.10's `aaCertAlgorithmConstraints` was read and named as not evaluated, because
nothing read attribute certificates. ADR 2797 built that reader. TS 119 172-2 clause 4.29's usage
URI `Usage#AaCert` was named among the usages not checked.

## What the member constrains

Clause 6.10 gives each of its five lists a one-word comment: the signer; the issuer of end-entity
certificates; the issuer of CA certificates; "Attribute Authority"; "TimeStamping Authority". On
its own, `aaCert…` could mean the authority's own certificate or the certificates the authority
issues. TS 119 172-2 clause 4.29.1 carries these lists forward and settles it: its usages include
algorithms "used for generating attribute certificates". So the member constrains what the
attribute authority signs: each attribute certificate's `signatureAlgorithm`, and the length of the
authority's key that verified it, against each entry's `minKeyLength`.

The same clause lists algorithms "used for generating time-stamp tokens", which reads clause 6.10's
`tsaCert…` as the authority's tokens rather than its certificate. ADR 2720 reads it as the
authority's certificate. This round does not reopen that reading. It is reported for a later round
to take back to the two clauses.

## Decision

1. **One finding per attribute certificate the signature carries**, in the order
   `signer_attributes::Carried::certified` holds them. It is broken where the list does not name the
   algorithm or the key is shorter than the minimum. It is not evaluated where the certificate does
   not read, or where a minimum is stated and no carried certificate of the authority's name both
   verifies the signature and has a key this reader measures. Otherwise it is met. A signature that
   carries none meets the list: there is nothing for it to constrain.
2. **The certificate need not be valid to be constrained.** The member is about what an authority
   generated, not about whether the policy believes it. `certified::made` reads every carried
   certificate and finds the issuer's key as RFC 5755 section 5's condition 2 does.
3. **TS 119 172 holds the same certificates to entries naming `Usage#AaCert`.** An entry that names
   no usage allows every usage (clause 4.29.1), so an XML list of such entries constrains them too.
   The ASN.1 syntax requires at least one usage. A signature carrying none is said only where an
   entry names the usage outright. Otherwise every usage-less list would report on every signature.

## Consequences

`policy_rules::Rules::attribute_algorithms` holds the list.
`Constraint::AttributeAlgorithm(AttributeIssuance)` is its finding, and
`ts_119_172_3::Usage::AttributeCertificate` is the usage. `AA_CERT_USAGE` is no longer among the
usages named as not evaluated. The fixture authority signs with `ecdsa-with-SHA256` under a P-256
key, and its expected values are 256 bits against minimums of 256 and 384.
