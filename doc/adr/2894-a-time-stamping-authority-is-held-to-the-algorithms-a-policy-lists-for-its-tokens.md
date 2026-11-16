# 2894 — A time-stamping authority is held to the algorithms a policy lists for its tokens

Session 2039, a signatures round. Status: implemented; supersedes the sentence of ADR 2720 that
judged `tsaCertAlgorithmConstraints` on the authority's certificate. Rows: §12.8.3.4.4. Context:
ADRs 1085, 2580, 2720, 2782, 2891; ETSI TR 102 272 V1.1.1 clause 6.10; ETSI TS 119 172-2 V1.1.1
clauses 4.29.1 and 4.29.2; TS 119 172-3 V1.1.1 clause 4.29; ETSI EN 319 122-1 clause 5.3; RFC 3161.
The ETSI texts are cited and paraphrased (ADR 1085).

## Context

ADR 2720 judged clause 6.10's `tsaCertAlgorithmConstraints` on the time-stamping authority's
certificate along the authority's path: the algorithm that certificate was issued under, with the
key of its issuer. ADR 2891 read the neighbouring `aaCertAlgorithmConstraints` the other way, as
what the attribute authority signs, from TS 119 172-2 clause 4.29.1's usages, and left this one for
a later round. TS 119 172-2's `Usage#Tstk` was named among the usages not evaluated.

## The two texts

Clause 6.10 comments its five lists: the signer; the issuer of end-entity certificates; the
issuer of CA certificates; Attribute Authority; TimeStamping Authority. The two certificate lists
name an *issuer* outright, and this tree judges each on what that issuer signed (the certificates
along the path, with the issuing key). The last two name the authority itself, as the first names
the signer; the signer's list is judged on what the signer signed. Read on its own, the TSA list
is about what the authority signs: its time-stamp tokens. The authority's certificate is an
end-entity certificate whose issuer is a CA, and what that CA signed is the business of the
issuer lists, not of a list named for the authority.

TS 119 172-2 clause 4.29.1 carries clause 6.10's purposes forward as usages, and one is algorithms
"used for generating time-stamp tokens" (`Usage#Tstk` in clause 4.29.2), with no usage for the
certificate of a time-stamping authority except as a certificate in a certification path. TS
119 172-3 clause 4.29 takes the same URIs. Both texts agree, so ADR 2720's reading was wrong.

## Decision

1. **TR 102 272: the token of the signature's `signature-time-stamp` attribute is judged.** Its
   `digestAlgorithm` and its `signatureAlgorithm` must each be listed, as
   `signerAlgorithmConstraints` holds the signature's own two; a `minKeyLength` applies to the
   signature algorithm and is held against the authority's key. `Constraint::TimestampAlgorithm`
   is the finding.
2. **The key is measured only where it is shown to have made the token**: steps 1 to 3 of
   `timestamp::signature_timestamp_established` pass and the signer's certificate states a key
   this program measures. A token that does not verify was still made with the algorithms it
   states, so they are judged; a minimum over it is not evaluated. ADR 2891 holds an attribute
   authority the same way.
3. **No token, nothing constrained**: met. An attribute whose value is no `SignedData`: not
   evaluated.
4. **TS 119 172: the usage `Usage#Tstk`** (`ts_119_172_3::TSTK_USAGE`) judges the same two
   algorithms with `Usage::TimestampToken`. A signature with no token is said only where an entry
   names the usage outright (ADR 2891's rule). The usage is no longer named as not evaluated.
5. The token judged is the signature-time-stamp, the one clause 6.8's condition is about. A
   content time stamp and §12.8.5's document time stamps are not read by `cms`, so they are not
   judged here.

## Consequences

`Rules::timestamp_algorithms` replaces `authority_certificate_algorithms`. The authority's path
is still searched for clause 6.8's trust condition (ADR 2720); only the algorithm list moved off
it. The tests use the minted token (`id-sha256`, `rsaEncryption`, RSA 2048). A list of
`sha256WithRSAEncryption`, the algorithm the authority's certificate was issued under, now fails
the token's signature algorithm, and that case is the test that tells the two readings apart.
